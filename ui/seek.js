// Click or drag the progress bar to seek. Pointer events, so a finger works.
// While the bar is held the time shown is the target and nothing is sent;
// the seek goes out on release. Arrow keys on the focused bar move 5 seconds.

import { fmt } from "./util.js";

export function bindSeek(bar, model) {
  const ctl = { dragging: false };
  const fraction = (e) => {
    const r = bar.getBoundingClientRect();
    return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
  };
  const show = (f) => {
    const len = model.st.current.length_ms;
    bar.style.setProperty("--p", f);
    bar.firstChild.style.transform = `scaleX(${f})`;
    const t = bar.parentNode.querySelector(".po-times");
    t.firstChild.textContent = fmt(f * len);
    t.lastChild.textContent = `${fmt(len - f * len)} left`;
  };
  const seekable = () => model.st && model.st.current && model.st.current.length_ms;

  bar.addEventListener("pointerdown", (e) => {
    if (!seekable() || e.button !== 0) return;
    ctl.dragging = true;
    bar.setPointerCapture(e.pointerId);
    bar.classList.add("held");
    show(fraction(e));
    e.preventDefault();
  });
  bar.addEventListener("pointermove", (e) => { if (ctl.dragging) show(fraction(e)); });
  const end = (e, send) => {
    if (!ctl.dragging) return;
    ctl.dragging = false;
    bar.classList.remove("held");
    if (send && seekable()) model.seek(fraction(e) * model.st.current.length_ms);
  };
  bar.addEventListener("pointerup", (e) => end(e, true));
  bar.addEventListener("pointercancel", (e) => end(e, false));
  bar.addEventListener("keydown", (e) => {
    const d = { ArrowRight: 5000, ArrowUp: 5000, ArrowLeft: -5000, ArrowDown: -5000 }[e.key];
    if (!d || !seekable()) return;
    e.preventDefault();
    e.stopPropagation();
    model.seek(Math.max(0, Math.min(model.st.current.length_ms, model.position() + d)));
  });
  return ctl;
}
