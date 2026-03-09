# MPRIS

Presto exposes an MPRIS service on the user session bus.

## Service identity

- Bus name: `org.mpris.MediaPlayer2.presto`
- Object path: `/org/mpris/MediaPlayer2`
- Interfaces:
  - `org.mpris.MediaPlayer2`
  - `org.mpris.MediaPlayer2.Player`

## Supported commands

Mapped to internal `ControlCmd`:

- `Play`
- `Pause`
- `PlayPause`
- `Stop`
- `Next`
- `Previous`
- `Quit`

## Player properties

- `PlaybackStatus`: `Stopped|Playing|Paused`
- `Metadata`:
  - `mpris:trackid`
  - `xesam:title`
  - `xesam:artist`
  - `xesam:album`
  - `xesam:url`
  - `mpris:length` (microseconds)
- `CanControl`, `CanPlay`, `CanPause`, `CanGoNext`, `CanGoPrevious` are `true`

## Quick verification

```sh
playerctl -l
playerctl -p presto status
playerctl -p presto metadata
playerctl -p presto play-pause
```

## Notes

- Metadata is pushed by runtime sync (`src/runtime/mpris_sync.rs`).
- If session bus is unavailable, app continues without MPRIS and logs an error to stderr.
