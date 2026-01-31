# MPRIS / playerctl integration

Presto exposes an MPRIS player on the **user session bus**:
- Bus name: `org.mpris.MediaPlayer2.presto`
- Object path: `/org/mpris/MediaPlayer2`

This allows desktop media keys (XF86 Play/Pause etc.) and tools like `playerctl` to control playback.

## Quick test
Run presto in one terminal:
- `cargo run`

In another terminal:
- `playerctl -l` (should show `presto`)
- `playerctl -p presto play-pause`
- `playerctl -p presto next`
- `playerctl -p presto previous`
- `playerctl -p presto stop`
- `playerctl -p presto status`

## Implementation overview
- File: `src/mpris.rs`
- Uses `zbus` to:
  - connect to the session bus
  - request the MPRIS name
  - register `org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player` interfaces

Presto emits `PropertiesChanged` signals; `playerctl` should update instantly on:
- track changes (UI actions, media keys, auto-advance)
- play/pause changes

Metadata currently includes:
- `xesam:title`
- `xesam:artist` (when available)
- `xesam:album` (when available)
- `xesam:url` (file path)
- `mpris:length` (microseconds, when duration is available)
- `mpris:trackid`
