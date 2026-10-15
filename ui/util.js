// Small helpers shared by the playout panel: DOM building, time, ids.

/** h("div.a.b", {title: "x", onclick}, [children]) -> element. */
export function h(spec, attrs, kids) {
  const m = /^([a-z0-9-]+)?((?:\.[^.#]+)*)$/i.exec(spec);
  const node = document.createElement(m && m[1] ? m[1] : "div");
  if (m && m[2]) node.className = m[2].split(".").filter(Boolean).join(" ");
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === null || v === undefined || v === false) continue;
    if (k === "text") node.textContent = String(v);
    else if (k === "html") node.innerHTML = v;
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else node.setAttribute(k, v === true ? "" : String(v));
  }
  for (const c of [].concat(kids ?? [])) {
    if (c !== null && c !== undefined && c !== false) node.append(c);
  }
  return node;
}

/** 83500 -> "1:23", 3723000 -> "1:02:03". With tenths: "1:23.5". */
export function fmt(ms, tenths = false) {
  if (!Number.isFinite(ms)) return "--:--";
  const total = Math.max(0, ms) / 1000;
  const s = Math.floor(total);
  const sec = String(s % 60).padStart(2, "0");
  const min = Math.floor(s / 60) % 60;
  const hr = Math.floor(s / 3600);
  const base = hr ? `${hr}:${String(min).padStart(2, "0")}:${sec}` : `${min}:${sec}`;
  return tenths ? `${base}.${Math.floor((total - s) * 10)}` : base;
}

/** "1:23.5", "83.5", "1:02:03" -> ms. Empty -> 0. Anything else -> NaN. */
export function parseTime(text) {
  const t = String(text ?? "").trim();
  if (!t) return 0;
  if (!/^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(t)) return NaN;
  const parts = t.split(":").map(Number);
  let secs = 0;
  for (const p of parts) secs = secs * 60 + p;
  return Math.round(secs * 1000);
}

/** The clock time `ms` from now, as the person's locale shows it. */
export function clockAt(ms) {
  return new Date(Date.now() + ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function basename(path) {
  return String(path || "").split(/[\/]/).pop();
}

export function slugify(text) {
  const s = String(text).toLowerCase().normalize("NFKD").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return s.slice(0, 40) || "playlist";
}

export function uniqueSlug(base, taken) {
  let id = base, n = 2;
  while (taken.has(id)) id = `${base}-${n++}`;
  return id;
}

export function newItemId(taken) {
  let id;
  do id = "k" + Math.random().toString(36).slice(2, 7); while (taken.has(id));
  return id;
}

export function debounce(fn, ms) {
  let t = 0;
  return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
}
