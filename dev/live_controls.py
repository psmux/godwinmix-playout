"""Drive every playout control on a running core and check what state says.

    GMX_URL=http://127.0.0.1:8080 GMX_TOKEN=... GMX_SOURCE=lobby python dev/live_controls.py

Needs a playlist source with items called welcome, soon and coffee, the
first two at least five seconds long. Standard
library only.
"""
import json, os, sys, time, urllib.request
BASE = os.environ.get("GMX_URL", "http://127.0.0.1:8080") + "/api/v1"
TOKEN = os.environ.get("GMX_TOKEN", "")
SOURCE = os.environ.get("GMX_SOURCE", "lobby")
def post(path, body):
    req = urllib.request.Request(BASE + path, data=json.dumps(body).encode(), method="POST",
          headers={"Authorization": "Bearer " + TOKEN, "content-type": "application/json"})
    try:
        return json.load(urllib.request.urlopen(req, timeout=10))
    except urllib.error.HTTPError as e:
        return {"error": e.read().decode()[:300]}
def tool(name, **a):
    a["source"] = SOURCE
    return post("/tool/call", {"name": "playout/" + name, "source": SOURCE, "arguments": a})
def st():
    return tool("state")["structuredContent"]
ok = True
def check(what, cond, detail=""):
    global ok
    ok &= bool(cond)
    print(("ok   " if cond else "FAIL ") + what + ("" if cond else "  " + str(detail)))
if __name__ == "__main__":
  s = st(); cur = s["current"]["id"]
  tool("play_item", id="soon"); time.sleep(1.5); s = st()
  check("play_item puts that item on now", s["current"]["id"] == "soon", s["current"])
  tool("cue", id="welcome"); time.sleep(0.3); s = st()
  check("cue makes it next", s["next"] == "welcome" and s["cued"] == "welcome", (s["next"], s["cued"]))
  tool("next"); time.sleep(1.5); s = st()
  check("next plays the cued item", s["current"]["id"] == "welcome", s["current"])
  check("the cue is used up", s["cued"] is None, s["cued"])
  tool("seek", position_ms=4000); time.sleep(1.2); s = st()
  check("seek lands near 4 s", 4000 <= s["current"]["position_ms"] <= 5600, s["current"])
  tool("pause"); time.sleep(0.4); p1 = st()["current"]["position_ms"]; time.sleep(1.0); s = st()
  check("pause holds the position", s["mode"] == "paused" and s["current"]["position_ms"] == p1, (s["mode"], p1, s["current"]))
  tool("play"); time.sleep(1.0); s = st()
  check("play resumes", s["mode"] == "playing" and s["current"]["position_ms"] > p1, (s["mode"], s["current"]))
  tool("previous"); time.sleep(1.5); s = st()
  check("previous after 3 s restarts the item", s["current"]["id"] == "welcome" and s["current"]["position_ms"] < 2500, s["current"])
  r = tool("play_item", id="nope")
  check("an unknown item is refused with the ids that exist", "error" in r and "coffee" in r["error"], r)
  r = post("/tool/call", {"name": "playout/state", "source": "nope", "arguments": {}})
  check("an unknown source is refused naming the sources", "error" in r and SOURCE in r["error"], r)
  print("ALL OK" if ok else "SOME FAILED")
