// The list: rows keyed by item id, a search filter, selection, and the live
// progress line on the playing row.

import { h } from "./util.js";
import { Row, itemLength } from "./row.js";

export class List {
  constructor(model, thumbs, actions) {
    this.m = model;
    this.rows = new Map();
    this.selected = null;
    this.query = "";
    this.ctl = {
      ...actions,
      thumbs,
      select: (id, e) => this.select(id, e),
    };
    this.el = h("ol.po-list", { role: "listbox", "aria-label": "Playlist items" });
  }

  select(id, e) {
    this.selected = id;
    this.sync();
    if (e && e.type === "click") this.el.closest(".po")?.focus({ preventScroll: true });
  }

  visibleIds() { return [...this.el.children].filter((li) => !li.hidden).map((li) => li.dataset.id); }

  /** Match words against title and path, the way the media search does. */
  matches(it) {
    const words = this.query.toLowerCase().split(/\s+/).filter(Boolean);
    const hay = `${it.title} ${it.path}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  }

  render() {
    const items = this.m.items;
    const ids = new Set(items.map((i) => i.id));
    for (const [id, row] of this.rows) if (!ids.has(id)) { row.destroy(); this.rows.delete(id); }
    let prev = null;
    items.forEach((it, i) => {
      let row = this.rows.get(it.id);
      if (!row) { row = new Row(it, this.ctl); this.rows.set(it.id, row); }
      row.setItem(it, i);
      row.el.hidden = !this.matches(it);
      const want = prev ? prev.nextSibling : this.el.firstChild;
      if (want !== row.el) this.el.insertBefore(row.el, want);
      prev = row.el;
    });
    if (this.selected && !ids.has(this.selected)) this.selected = null;
    this.sync();
  }

  sync() {
    const st = this.m.st;
    if (!st) return;
    const cur = st.current && st.current.id;
    for (const [id, row] of this.rows) row.flag(id === cur, id === st.next, id === st.cued, id === this.selected);
    const sel = this.selected && this.rows.get(this.selected);
    if (sel) this.el.setAttribute("aria-activedescendant", sel.el.id);
    else this.el.removeAttribute("aria-activedescendant");
  }

  frame(now) {
    const st = this.m.st;
    const c = st && st.current;
    const row = c && this.rows.get(c.id);
    if (!row) return;
    const len = c.length_ms || itemLength(row.item);
    row.prog.style.transform = `scaleX(${len ? Math.min(1, this.m.position(now) / len) : 0})`;
  }

  moveSelection(delta) {
    const ids = this.visibleIds();
    if (!ids.length) return;
    const at = ids.indexOf(this.selected);
    this.selected = ids[Math.max(0, Math.min(ids.length - 1, at < 0 ? 0 : at + delta))];
    this.sync();
    this.rows.get(this.selected).el.scrollIntoView({ block: "nearest" });
  }

  setQuery(q) { this.query = q; this.render(); }
}
