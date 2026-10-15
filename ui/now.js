// The now playing card: picture, progress with seek, times, transport,
// loop and shuffle, what is next and what is cued.

import { h, fmt, clockAt } from "./util.js";
import { icon } from "./icons.js";
import { cached } from "./thumbs.js";
import { bindSeek } from "./seek.js";

const MODES = { playing: "Playing", paused: "Paused", holding: "Holding", ended: "Ended", idle: "Idle" };
const LOOPS = { all: "Loop all", one: "Loop one", off: "No loop" };

export function loopNext(v) { return v === "all" ? "one" : v === "one" ? "off" : "all"; }

function badge(text, cls = "") { return h("span.po-badge" + cls, { text }); }

export class NowCard {
  constructor(model, thumbs) {
    this.m = model;
    this.thumbs = thumbs;
    this.img = h("img.po-img", { alt: "", hidden: true });
    this.chip = h("span.po-chip");
    this.title = h("div.po-now-title");
    this.badges = h("div.po-badges");
    this.fill = h("div.po-fill");
    this.bar = h("div.po-bar", { role: "slider", tabindex: "0", "aria-label": "Position in the playing item", "aria-valuemin": "0" }, [this.fill, h("div.po-knob")]);
    this.el1 = h("span.num");
    this.el2 = h("span.num");
    this.stats = h("div.po-stats");
    this.nextImg = h("img.po-img", { alt: "", hidden: true });
    this.nextTitle = h("span.ellipsis");
    this.cueBox = h("div.po-cue", { hidden: true });
    this.play = h("button.po-btn.big.primary", { onclick: () => this.toggle() });
    this.loop = h("button.po-btn.tog", { onclick: () => this.m.setOption("loop", loopNext(this.m.st.loop)) });
    this.shuf = h("button.po-btn.tog", { onclick: () => this.m.setOption("shuffle", !this.m.st.shuffle) }, [icon("shuffle"), h("span.lbl", { text: "Shuffle" })]);
    const prev = h("button.po-btn.big", { title: "Previous (P)", "aria-label": "Previous", onclick: () => this.m.act("previous") }, [icon("prev", 22)]);
    const next = h("button.po-btn.big", { title: "Next (N)", "aria-label": "Next", onclick: () => this.m.act("next") }, [icon("next", 22)]);
    this.el = h("section.po-now", { "aria-label": "Now playing" }, [
      h("div.po-now-top", {}, [
        h("div.po-media", {}, [this.img, this.chip]),
        h("div.po-now-info", {}, [this.title, this.badges, this.bar, h("div.po-times", {}, [this.el1, this.el2]), this.stats]),
      ]),
      h("div.po-transport", {}, [prev, this.play, next, h("span.grow"), this.loop, this.shuf]),
      h("div.po-nextrow", {}, [
        h("div.po-next", {}, [h("div.po-mini", {}, [this.nextImg]), h("div.po-next-text", {}, [h("span.faint.sm", { text: "Next up" }), this.nextTitle])]),
        this.cueBox,
      ]),
    ]);
    this.seek = bindSeek(this.bar, model);
    this.shown = {};
  }

  toggle() { this.m.act(this.m.st && this.m.st.mode === "playing" ? "pause" : "play"); }

  text(node, key, value) {
    if (node.textContent !== value) node.textContent = value;
  }

  thumb(img, it, key) {
    if (!it) { img.hidden = true; this.shown[key] = null; return; }
    const sig = it.path + "|" + it.in_ms;
    if (this.shown[key] === sig) return;
    this.shown[key] = sig;
    img.hidden = true;
    const set = (uri) => { if (this.shown[key] === sig) { img.src = uri; img.hidden = false; } };
    const hit = cached(it);
    if (hit) set(hit); else this.thumbs.want(it, set);
  }

  /** Called with each new answer. */
  update() {
    const st = this.m.st;
    if (!st) return;
    const cur = st.current && this.m.item(st.current.id);
    this.thumb(this.img, cur, "img");
    this.text(this.title, "title", cur ? cur.title : st.mode === "ended" ? "The list has ended" : "Nothing playing");
    this.chip.textContent = MODES[st.mode] || st.mode;
    this.chip.dataset.mode = st.mode;
    const key = cur ? [cur.kind, cur.codec, cur.width, cur.height, st.current.decoder].join("|") : "";
    if (this.shown.badges !== key) {
      this.shown.badges = key;
      this.badges.replaceChildren(...(cur ? [cur.kind && badge(cur.kind), cur.codec && badge(cur.codec), cur.width && badge(`${cur.width}x${cur.height}`),
        st.current.decoder && badge(st.current.decoder, ".dim")].filter(Boolean) : []));
    }
    this.play.replaceChildren(icon(st.mode === "playing" ? "pause" : "play", 28));
    const label = st.mode === "playing" ? "Pause" : st.mode === "holding" ? "Continue" : "Play";
    this.play.title = `${label} (Space)`;
    this.play.setAttribute("aria-label", label);
    this.loop.replaceChildren(icon("loop"), h("span.lbl", { text: LOOPS[st.loop] || "Loop" }), st.loop === "one" ? h("b.one", { text: "1" }) : "");
    this.loop.classList.toggle("on", st.loop !== "off");
    this.shuf.classList.toggle("on", !!st.shuffle);
    this.loop.title = "Loop (L)";
    this.shuf.title = "Shuffle (S)";
    const nx = st.next && this.m.item(st.next);
    this.thumb(this.nextImg, nx, "nimg");
    this.text(this.nextTitle, "ntitle", nx ? nx.title + (st.next_ready === false ? " (loading)" : "") : st.loop === "off" ? "End of list" : "Nothing");
    this.cueBox.hidden = !st.cued;
    const cu = st.cued && this.m.item(st.cued);
    if (st.cued) {
      this.cueBox.replaceChildren(h("span.faint.sm", { text: "Cued" }), h("span.ellipsis", { text: cu ? cu.title : st.cued }),
        h("button.po-btn.icon", { title: "Clear the cue", "aria-label": "Clear the cue", onclick: () => this.m.act("cue", { id: "" }) }, [icon("close", 14)]));
    }
    this.frame(performance.now());
  }

  /** Called every animation frame while something moves. */
  frame(now) {
    const st = this.m.st;
    if (!st) return;
    const c = st.current;
    const pos = this.m.position(now);
    const len = c && c.length_ms;
    const frac = len ? Math.min(1, pos / len) : 0;
    if (!this.seek.dragging) {
      this.fill.style.transform = `scaleX(${frac})`;
      this.bar.style.setProperty("--p", frac);
      this.text(this.el1, "e1", c ? fmt(pos) : "0:00");
      this.text(this.el2, "e2", c && len ? `${fmt(len - pos)} left` : c ? "live" : "");
    }
    this.bar.setAttribute("aria-valuenow", String(Math.round(pos)));
    this.bar.setAttribute("aria-valuemax", String(len || 0));
    this.bar.classList.toggle("off", !len);
    const rem = this.m.listRemaining(now);
    const part = st.list && st.list.unknown_lengths ? "about " : "";
    const line = rem == null ? "" : `List left ${part}${fmt(rem)}${st.loop === "off" ? ` · ends ${clockAt(rem)}` : ` · pass ends ${clockAt(rem)}`}`;
    this.text(this.stats, "stats", line);
  }
}
