// The shortcut list, as a small dialog. Loaded when asked for.

import { h } from "./util.js";
import { openDialog } from "./dialog.js";
import { SHORTCUTS } from "./keys.js";

export function openHelp() {
  const { body, foot, close } = openDialog("Keyboard shortcuts");
  body.append(
    h("p.faint.sm", { text: "These work while the panel has focus. Click the panel first." }),
    h("dl.po-keys", {}, SHORTCUTS.flatMap(([k, d]) => [
      h("dt", {}, k.split(/ or |, /).flatMap((part, i) => [i ? h("span.faint", { text: " " }) : null, h("kbd", { text: part })])),
      h("dd", { text: d }),
    ])),
  );
  foot.append(h("button.po-btn.primary", { text: "Done", onclick: close }));
}
