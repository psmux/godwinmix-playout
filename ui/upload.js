// Upload files from the browser picker or from a drop, then add the stored
// paths to the list. Loaded when the first file arrives.

import { toast, errorToast } from "./toast.js";

const KNOWN = /\.(mp4|mov|m4v|mkv|webm|avi|ts|mpg|mpeg|flv|wmv|mp3|wav|wave|flac|ogg|oga|opus|m4a|aac|aiff|aif|png|jpe?g|bmp|gif|webp|tiff?)$/i;

/** A unique ASCII name, so an upload never replaces footage the mixer holds. */
function uploadName(file) {
  const ext = (/\.([a-z0-9]{1,8})$/i.exec(file.name) || [])[1];
  const stem = file.name.replace(/\.[^.]*$/, "").normalize("NFKD").replace(/[^a-z0-9]+/gi, "-").replace(/^-+|-+$/g, "").slice(0, 80) || "media";
  const r = [...crypto.getRandomValues(new Uint32Array(2))].map((x) => x.toString(36)).join("");
  return `${stem}-${Date.now().toString(36)}-${r}.${ext}`;
}

/** progress(fraction, text) is called as files go up. Resolves to the stored paths. */
export async function uploadFiles(client, files, progress) {
  const paths = [];
  const list = [...files];
  for (let i = 0; i < list.length; i++) {
    const f = list[i];
    if (!KNOWN.test(f.name)) { toast(`${f.name} is not a video, audio or picture file the mixer takes. Choose a media file.`, { error: true }); continue; }
    try {
      progress((i) / list.length, `Uploading ${f.name} (${i + 1} of ${list.length})`);
      const r = await client.upload(uploadName(f), f, (p) => progress((i + (Number(p) || 0)) / list.length, `Uploading ${f.name} (${i + 1} of ${list.length})`));
      if (!r || typeof r.path !== "string") throw new Error("The mixer did not say where it stored " + f.name + ". Open the media library to find it.");
      paths.push(r.path.replace(/\\/g, "/"));
    } catch (e) { errorToast(e); }
  }
  progress(1, "");
  return paths;
}
