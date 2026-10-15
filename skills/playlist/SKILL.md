---
name: playout-playlist
description: Run a playout playlist source in GodwinMix, a list of video, sound and picture files that plays back to back as one source with no gap. Use when asked to make a playlist, add or remove or reorder files in one, loop or shuffle it, skip, cue the next item, seek, pause, or say what is playing and how long is left. Edit the list with source.set; drive playback with the playout tools (state, play, pause, next, previous, play_item, cue, seek), passing the source id.
---

# Playout playlists

A playlist is one source of type `playout/playlist`. Its params are the list,
so the project file saves it, and several playlists are several sources.

## Make one

```
source.add {"id": "lobby", "uri": "playout/playlist", "type": "playout/playlist", "name": "Lobby loop",
            "params": {"items": [{"id": "intro", "path": "/media/intro.mp4"},
                                 {"id": "logo", "path": "/media/logo.png", "still_ms": 5000}],
                       "loop": "all"}}
```

Put it on a scene like any other source. Paths are on the machine the mixer
runs on; `media.list` gives the paths of files in the media library.

## Change the list

`source.set` with `params.items` replaces the whole list, in the new order.
Read it first with `source.get`, change it, send it back. The item playing
keeps playing if its `id` is still in the list; give every item an `id` and
keep it when you move items.

```
source.set {"id": "lobby", "params": {"items": [...], "shuffle": true}}
```

Other params: `loop` (`all`, `one`, `off`), `shuffle`, `at_end` (`black` or
`hold`, with loop off), `still_seconds`, `fit` (`fit`, `fill`, `stretch`),
`autoplay`. Per item: `in_ms`, `out_ms`, `still_ms`, `hold` (stop on its
last frame until play), `title`.

## Drive it

Every tool takes `source`, the playlist's source id.

| Tool | Does |
|---|---|
| `state` | what is on, position, time left in the item and to the end of the list, next, every item with its length and any error |
| `play`, `pause` | resume, continue after a hold, restart a list that ended; pause on the picture |
| `next`, `previous` | skip now; back to the start, or the item before in the first three seconds |
| `play_item {id}` | play that item now |
| `cue {id}` | play that item next, once; `{"id": ""}` clears |
| `seek {position_ms}` | move in the current item |

Call them with `tool.call {"name": "playout/next", "source": "lobby"}`, or
as MCP tools, or `gmx tool call playout/next '{"source": "lobby"}'`.

## What to expect

* An item that is missing or will not decode is skipped and named in
  `state.items[].error`; the list plays on.
* Changing items is instant on air: the next item is opened five seconds
  before the current one ends, and a skip waits for the next item's first
  picture rather than showing black.
* Video is decoded by the platform's decoders (DXVA on Windows,
  VideoToolbox on macOS, VA or V4L2 on Linux) or openh264, dav1d, libvpx and
  libde265. There is no FFmpeg. A codec with no decoder here says so.
