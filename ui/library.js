// Pick files from the mixer's media library and add them to the list.
// Loaded the first time someone opens it. It reads `media.list` when it opens
// and again as the search changes, and not otherwise.

import { h, fmt, debounce } from "./util.js";
import { icon, kindIcon } from "./icons.js";
import { openDialog } from "./dialog.js";
import { errorToast } from "./toast.js";

const PICTURE = /\.(png|jpe?g|bmp|gif|webp|tiff?)$/i;

function kindOf(m) {
  if (PICTURE.test(m.name)) return "picture";
  return m.has_video ? "video" : "audio";
}

function size(bytes) {
  if (bytes > 1e9) return (bytes / 1e9).toFixed(1) + " GB";
  if (bytes > 1e6) return Math.round(bytes / 1e6) + " MB";
  return Math.max(1, Math.round(bytes / 1e3)) + " KB";
}

export function openLibrary(client, model) {
  const { body, foot, close } = openDialog("Media library", { wide: true });
  const search = h("input.po-search", { type: "search", placeholder: "Search the library", "aria-label": "Search the library" });
  const chips = h("div.po-chips");
  const list = h("ul.po-lib", { role: "listbox", "aria-multiselectable": "true", "aria-label": "Media files" });
  const status = h("p.faint.sm", { role: "status" });
  const add = h("button.po-btn.primary", { text: "Add", disabled: true });
  const picked = new Map();
  let folder = "", folders = [], seq = 0;

  const refreshButton = () => {
    add.disabled = picked.size === 0;
    add.textContent = picked.size ? `Add ${picked.size} to the list` : "Add";
  };

  const row = (m) => {
    const box = h("input", { type: "checkbox", tabindex: "-1", "aria-hidden": "true" });
    const kind = kindOf(m);
    const li = h("li.po-libitem", { role: "option", tabindex: "0", "aria-selected": "false" }, [
      box, kindIcon(kind),
      h("div.po-rtext", {}, [
        h("div.po-rtitle", { text: m.name, title: m.path }),
        h("div.po-rmeta", { text: [m.video_codec, m.width ? `${m.width}x${m.height}` : null, m.audio_codec && !m.has_video ? m.audio_codec : null, size(m.size_bytes)].filter(Boolean).join(" · ") }),
      ]),
      h("span.po-len.num", { text: m.duration_ms ? fmt(m.duration_ms) : "" }),
    ]);
    const toggle = () => {
      const on = !picked.has(m.path);
      if (on) picked.set(m.path, m); else picked.delete(m.path);
      box.checked = on;
      li.setAttribute("aria-selected", String(on));
      li.classList.toggle("sel", on);
      refreshButton();
    };
    li.addEventListener("click", toggle);
    li.addEventListener("keydown", (e) => { if (e.key === " " || e.key === "Enter") { e.preventDefault(); toggle(); } });
    li.addEventListener("dblclick", () => { if (!picked.has(m.path)) toggle(); commit(); });
    if (picked.has(m.path)) { box.checked = true; li.classList.add("sel"); li.setAttribute("aria-selected", "true"); }
    return li;
  };

  const drawChips = () => {
    const all = ["", ...folders];
    chips.replaceChildren(...(folders.length ? all.map((f) => h("button.po-chip2" + (f === folder ? ".on" : ""), {
      text: f || "All", onclick: () => { folder = f; load(); },
    })) : []));
  };

  async function load() {
    const mine = ++seq;
    status.textContent = "Loading";
    try {
      const params = { query: search.value.trim(), subfolders: true };
      if (folder) params.folder = folder;
      const r = await client.call("media.list", params);
      if (mine !== seq) return;
      folders = r.folders || folders;
      drawChips();
      list.replaceChildren(...(r.items || []).map(row));
      status.textContent = r.error ? r.error : (r.items || []).length ? `${r.items.length} files` : search.value ? "Nothing matches. Try fewer words." : "The library is empty. Upload a file to start it.";
    } catch (e) { if (mine === seq) { status.textContent = ""; errorToast(e); } }
  }

  function commit() {
    if (!picked.size) return;
    const paths = [...picked.values()].map((m) => m.path.replace(/\\/g, "/"));
    close();
    model.addPaths(paths);
  }

  search.addEventListener("input", debounce(load, 200));
  add.addEventListener("click", commit);
  body.append(h("div.po-lib-top", {}, [icon("search", 16), search]), chips, list, status);
  foot.append(h("button.po-btn", { text: "Cancel", onclick: close }), add);
  load();
  search.focus();
}
