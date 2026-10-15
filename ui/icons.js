// Inline icons, 24 by 24, filled with the text colour.

const P = {
  play: "M8 5v14l11-7z",
  pause: "M6 5h4v14H6zM14 5h4v14h-4z",
  next: "M6 6l9 6-9 6zM16 6h2v12h-2z",
  prev: "M18 6l-9 6 9 6zM6 6h2v12H6z",
  loop: "M7 7h10v3l4-4-4-4v3H5v6h2zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2z",
  shuffle: "M17 3l4 4-4 4V8h-2.6l-3.1 4.3L8.6 16H3v-2h4.6l2.7-3.7L7.6 7H3V5h5.4l2.9 4 2.9-4H17zM3 18h4.6l1.3-1.8 1.4 2L8.6 21H3zm14-3l4 4-4 4v-3h-2.6l-2.2-3 1.2-1.7 1.5 2H17z",
  video: "M4 6h11a1 1 0 011 1v3.5l4-2.5v8l-4-2.5V17a1 1 0 01-1 1H4a1 1 0 01-1-1V7a1 1 0 011-1z",
  audio: "M9 4v10.3A3.5 3.5 0 107 17.5V8h8V4z",
  picture: "M4 5h16a1 1 0 011 1v12a1 1 0 01-1 1H4a1 1 0 01-1-1V6a1 1 0 011-1zm2 11h12l-3.5-5-3 4-2-2.5z",
  warn: "M12 3l10 18H2zm-1 7v5h2v-5zm0 6v2h2v-2z",
  edit: "M4 17.3V20h2.7L17.8 8.9l-2.7-2.7zM19.7 7a1 1 0 000-1.4l-1.3-1.3a1 1 0 00-1.4 0l-1.1 1.1 2.7 2.7z",
  trash: "M6 7h12l-1 13H7zM9 4h6l1 2H8z",
  cue: "M5 4h14v2H5zm0 5h14v2H5zm0 5h8v2H5zm11 0l5 3-5 3z",
  plus: "M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6z",
  upload: "M11 16V8l-3 3-1.4-1.4L12 5l5.4 4.6L16 11l-3-3v8zM5 18h14v2H5z",
  library: "M4 6h16v2H4zm0 5h16v2H4zm0 5h16v2H4z",
  grip: "M9 5a1.5 1.5 0 110 3 1.5 1.5 0 010-3zm6 0a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM9 10.5a1.5 1.5 0 110 3 1.5 1.5 0 010-3zm6 0a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM9 16a1.5 1.5 0 110 3 1.5 1.5 0 010-3zm6 0a1.5 1.5 0 110 3 1.5 1.5 0 010-3z",
  help: "M12 3a9 9 0 100 18 9 9 0 000-18zm-1 13v-2h2v2zm1-10a3 3 0 011.9 5.3c-.6.5-.9.8-.9 1.7h-2c0-1.5.6-2.1 1.3-2.7A1.1 1.1 0 0012 8a1 1 0 00-1 1H9a3 3 0 013-3z",
  gear: "M12 8.5a3.5 3.5 0 100 7 3.5 3.5 0 000-7zM10.6 2h2.8l.4 2.5 1.5.6 2.1-1.5 2 2-1.5 2.1.6 1.5 2.5.4v2.8l-2.5.4-.6 1.5 1.5 2.1-2 2-2.1-1.5-1.5.6-.4 2.5h-2.8l-.4-2.5-1.5-.6-2.1 1.5-2-2 1.5-2.1-.6-1.5L2 13.4v-2.8l2.5-.4.6-1.5-1.5-2.1 2-2 2.1 1.5 1.5-.6z",
  close: "M6.4 5L12 10.6 17.6 5 19 6.4 13.4 12 19 17.6 17.6 19 12 13.4 6.4 19 5 17.6 10.6 12 5 6.4z",
  search: "M10 4a6 6 0 014.7 9.7l5 5-1.4 1.4-5-5A6 6 0 1110 4zm0 2a4 4 0 100 8 4 4 0 000-8z",
  undo: "M12 5a8 8 0 017.5 5.3l-1.9.7A6 6 0 0012 7c-2 0-3.700 1-4.800 2.500L10 12H3V5l2.700 2.700A8 8 0 0112 5z",
};

export function icon(name, size = 18) {
  const span = document.createElement("span");
  span.className = "ico";
  span.innerHTML = `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="currentColor" aria-hidden="true"><path d="${P[name] || ""}"/></svg>`;
  return span;
}

export function kindIcon(kind) {
  return icon(kind === "audio" ? "audio" : kind === "picture" ? "picture" : "video", 16);
}
