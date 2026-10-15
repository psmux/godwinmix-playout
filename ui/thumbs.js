// Thumbnails, asked for through the `thumbnail` tool and kept in memory.
// A row asks only when it scrolls into view. When the plugin has not made a
// picture yet it answers ready:false, and the ask is repeated once
// `thumbs_made` in the state answer has grown.

const cache = new Map();
const WIDTH = 240;

const keyOf = (it) => `${it.path}|${it.in_ms || 0}`;
export const cached = (it) => cache.get(keyOf(it)) || null;

export class Thumbs {
  constructor(model) {
    this.model = model;
    this.queue = [];
    this.waiting = new Map();
    this.active = 0;
    this.made = -1;
    this.dead = new Set();
    this.off = model.subscribe((m) => this.seen(m.st));
  }

  destroy() { this.off(); this.queue = []; this.waiting.clear(); }

  /** Call back with a data URI now or later. Returns a cancel function. */
  want(it, cb) {
    const key = keyOf(it);
    if (cache.has(key)) { cb(cache.get(key)); return () => {}; }
    if (this.dead.has(key) || it.error || it.kind === "audio" || it.kind === null) return () => {};
    const job = { key, id: it.id, cb };
    this.queue.push(job);
    this.pump();
    return () => { job.cb = null; };
  }

  seen(st) {
    if (!st) return;
    if (st.thumbs_made === this.made) return this.retry(false);
    const grew = st.thumbs_made > this.made && this.made >= 0;
    this.made = st.thumbs_made;
    this.retry(grew);
  }

  /** Ask again when the count grew, and anyway every 4 seconds, up to 8 times. */
  retry(grew) {
    const now = performance.now();
    const stale = now - (this.last || 0) > 4000;
    if (this.waiting.size && (grew || stale)) {
      this.last = now;
      for (const [k, job] of this.waiting) if ((job.tries = (job.tries || 0) + 1) > 8) this.waiting.delete(k);
      this.queue.push(...this.waiting.values());
      this.waiting.clear();
      this.pump();
    }
  }

  pump() {
    while (this.active < 2 && this.queue.length) {
      const job = this.queue.shift();
      if (!job.cb) continue;
      this.active++;
      this.model.tool("thumbnail", { id: job.id, width: WIDTH })
        .then((r) => {
          if (r && r.ready && r.uri) { cache.set(job.key, r.uri); if (job.cb) job.cb(r.uri); }
          else if (job.cb) this.waiting.set(job.key + job.id, job);
        })
        .catch(() => this.dead.add(job.key))
        .finally(() => { this.active--; this.pump(); });
    }
  }
}
