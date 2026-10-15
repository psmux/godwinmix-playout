// Drag to reorder with pointer events. HTML5 drag and drop is not used, because
// WebView2 swallows it inside the page. A mouse drags a row anywhere on it; a
// finger drags by the grip, so the rest of the row still scrolls the page.

const THRESHOLD = 5;
const EDGE = 48;

function scroller(node) {
  for (let n = node.parentElement; n; n = n.parentElement) {
    const o = getComputedStyle(n).overflowY;
    if ((o === "auto" || o === "scroll") && n.scrollHeight > n.clientHeight) return n;
  }
  return document.scrollingElement;
}

export function bindReorder(list, { enabled, onMove }) {
  const el = list.el;
  let s = null;
  let line = null;

  const rows = () => [...el.children].filter((li) => li.matches("li.po-row") && !li.hidden && li !== s?.row);
  const indexAt = (y) => {
    const all = rows();
    let i = 0;
    for (const li of all) { const r = li.getBoundingClientRect(); if (y > r.top + r.height / 2) i++; }
    return i;
  };
  const place = () => {
    const all = rows();
    const i = s.to;
    const ref = all[i] || all[all.length - 1];
    if (!ref) return;
    const r = ref.getBoundingClientRect(), box = el.getBoundingClientRect();
    line.style.transform = `translateY(${(all[i] ? r.top : r.bottom) - box.top - 1}px)`;
  };
  const frame = () => {
    if (!s || !s.live) return;
    const sc = s.scroller, r = sc === document.scrollingElement ? { top: 0, bottom: innerHeight } : sc.getBoundingClientRect();
    const d = s.y < r.top + EDGE ? -12 : s.y > r.bottom - EDGE ? 12 : 0;
    if (d) { sc.scrollTop += d; s.to = indexAt(s.y); place(); }
    s.ghost.style.transform = `translateY(${s.y - s.grab - s.top}px)`;
    s.raf = requestAnimationFrame(frame);
  };
  const begin = () => {
    s.live = true;
    const r = s.row.getBoundingClientRect();
    s.top = r.top;
    s.grab = s.startY - r.top;
    s.ghost = s.row.cloneNode(true);
    s.ghost.classList.add("ghost");
    Object.assign(s.ghost.style, { position: "fixed", left: r.left + "px", top: r.top + "px", width: r.width + "px", zIndex: 50, pointerEvents: "none" });
    document.body.append(s.ghost);
    s.row.classList.add("lifted");
    el.classList.add("dragging");
    line = document.createElement("div");
    line.className = "po-dropline";
    el.append(line);
    s.scroller = scroller(el);
    s.to = indexAt(s.y);
    place();
    frame();
  };
  const finish = (commit) => {
    if (!s) return;
    cancelAnimationFrame(s.raf);
    if (s.live) {
      s.ghost.remove(); line.remove();
      s.row.classList.remove("lifted");
      el.classList.remove("dragging");
      const ids = [...el.children].filter((li) => li.dataset.id).map((li) => li.dataset.id);
      const from = ids.indexOf(s.row.dataset.id);
      if (commit && s.to !== from) onMove(s.row.dataset.id, s.to);
      const swallow = (e) => e.stopPropagation();
      el.addEventListener("click", swallow, true);
      setTimeout(() => el.removeEventListener("click", swallow, true), 60);
    }
    try { el.releasePointerCapture(s.pid); } catch { /* already released */ }
    s = null;
  };

  el.addEventListener("pointerdown", (e) => {
    const row = e.target.closest("li.po-row");
    if (!row || e.button !== 0 || !enabled() || e.target.closest(".po-acts, .po-warn")) return;
    const touch = e.pointerType !== "mouse";
    if (touch && !e.target.closest(".po-grip")) return;
    s = { row, pid: e.pointerId, startY: e.clientY, y: e.clientY, live: false };
  });
  el.addEventListener("pointermove", (e) => {
    if (!s || e.pointerId !== s.pid) return;
    s.y = e.clientY;
    if (!s.live && Math.abs(s.y - s.startY) > THRESHOLD) { el.setPointerCapture(s.pid); begin(); }
    if (s.live) { s.to = indexAt(s.y); place(); }
  });
  el.addEventListener("pointerup", () => finish(true));
  el.addEventListener("pointercancel", () => finish(false));
  addEventListener("keydown", (e) => { if (e.key === "Escape" && s && s.live) { e.stopPropagation(); finish(false); } }, true);
}
