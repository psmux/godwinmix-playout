// Getting files into the list: the browser picker, and files dropped from the
// desktop onto the panel. A drop from outside the page is the operating
// system's, so the native drop event is right here; moving rows inside the
// page uses pointer events (reorder.js).
//
// The shell also listens for drops on the window. A drop that lands on this
// panel is kept from reaching it, and the shell's own drag counter is put
// back to zero so its overlay does not stay up.

import { h } from "./util.js";
import { icon } from "./icons.js";
import { toast } from "./toast.js";

const hasFiles = (e) => e.dataTransfer && [...(e.dataTransfer.types || [])].includes("Files");

export class Files {
  constructor(root, client, getModel) {
    this.client = client;
    this.getModel = getModel;
    this.busy = false;
    this.input = h("input", { type: "file", multiple: true, hidden: true, accept: "video/*,audio/*,image/*,.mkv,.ts,.mov,.webm,.wav,.flac,.m4a" });
    this.bar = h("progress", { max: "1", value: "0", "aria-label": "Upload progress" });
    this.text = h("span.sm.dim.ellipsis");
    this.status = h("div.po-upload", { hidden: true, role: "status" }, [this.bar, this.text]);
    this.veil = h("div.po-drop", { hidden: true }, [icon("upload", 28), h("strong", { text: "Drop files to add them" }), h("span.sm", { text: "They are uploaded to the mixer, then added at the end of the list." })]);
    this.depth = 0;
    this.winDepth = 0;
    root.append(this.input, this.veil);
    this.input.addEventListener("change", () => { this.add([...this.input.files]); this.input.value = ""; });
    this.root = root;
    const on = (t, n, f, c) => { t.addEventListener(n, f, c); return () => t.removeEventListener(n, f, c); };
    this.offs = [
      on(window, "dragenter", () => this.winDepth++, true),
      on(window, "dragleave", () => { this.winDepth = Math.max(0, this.winDepth - 1); }, true),
      on(root, "dragenter", (e) => { if (hasFiles(e)) { this.depth++; this.veil.hidden = false; } }),
      on(root, "dragover", (e) => { if (hasFiles(e)) { e.preventDefault(); e.dataTransfer.dropEffect = "copy"; } }),
      on(root, "dragleave", () => { this.depth = Math.max(0, this.depth - 1); if (!this.depth) this.veil.hidden = true; }),
      on(root, "drop", (e) => this.drop(e)),
    ];
  }

  destroy() { for (const off of this.offs) off(); }

  browse() { if (!this.busy) this.input.click(); }

  drop(e) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    this.depth = 0;
    this.veil.hidden = true;
    // Settle the shell's counter, which never saw this drop.
    for (let i = 0; i < this.winDepth; i++) window.dispatchEvent(new Event("dragleave"));
    this.winDepth = 0;
    document.body.classList.remove("dropping");
    this.add(e.dataTransfer.files);
  }

  async add(files) {
    if (!files || !files.length) return;
    if (this.busy) { toast("An upload is still running. Drop the files again when it finishes."); return; }
    this.busy = true;
    this.status.hidden = false;
    const m = this.getModel();
    try {
      const { uploadFiles } = await import("./upload.js");
      const paths = await uploadFiles(this.client, files, (f, t) => { this.bar.value = f; this.text.textContent = t; });
      if (paths.length) {
        await m.addPaths(paths);
        toast(paths.length === 1 ? "Added 1 file to the list." : `Added ${paths.length} files to the list.`);
      }
    } finally {
      this.busy = false;
      this.status.hidden = true;
    }
  }
}
