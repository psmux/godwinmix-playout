// The panel's fixed skeleton and the parts that change with the answers:
// tabs, banner, empty states and the summary line.

import { h, fmt } from "./util.js";
import { icon } from "./icons.js";
import { Files } from "./files.js";

const lazy = (name, fn) => import(name).then(fn);

export function buildView(p) {
  const btn = (name, label, run) => h("button.po-btn", { title: label, "aria-label": label, onclick: run }, [icon(name, 18)]);
  p.tabs = h("div.po-tabs", { role: "tablist", "aria-label": "Playlists" });
  p.banner = h("div.po-banner", { hidden: true, role: "status" });
  p.nowHost = h("div");
  p.search = h("input.po-search", { type: "search", placeholder: "Search this list ( / )", "aria-label": "Search this list",
    oninput: (e) => { if (p.list) p.list.setQuery(e.target.value); } });
  p.empty = h("div.po-empty");
  p.summary = h("div.po-summary.faint.sm");
  p.listHost = h("div.po-listwrap");
  const lib = h("button.po-btn", { onclick: () => p.openLibrary() }, [icon("library", 16), h("span", { text: "Library" })]);
  const up = h("button.po-btn", { onclick: () => p.files.browse() }, [icon("upload", 16), h("span", { text: "Upload" })]);
  const bar = h("div.po-toolbar", {}, [lib, up, h("div.po-searchbox", {}, [icon("search", 14), p.search])]);
  p.body = h("div.po-body", {}, [p.nowHost, bar, p.listHost, p.summary]);
  p.root = h("div.po", { tabindex: "0", "aria-label": "Playout" }, [
    h("header.po-head", {}, [p.tabs,
      btn("plus", "New playlist", () => p.create()),
      btn("gear", "Playlist options", () => p.model && lazy("./create.js", (m) => m.openOptions(p.model))),
      btn("help", "Keyboard shortcuts ( ? )", () => p.help())]),
    p.banner, p.empty, p.body,
  ]);
  p.append(p.root);
  p.files = new Files(p.root, p.client, () => p.model);
  p.root.append(p.files.status);
}

export function paintView(p) {
  const none = !p.playlists.length;
  p.body.hidden = none;
  p.empty.hidden = !none;
  if (none && !p.empty.firstChild) {
    p.empty.append(icon("play", 30), h("strong", { text: "No playlist yet" }),
      h("p", { text: "Make a playlist, add video, audio or picture files, and press play. It becomes a source you can put in any scene." }),
      h("button.po-btn.primary.big", { text: "Create a playlist", onclick: () => p.create() }));
  }
  const mine = (id) => !!p.model && p.model.id === id;
  p.tabs.replaceChildren(...p.playlists.map((l) => h("button.po-tab" + (mine(l.id) ? ".on" : ""), {
    role: "tab", "aria-selected": String(mine(l.id)), text: l.name, title: l.id, onclick: () => p.choose(l.id) })));
  const m = p.model;
  p.banner.hidden = !(m && m.error);
  p.banner.textContent = m && m.error ? m.error : "";
  const st = m && m.st;
  if (!st) { p.summary.textContent = m ? "Loading the playlist" : ""; return; }
  const l = st.list || {};
  p.summary.textContent = st.items.length
    ? `${l.count ?? st.items.length} items · ${fmt(l.total_ms ?? 0)} total${l.unknown_lengths ? ` · ${l.unknown_lengths} of unknown length` : ""}` : "";
  emptyList(p, st.items.length === 0);
}

function emptyList(p, on) {
  if (!on) { if (p.hint) { p.hint.remove(); p.hint = null; } return; }
  if (p.hint) return;
  p.hint = h("div.po-empty.inline", {}, [icon("library", 26), h("strong", { text: "This playlist is empty" }),
    h("p", { text: "Add files from the library, upload them, or drop them here." }),
    h("button.po-btn.primary", { text: "Open the library", onclick: () => p.openLibrary() })]);
  p.listHost.append(p.hint);
}
