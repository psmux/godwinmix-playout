// The item editor: title, in and out points, picture time and hold.
// Loaded the first time someone opens it.

import { h, fmt, parseTime } from "./util.js";
import { openDialog } from "./dialog.js";
import { itemLength } from "./row.js";

function field(label, input, hint) {
  return h("label.po-field", {}, [h("span", { text: label }), input, hint ? h("small.faint", { text: hint }) : null]);
}

const secs = (ms) => (ms ? String(ms / 1000) : "");

export function openEditor(model, id) {
  const it = model.item(id);
  if (!it) return;
  const { body, foot, close } = openDialog("Edit item");
  const picture = it.kind === "picture";
  const title = h("input", { type: "text", value: it.title || "", placeholder: it.path });
  const inp = h("input", { type: "text", inputmode: "decimal", value: it.in_ms ? fmt(it.in_ms, true) : "", placeholder: "0:00" });
  const out = h("input", { type: "text", inputmode: "decimal", value: it.out_ms ? fmt(it.out_ms, true) : "", placeholder: it.file_ms ? fmt(it.file_ms, true) : "end of file" });
  const still = h("input", { type: "number", min: "0", step: "0.1", value: secs(it.still_ms), placeholder: "list default" });
  const hold = h("input", { type: "checkbox" });
  hold.checked = !!it.hold;
  const msg = h("p.po-formerr", { role: "alert", hidden: true });

  const len = itemLength(it);
  body.append(...[
    h("p.faint.sm.ellipsis", { text: it.path, title: it.path }),
    field("Title", title),
    kindRow(),
    picture || it.kind === null ? field("Picture time in seconds", still, "How long a picture stays on screen.") : null,
    h("label.po-check", {}, [hold, h("span", { text: "Hold on the last frame and wait for Play" })]),
    h("p.faint.sm", { text: len != null ? `Plays for ${fmt(len, true)} as set.` : "The length is not known yet." }),
    msg,
  ].filter(Boolean));

  function kindRow() {
    if (picture) return null;
    return h("div.po-two", {}, [
      field("In point", inp, "Start here. Minutes and seconds, such as 1:05.5"),
      field("Out point", out, "Stop here. Empty plays to the end."),
    ]);
  }

  const save = () => {
    const a = parseTime(inp.value), b = parseTime(out.value);
    const s = still.value === "" ? 0 : Math.round(Number(still.value) * 1000);
    let err = "";
    if (Number.isNaN(a)) err = "The in point is not a time. Write it like 1:05.5 or 65.5.";
    else if (Number.isNaN(b)) err = "The out point is not a time. Write it like 1:05.5 or 65.5.";
    else if (b && b <= a) err = "The out point has to be after the in point. Leave it empty to play to the end.";
    else if (!(s >= 0)) err = "The picture time has to be zero or more seconds.";
    if (err) { msg.textContent = err; msg.hidden = false; return; }
    close();
    model.update(id, { title: title.value.trim() || it.title, in_ms: a, out_ms: b, still_ms: s, hold: hold.checked });
  };
  foot.append(
    h("button.po-btn", { text: "Cancel", onclick: close }),
    h("button.po-btn.primary", { text: "Save", onclick: save }),
  );
  body.addEventListener("keydown", (e) => { if (e.key === "Enter" && e.target.tagName === "INPUT" && e.target.type !== "checkbox") { e.preventDefault(); save(); } });
  title.focus();
}
