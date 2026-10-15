"""Loop one, loop off with hold at the end, a held item, in and out points.

    GMX_URL=... GMX_TOKEN=... GMX_SOURCE=lobby GMX_GMX=path/to/gmx python dev/live_modes.py MEDIA_DIR

MEDIA_DIR holds g1_1080p30.mkv (4 s), g5_tone.wav (2 s) and h_still.png,
the files dev/make-test-media.sh writes. Changes the source's list.
"""
import json, os, subprocess, sys, time
sys.path.insert(0, os.path.dirname(__file__))
from live_controls import tool, st, check, SOURCE, TOKEN, BASE
import live_controls as controls
G = os.environ.get("GMX_GMX", "gmx")
M = os.path.abspath(sys.argv[1]).replace("\\", "/")
env = dict(os.environ, GODWINMIX_URL=BASE[: -len("/api/v1")], GODWINMIX_TOKEN=TOKEN)
def set_params(params):
    out = subprocess.run([G, "ctl", "rpc", "source.set", json.dumps({"id": SOURCE, "params": params})], env=env, capture_output=True, text=True)
    if out.returncode:
        print(out.stderr[:300])
g = lambda n: {"id": n, "path": f"{M}/{n}"}
# In and out: 2 s of a 4 s file, from 1 s in.
set_params({"items": [dict(g("g1_1080p30.mkv"), in_ms=1000, out_ms=3000), g("g5_tone.wav")], "loop": "one", "shuffle": False})
tool("play_item", id="g1_1080p30.mkv"); time.sleep(1.2); s = st()
check("an in point starts the item there and the length is out minus in", s["current"]["length_ms"] == 2000, s["current"])
c0 = s["changes"]; time.sleep(2.6); s = st()
check("loop one plays the same item again", s["current"]["id"] == "g1_1080p30.mkv" and s["changes"] > c0, (s["current"], c0, s["changes"]))
# Loop off, hold at the end.
set_params({"items": [g("g5_tone.wav"), dict(g("h_still.png"), still_ms=1000)], "loop": "off", "at_end": "hold"})
tool("play_item", id="g5_tone.wav"); time.sleep(4.5); s = st()
check("a list with loop off ends", s["mode"] == "ended" and s["current"] is None, (s["mode"], s["current"]))
tool("play"); time.sleep(1.2); s = st()
check("play after the end starts again from the top", s["mode"] == "playing" and s["current"]["id"] == "g5_tone.wav", (s["mode"], s["current"]))
# A held item.
set_params({"items": [dict(g("h_still.png"), still_ms=1000, hold=True), g("g5_tone.wav")], "loop": "all"})
tool("play_item", id="h_still.png"); time.sleep(2.5); s = st()
check("an item marked hold stops on its last frame", s["mode"] == "holding" and s["current"]["id"] == "h_still.png", (s["mode"], s["current"]))
tool("play"); time.sleep(1.2); s = st()
check("play goes on to the next item", s["mode"] == "playing" and s["current"]["id"] == "g5_tone.wav", (s["mode"], s["current"]))
print("ALL OK" if controls.ok else "SOME FAILED")
