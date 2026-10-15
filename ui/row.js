// One row of the list. A row is built once per item id and updated in place,
// so a poll that changes nothing touches nothing.

import { h, fmt } from "./util.js";
import { icon, kindIcon } from "./icons.js";
import { cached } from "./thumbs.js";

export function itemLength(it) {
  if (it.length_ms != null) return it.length_ms;
  return it.kind === "picture" && it.still_ms ? it.still_ms : null;
}

function button(name, label, run) {
  return h("button.po-btn.icon.act", { title: label, "aria-label": label, tabindex: "-1",
    onclick: (e) => { e.stopPropagation(); run(); } }, [icon(name, 16)]);
}

export class Row {
  constructor(it, ctl) {
    this.id = it.id;
    this.ctl = ctl;
    this.sig = "";
    this.img = h("img.po-img", { alt: "", hidden: true, loading: "lazy" });
    this.th = h("div.po-th", {}, [this.img, h("span.po-kind")]);
    this.num = h("span.po-num");
    this.title = h("div.po-rtitle");
    this.meta = h("div.po-rmeta");
    this.err = h("div.po-rerr", { hidden: true });
    this.len = h("span.po-len.num");
    this.prog = h("div.po-rprog");
    const id = it.id;
    this.el = h("li.po-row", { "data-id": id, role: "option", id: `po-row-${id}` }, [
      h("span.po-grip", { title: "Drag to move, or Alt+Up and Alt+Down" }, [icon("grip", 18)]),
      this.num,
      this.th,
      h("div.po-rtext", {}, [this.title, this.meta, this.err]),
      this.len,
      h("div.po-acts", {}, [
        button("play", "Play now (Enter)", () => ctl.play(id)),
        button("cue", "Cue next (C)", () => ctl.cue(id)),
        button("edit", "Edit", () => ctl.edit(id)),
        button("trash", "Remove (Delete)", () => ctl.remove(id)),
      ]),
      this.prog,
    ]);
    this.el.addEventListener("click", (e) => { if (!e.target.closest(".po-acts")) ctl.select(id, e); });
    this.el.addEventListener("dblclick", (e) => { if (!e.target.closest(".po-acts")) ctl.play(id); });
    this.observer = null;
    this.watch();
  }

  /** Ask for the picture only when the row is on screen. */
  watch() {
    this.observer = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) { this.visible = true; this.picture(); }
    }, { rootMargin: "120px" });
    this.observer.observe(this.el);
  }

  picture() {
    if (!this.visible || !this.item) return;
    const it = this.item;
    const sig = it.path + "|" + it.in_ms;
    if (this.picSig === sig) return;
    this.picSig = sig;
    this.img.hidden = true;
    const set = (uri) => { if (this.picSig === sig) { this.img.src = uri; this.img.hidden = false; } };
    const hit = cached(it);
    if (hit) set(hit); else if (this.ctl.thumbs) this.cancel = this.ctl.thumbs.want(it, set);
  }

  destroy() { this.observer.disconnect(); if (this.cancel) this.cancel(); this.el.remove(); }

  /** Content that depends on the item; skipped when nothing in it changed. */
  setItem(it, index) {
    this.item = it;
    this.num.textContent = String(index + 1);
    const len = itemLength(it);
    const sig = JSON.stringify([it.title, it.kind, it.codec, it.width, it.height, len, it.error, it.in_ms, it.out_ms, it.still_ms, it.hold, it.probed]);
    if (sig === this.sig) return;
    this.sig = sig;
    this.title.textContent = it.title || it.path;
    this.title.title = it.path;
    this.len.textContent = len != null ? fmt(len) : it.probed === false ? "..." : "--:--";
    const bits = [it.codec, it.width ? `${it.width}x${it.height}` : null, it.in_ms || it.out_ms ? "trimmed" : null, it.hold ? "holds" : null].filter(Boolean);
    this.meta.textContent = bits.join(" · ");
    this.th.lastChild.replaceChildren(kindIcon(it.kind));
    this.el.classList.toggle("bad", !!it.error);
    this.err.hidden = true;
    this.err.textContent = it.error || "";
    this.warn = this.warn || h("button.po-warn", { title: "", "aria-label": "Why this will not play",
      onclick: (e) => { e.stopPropagation(); this.err.hidden = !this.err.hidden; } }, [icon("warn", 15)]);
    this.warn.title = it.error || "";
    if (it.error) this.th.append(this.warn); else this.warn.remove();
    this.picture();
  }

  /** State that changes every answer: playing, next, cued, selected. */
  flag(playing, next, cued, selected) {
    const c = this.el.classList;
    c.toggle("playing", playing);
    c.toggle("next", next && !playing);
    c.toggle("cued", cued);
    c.toggle("sel", selected);
    this.el.setAttribute("aria-selected", selected ? "true" : "false");
    if (!playing) this.prog.style.transform = "scaleX(0)";
  }
}
