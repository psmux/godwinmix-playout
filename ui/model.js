// The polling half of a playlist: its last answer from the `state` tool, a local clock that
// carries the position between answers, and the writes that change the list.
//
// The list lives in the source's params and only `source.set` changes it. The
// panel keeps the whole list, edits a copy, and sends the full new list. It
// asks for `state` about twice a second while the panel is connected and the
// page is visible, and for nothing else on a timer.

import { errorToast } from "./toast.js";

const POLL_MS = 500;
const SLOW_MS = 1800;

export class Poller {
  constructor(client, id) {
    this.client = client;
    this.id = id;
    this.st = null;
    this.at = 0;
    this.error = "";
    this.writes = 0;
    this.inflight = 0;
    this.custom = new Map();
    this.subs = new Set();
    this.timer = 0;
    this.running = false;
    this.seeking = null;
    this.onVisible = () => (document.hidden ? this.pause() : this.again(0));
  }

  subscribe(fn) { this.subs.add(fn); return () => this.subs.delete(fn); }
  emit() { for (const fn of this.subs) fn(this); }

  start() {
    this.running = true;
    document.addEventListener("visibilitychange", this.onVisible);
    this.again(0);
  }
  stop() {
    this.running = false;
    document.removeEventListener("visibilitychange", this.onVisible);
    this.pause();
  }
  pause() { clearTimeout(this.timer); this.timer = 0; }

  again(ms) {
    clearTimeout(this.timer);
    this.timer = this.running && !document.hidden ? setTimeout(() => this.poll(), ms) : 0;
  }

  async poll() {
    this.timer = 0;
    if (!this.running || document.hidden) return;
    if (!this.client.state.connected) return this.again(SLOW_MS);
    const sent = performance.now();
    const gen = this.writes;
    try {
      const st = await this.tool("state");
      if (gen === this.writes && !this.inflight) this.accept(st, (sent + performance.now()) / 2);
      this.error = "";
    } catch (e) {
      this.error = e && e.message ? e.message : String(e);
      this.emit();
    }
    if (this.running && !this.timer) this.again(this.error ? SLOW_MS : POLL_MS);
  }

  accept(st, at) { this.st = st; this.at = at; this.seeking = null; this.emit(); }

  /** Run a playout tool on this source and answer its structured content. */
  async tool(name, args = {}) {
    const r = await this.client.call("tool.call", {
      name: `playout/${name}`, source: this.id, arguments: { source: this.id, ...args },
    });
    return r && r.structuredContent !== undefined ? r.structuredContent : r;
  }

  /** A transport action: send it, then ask again at once so the panel follows. */
  async act(name, args) {
    try { await this.tool(name, args); } catch (e) { errorToast(e); }
    this.again(40);
  }

  get playing() { return !!this.st && this.st.mode === "playing" && !!this.st.current; }

  /** Position in the current item now, from the last answer and the local clock. */
  position(now = performance.now()) {
    const c = this.st && this.st.current;
    if (!c) return 0;
    const base = this.seeking ?? c.position_ms;
    const moved = this.playing ? now - this.at : 0;
    const len = c.length_ms ?? Infinity;
    return Math.min(len, base + Math.max(0, moved));
  }

  /** Whole list time left now. */
  listRemaining(now = performance.now()) {
    const l = this.st && this.st.list;
    if (!l || l.remaining_ms == null) return null;
    return Math.max(0, l.remaining_ms - (this.playing ? now - this.at : 0));
  }

  seek(ms) {
    this.seeking = ms;
    this.at = performance.now();
    this.emit();
    return this.act("seek", { position_ms: Math.round(ms) });
  }
}
