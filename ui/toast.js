// Toasts inside the panel. An error is shown as the core wrote it.

import { h } from "./util.js";

let host = null;

/** A popover sits in the top layer, so a toast is seen above an open dialog. */
function top() {
  try { host.hidePopover(); host.showPopover(); } catch { /* no popover support: it floats as it is */ }
}

export function toast(text, opts = {}) {
  if (!host || !host.isConnected) {
    host = h("div.po-toasts", { role: "status", "aria-live": "polite", popover: "manual" });
    document.body.append(host);
  }
  top();
  const node = h("div.po-toast" + (opts.error ? ".err" : ""), {}, [h("span.grow", { text })]);
  const close = () => { clearTimeout(timer); node.remove(); };
  if (opts.action) {
    node.append(h("button.po-btn", { text: opts.action.label, onclick: () => { close(); opts.action.run(); } }));
  }
  node.append(h("button.po-btn.icon", { text: "\u00d7", "aria-label": "Dismiss", onclick: close }));
  host.append(node);
  const timer = setTimeout(close, opts.ms ?? (opts.error ? 8000 : 4000));
  return close;
}

/** Errors from client.call are {code, message, data}; show the message as it came. */
export function errorToast(e) {
  console.warn("playout call failed", e);
  toast(e && e.message ? e.message : String(e), { error: true });
}
