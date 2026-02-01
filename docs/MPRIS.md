
# MPRIS

Presto exposes an MPRIS player on the user session bus. This enables desktop media keys and tools like `playerctl`.

## What you get

- Play/pause, next, previous, stop via MPRIS clients
- Metadata updates when tracks change
- Status updates (Playing/Paused/Stopped)

## Bus details

- Bus: user session bus
- Bus name: `org.mpris.MediaPlayer2.presto`
- Object path: `/org/mpris/MediaPlayer2`
- Interfaces: `org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player`

## Quick test

Run Presto, then in another terminal:

```sh
playerctl -l
playerctl -p presto status
playerctl -p presto metadata
playerctl -p presto play-pause
playerctl -p presto next
playerctl -p presto previous
playerctl -p presto stop
```

## Metadata fields

Presto attempts to populate the common xesam/mpris fields when available:

- `xesam:title`
- `xesam:artist`
- `xesam:album`
- `xesam:url`
- `mpris:length` (microseconds)
- `mpris:trackid`

Not all audio files have tags; in those cases you’ll see fewer fields.

## Troubleshooting

If `playerctl -l` does not show `presto`:

- Confirm you are on a graphical session with a user session bus.
- Check that `dbus` is running for your user.
- Run `RUST_LOG=zbus=debug` (or similar) if you add logging; Presto currently keeps stderr output minimal.

If the player shows up but metadata doesn’t update:

- Some clients cache metadata; try `playerctl -p presto metadata` repeatedly during track changes.
- Confirm that Presto is emitting `PropertiesChanged` on track transitions (see `src/mpris.rs`).

If media keys don’t work:

- Verify your DE routes media keys through an MPRIS-capable mediator.
- Test using `playerctl` first; if `playerctl` works but keys don’t, it’s a desktop config issue.

For broader runtime issues, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
