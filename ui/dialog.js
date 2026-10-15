// A modal built on <dialog>, which brings focus trapping, Escape and a
// backdrop. The panel's theme variables reach it because it sits in the page.

import { h } from "./util.js";
import { icon } from "./icons.js";

/** Returns {dlg, body, foot, close}. The dialog removes itself when it closes. */
export function openDialog(title, { wide = false, onClose } = {}) {
  const body = h("div.po-dlg-body");
  const foot = h("div.po-dlg-foot");
  const dlg = h("dialog.po-dlg" + (wide ? ".wide" : ""), { "aria-label": title }, [
    h("header.po-dlg-head", {}, [
      h("strong.grow", { text: title }),
      h("button.po-btn.icon", { "aria-label": "Close", onclick: () => dlg.close() }, [icon("close", 16)]),
    ]),
    body,
    foot,
  ]);
  dlg.addEventListener("close", () => { dlg.remove(); if (onClose) onClose(); });
  dlg.addEventListener("keydown", (e) => e.stopPropagation());
  dlg.addEventListener("click", (e) => { if (e.target === dlg) dlg.close(); });
  document.body.append(dlg);
  dlg.showModal();
  return { dlg, body, foot, close: () => dlg.close() };
}
