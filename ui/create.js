// Make a new playlist, and the options for one. Loaded when asked for.

import { h, slugify, uniqueSlug } from "./util.js";
import { openDialog } from "./dialog.js";
import { errorToast } from "./toast.js";

export function openCreate(client, taken, onMade) {
  const { body, foot, close } = openDialog("New playlist");
  const name = h("input", { type: "text", placeholder: "Lobby loop", "aria-label": "Playlist name", maxlength: "60" });
  const make = async () => {
    const label = name.value.trim();
    if (!label) { name.focus(); return; }
    const id = uniqueSlug(slugify(label), taken);
    try {
      await client.call("source.add", { id, name: label, uri: "playout/playlist", type: "playout/playlist", params: { items: [] } });
      close();
      onMade(id);
    } catch (e) { errorToast(e); }
  };
  body.append(h("label.po-field", {}, [h("span", { text: "Name" }), name, h("small.faint", { text: "It becomes a source you can put in a scene." })]));
  name.addEventListener("keydown", (e) => { if (e.key === "Enter") make(); });
  foot.append(h("button.po-btn", { text: "Cancel", onclick: close }), h("button.po-btn.primary", { text: "Create playlist", onclick: make }));
  name.focus();
}

function select(label, value, choices, onPick) {
  const sel = h("select", {}, choices.map(([v, t]) => h("option", { value: v, text: t })));
  sel.value = value;
  sel.addEventListener("change", () => onPick(sel.value));
  return h("label.po-field", {}, [h("span", { text: label }), sel]);
}

function seconds(value, onPick) {
  const input = h("input", { type: "number", min: "0.5", max: "86400", step: "0.5", value: String(value) });
  input.addEventListener("change", () => { const v = Number(input.value); if (v >= 0.5) onPick(v); });
  return h("label.po-field", {}, [h("span", { text: "Picture time, seconds" }), input, h("small.faint", { text: "How long a picture stays up when its item does not say." })]);
}

export function openOptions(model) {
  const { body, foot, close } = openDialog("Playlist options");
  const st = model.st || {};
  body.append(
    select("When the list ends", st.at_end || "black", [["black", "Go to black"], ["hold", "Hold the last picture"]], (v) => model.setOption("at_end", v)),
    select("Fit pictures and video", st.fit || "fit", [["fit", "Fit inside the frame"], ["fill", "Fill the frame and crop"], ["stretch", "Stretch to the frame"]], (v) => model.setOption("fit", v)),
    seconds(st.still_seconds ?? 10, (v) => model.setOption("still_seconds", v)),
    select("When the source starts", st.autoplay === false ? "wait" : "play", [["play", "Start playing"], ["wait", "Wait for play"]], (v) => model.setOption("autoplay", v === "play")),
  );
  foot.append(h("button.po-btn.primary", { text: "Done", onclick: close }));
}
