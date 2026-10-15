// Keyboard shortcuts while the panel has focus. Typing in a field never
// triggers one, except Escape and the arrows inside the search box.

import { loopNext } from "./now.js";

export const SHORTCUTS = [
  ["Space", "Play or pause"],
  ["N or Right", "Next item"],
  ["P or Left", "Previous item"],
  ["Up, Down", "Move the selection"],
  ["Enter", "Play the selected item"],
  ["C", "Cue the selected item next"],
  ["Delete", "Remove the selected item"],
  ["Alt+Up, Alt+Down", "Move the selected item"],
  ["L", "Cycle loop: all, one, off"],
  ["S", "Shuffle on or off"],
  ["/", "Search the list"],
  ["?", "Show this help"],
];

/** ctl: {model, list, search, help}. Returns an unbind function. */
export function bindKeys(root, ctl) {
  const onKey = (e) => {
    if (e.ctrlKey || e.metaKey) return;
    const m = ctl.model(), st = m.st;
    const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName) && e.target.type !== "checkbox";
    if (typing) { if (e.key === "Escape" && e.target === ctl.search) { e.target.value = ""; e.target.dispatchEvent(new Event("input")); root.focus(); } return; }
    if (e.target.closest("button") && (e.key === " " || e.key === "Enter")) return;
    if (e.target.closest("[role=slider]") && e.key.startsWith("Arrow")) return;
    const sel = ctl.list.selected;
    const key = e.key;
    const done = () => { e.preventDefault(); e.stopPropagation(); };
    if (e.altKey && (key === "ArrowUp" || key === "ArrowDown") && sel) {
      done();
      const at = m.items.findIndex((i) => i.id === sel);
      m.move(sel, at + (key === "ArrowUp" ? -1 : 1));
      return;
    }
    if (e.altKey) return;
    const k = key.length === 1 ? key.toLowerCase() : key;
    if (k === " ") { done(); m.act(st && st.mode === "playing" ? "pause" : "play"); }
    else if (k === "n" || k === "ArrowRight") { done(); m.act("next"); }
    else if (k === "p" || k === "ArrowLeft") { done(); m.act("previous"); }
    else if (k === "ArrowDown") { done(); ctl.list.moveSelection(1); }
    else if (k === "ArrowUp") { done(); ctl.list.moveSelection(-1); }
    else if (k === "Enter" && sel) { done(); m.act("play_item", { id: sel }); }
    else if (k === "c" && sel) { done(); m.act("cue", { id: st && st.cued === sel ? "" : sel }); }
    else if ((k === "Delete" || k === "Backspace") && sel) { done(); ctl.remove(sel); }
    else if (k === "l" && st) { done(); m.setOption("loop", loopNext(st.loop)); }
    else if (k === "s" && st) { done(); m.setOption("shuffle", !st.shuffle); }
    else if (k === "/") { done(); ctl.search.focus(); }
    else if (k === "?") { done(); ctl.help(); }
  };
  root.addEventListener("keydown", onKey);
  return () => root.removeEventListener("keydown", onKey);
}
