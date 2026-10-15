// The Playout panel: run a playlist source from the web UI.
//
// A playlist is a source of type playout/playlist. This panel finds them in
// the client's state, polls the plugin's `state` tool about twice a second
// while it is on screen and the page is visible, and moves the progress bars
// between answers with requestAnimationFrame. Everything the panel does goes
// through the public client, the same calls any other client can make.

import { h } from "./util.js";
import { Model } from "./edits.js";
import { Thumbs } from "./thumbs.js";
import { NowCard } from "./now.js";
import { List } from "./list.js";
import { bindReorder } from "./reorder.js";
import { bindKeys } from "./keys.js";
import { buildView, paintView } from "./view.js";
import { toast } from "./toast.js";

window.godwinmixPanels = window.godwinmixPanels || [];

const TYPE = "playout/playlist";
const STORE = "playout.selected";
const HERE = new URL(".", import.meta.url).href;

function loadStyles() {
  if (document.querySelector("link[data-playout]")) return;
  for (const f of ["panel.css", "list.css", "dialog.css"]) document.head.append(h("link", { rel: "stylesheet", href: HERE + f, "data-playout": "" }));
}

class PlayoutPanel extends HTMLElement {
  static get panel() {
    return { id: "playout/panel", title: "Playout", slots: ["sidebar", "main"] };
  }

  setClient(client) { this.client = client; }

  connectedCallback() {
    loadStyles();
    this.model = null;
    this.playlists = [];
    this.build();
    this.offRender = this.client.onRender((s) => this.found(s));
    this.offOpen = this.client.on("open", () => this.model && this.model.again(0));
    this.raf = 0;
    this.visible = () => this.loop();
    document.addEventListener("visibilitychange", this.visible);
  }

  disconnectedCallback() {
    this.offRender(); this.offOpen(); if (this.unbindKeys) this.unbindKeys();
    document.removeEventListener("visibilitychange", this.visible);
    cancelAnimationFrame(this.raf);
    this.files.destroy();
    this.drop();
  }

  drop() {
    if (!this.model) return;
    this.offModel(); this.model.stop(); this.thumbs.destroy();
    this.model = null;
  }

  build() { buildView(this); }

  paint() { paintView(this); }

  openLibrary() {
    if (this.model) import("./library.js").then((m) => m.openLibrary(this.client, this.model));
  }

  help() { import("./help.js").then((m) => m.openHelp()); }

  create() {
    const taken = new Set((this.client.state.sources || []).map((s) => s.id));
    import("./create.js").then((m) => m.openCreate(this.client, taken, (id) => { this.choose(id); }));
  }

  /** The playlist sources in the client's state; called at each flush. */
  found(state) {
    const ids = (state.sources || []).filter((s) => s.type === TYPE).map((s) => ({ id: s.id, name: s.name || s.id }));
    const sig = JSON.stringify(ids);
    if (sig === this.sig) return;
    this.sig = sig;
    this.playlists = ids;
    const saved = localStorage.getItem(STORE);
    const keep = this.model && ids.find((p) => p.id === this.model.id);
    const pick = keep ? keep.id : ids.find((p) => p.id === saved) ? saved : ids[0] && ids[0].id;
    if (!pick) this.drop();
    else if (!keep) this.choose(pick);
    this.paint();
  }

  choose(id) {
    if (this.model && this.model.id === id) return;
    localStorage.setItem(STORE, id);
    this.drop();
    const model = new Model(this.client, id);
    this.model = model;
    this.thumbs = new Thumbs(model);
    this.now = new NowCard(model, this.thumbs);
    this.nowHost.replaceChildren(this.now.el);
    this.list = new List(model, this.thumbs, {
      play: (i) => model.act("play_item", { id: i }),
      cue: (i) => model.act("cue", { id: model.st && model.st.cued === i ? "" : i }),
      edit: (i) => import("./editor.js").then((m) => m.openEditor(model, i)),
      remove: (i) => this.remove(i),
    });
    this.listHost.replaceChildren(this.list.el);
    this.hint = null;
    this.search.value = "";
    bindReorder(this.list, { enabled: () => !this.list.query, onMove: (i, to) => model.move(i, to) });
    this.unbindKeys && this.unbindKeys();
    this.unbindKeys = bindKeys(this.root, { model: () => this.model, list: this.list, search: this.search, help: () => this.help(), remove: (i) => this.remove(i) });
    this.offModel = model.subscribe(() => this.onModel());
    model.start();
    this.paint();
  }

  remove(id) {
    const m = this.model, it = m.item(id);
    if (!it) return;
    const at = m.items.findIndex((i) => i.id === id);
    m.remove(id);
    toast(`Removed ${it.title || "the item"}.`, { ms: 6000, action: { label: "Undo", run: () => m.insert(it, at) } });
  }

  onModel() {
    this.now.update();
    this.list.render();
    this.paint();
    this.loop();
  }

  /** Animate the bars between answers, only while something moves. */
  loop() {
    cancelAnimationFrame(this.raf);
    if (!this.model || !this.model.playing || document.hidden) return;
    const step = (now) => {
      this.now.frame(now);
      this.list.frame(now);
      this.raf = requestAnimationFrame(step);
    };
    this.raf = requestAnimationFrame(step);
  }
}

customElements.define("gmx-playout-panel", PlayoutPanel);
window.godwinmixPanels.push(PlayoutPanel);
