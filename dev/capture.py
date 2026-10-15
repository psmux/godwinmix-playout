#!/usr/bin/env python3
"""Run the engine with no mixer, play a list, and keep what it sends.

    python dev/capture.py --seconds 30 --out run.mkv a.mp4 b.mkv c.png

Speaks the plugin protocol the way the core does (initialize, start, stop,
shutdown), writes the engine's stdout, which is the Matroska stream the core
would read, to --out, and prints the engine's CPU time and peak memory when
it is done. Standard library only. The capture is raw video, so re-encode it
before looking at it: dev/measure.sh does that and runs the gaps example.
"""
import argparse, json, os, subprocess, sys, threading, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def engine():
    name = "playout-engine.exe" if os.name == "nt" else "playout-engine"
    for d in ("bin", "target/release", "../target/release"):
        p = os.path.join(ROOT, d, name)
        if os.path.exists(p):
            return p
    sys.exit("build the engine first: cargo build --release")


def cpu_seconds(proc):
    """User plus kernel time of a running child, in seconds."""
    if os.name == "nt":
        import ctypes
        from ctypes import wintypes
        k = ctypes.windll.kernel32
        h = k.OpenProcess(0x1000, False, proc.pid)
        times = [wintypes.FILETIME() for _ in range(4)]
        k.GetProcessTimes(h, *[ctypes.byref(t) for t in times])
        k.CloseHandle(h)
        as_s = lambda t: ((t.dwHighDateTime << 32) | t.dwLowDateTime) / 1e7
        return as_s(times[2]) + as_s(times[3])
    with open("/proc/%d/stat" % proc.pid) as f:
        fields = f.read().rsplit(")", 1)[1].split()
    return (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--seconds", type=float, default=20)
    ap.add_argument("--out", default="capture.mkv")
    ap.add_argument("--size", default="1280x720x30")
    ap.add_argument("--loop", default="all")
    a = ap.parse_args()
    w, h, fps = (int(x) for x in a.size.split("x"))
    items = [{"id": "i%d" % n, "path": os.path.abspath(f)} for n, f in enumerate(a.files)]
    env = dict(os.environ, GMX_PLUGIN="playout", GMX_PROVIDE="playlist", GMX_INSTANCE="capture",
               GMX_API_LEVEL="1", GMX_PLUGIN_ROOT=ROOT)
    out = open(a.out, "wb")
    p = subprocess.Popen([engine()], cwd=ROOT, env=env, stdin=subprocess.PIPE, stdout=out, stderr=subprocess.PIPE)
    lines = []
    threading.Thread(target=lambda: [lines.append(l.decode("utf-8", "replace").strip()) for l in p.stderr], daemon=True).start()

    def send(obj):
        p.stdin.write((json.dumps(obj) + "\n").encode())
        p.stdin.flush()

    while not any('"initialize"' in l for l in lines):
        time.sleep(0.05)
    canvas = {"width": w, "height": h, "fps": fps}
    send({"jsonrpc": "2.0", "id": 0, "result": {"core": "capture", "version": "0", "api_level": 1, "api_compatible": 1,
          "canvas": canvas, "transport": "container", "media": "", "instance": "capture", "provide": "playlist",
          "params": {"items": items, "loop": a.loop}}})
    send({"jsonrpc": "2.0", "id": 1, "method": "start", "params": {"canvas": canvas, "transport": "container", "media": ""}})
    started = time.time()
    # The first five seconds include opening the first file; measure after.
    time.sleep(min(5, a.seconds / 3))
    t0, c0 = time.time(), cpu_seconds(p)
    while time.time() - started < a.seconds:
        time.sleep(0.5)
    t1, c1 = time.time(), cpu_seconds(p)
    print("cpu: %.1f%% of one core over %.1f s" % ((c1 - c0) / (t1 - t0) * 100, t1 - t0))
    send({"jsonrpc": "2.0", "id": 2, "method": "tool.call", "params": {"name": "state", "arguments": {}}})
    time.sleep(0.3)
    send({"jsonrpc": "2.0", "id": 3, "method": "stop", "params": {}})
    send({"jsonrpc": "2.0", "id": 4, "method": "shutdown", "params": {"reason": "capture done"}})
    p.stdin.close()
    p.wait(timeout=10)
    out.close()
    for l in lines:
        if '"log"' in l or '"id":2' in l:
            print(l)
    print("wrote", a.out, os.path.getsize(a.out), "bytes")


if __name__ == "__main__":
    main()
