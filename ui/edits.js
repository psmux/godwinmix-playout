// The writing half of a playlist: edits to the list and its options. Every edit
// is shown at once, sent whole through `source.set`, and confirmed by the next
// answer from `state`.

import { basename, newItemId } from "./util.js";
import { errorToast } from "./toast.js";
import { Poller } from "./model.js";

export class Model extends Poller {
  get items() { return (this.st && this.st.items) || []; }
  item(id) { return this.items.find((i) => i.id === id) || null; }

  wire(it) {
    const out = { id: it.id, path: it.path };
    // `title_set` is what the params hold; `title` falls back to the file name.
    const title = this.custom.has(it.id) ? this.custom.get(it.id) : (it.title_set ?? it.title);
    if (title && title !== basename(it.path)) out.title = title;
    for (const k of ["in_ms", "out_ms", "still_ms"]) if (it[k] > 0) out[k] = Math.round(it[k]);
    if (it.hold) out.hold = true;
    return out;
  }

  /** Show a new list now, send it, and let the next answer confirm it. */
  async setItems(items) {
    if (!this.st) return;
    const known = new Map(this.items.map((i) => [i.id, i]));
    this.st = { ...this.st, items: items.map((i) => ({ ...(known.get(i.id) || { kind: null, error: null }), ...i })) };
    if (!this.st.shuffle) this.st.order = items.map((i) => i.id);
    this.emit();
    await this.write({ items: items.map((i) => this.wire(i)) });
  }

  async write(params) {
    this.writes++;
    this.inflight++;
    try {
      await this.client.call("source.set", { id: this.id, params });
    } catch (e) {
      errorToast(e);
    } finally {
      this.inflight--;
      this.writes++;
      this.again(60);
    }
  }

  setOption(key, value) {
    if (this.st) { this.st = { ...this.st, [key]: value }; this.emit(); }
    return this.write({ [key]: value });
  }

  update(id, patch) {
    if (patch.title !== undefined) this.custom.set(id, patch.title);
    return this.setItems(this.items.map((i) => (i.id === id ? { ...i, ...patch } : i)));
  }

  remove(id) { return this.setItems(this.items.filter((i) => i.id !== id)); }

  move(id, to) {
    const list = this.items.slice();
    const from = list.findIndex((i) => i.id === id);
    if (from < 0 || to < 0 || to >= list.length || from === to) return Promise.resolve();
    list.splice(to, 0, list.splice(from, 1)[0]);
    return this.setItems(list);
  }

  /** Put an item back where it was (undo of a remove). */
  insert(item, at) {
    if (this.item(item.id)) return Promise.resolve();
    const list = this.items.slice();
    list.splice(Math.min(at, list.length), 0, item);
    return this.setItems(list);
  }

  /** Add paths at the end (or at `at`), each with a fresh id. */
  addPaths(paths, at) {
    const taken = new Set(this.items.map((i) => i.id));
    const fresh = paths.map((path) => {
      const id = newItemId(taken);
      taken.add(id);
      return { id, path, title: basename(path), in_ms: 0, out_ms: 0, still_ms: 0, hold: false };
    });
    const list = this.items.slice();
    list.splice(at ?? list.length, 0, ...fresh);
    return this.setItems(list);
  }
}
