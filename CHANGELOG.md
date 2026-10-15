# Changelog

## 1.0.0

* A playlist of video, sound and picture files as one GodwinMix source, played
  back to back with no black frame, no gap in the sound and no jump in the
  timestamps, measured off the mixer's programme.
* Loop the list or one item, shuffle, cue, play now, skip, previous, pause,
  seek, in and out points, picture times and a hold on an item's last frame.
* Mixed sizes, frame rates and codecs come out at the show's canvas, fitted,
  filled or stretched.
* Video decoded by the platform's own decoders, and openh264, dav1d, libvpx
  and libde265; everything else in Rust. No FFmpeg.
* A panel in the mixer: now playing with time left in the item and the list,
  next up, thumbnails, drag to reorder, the media library, upload and drop,
  keyboard shortcuts, light and dark themes, phone layout.
* Tools for the CLI, MCP and agents: state, play, pause, next, previous,
  play_item, cue, seek and thumbnail.
