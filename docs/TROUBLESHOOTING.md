# Troubleshooting

This page is a short list of common failure modes and what to check.

## Presto starts but finds no tracks

- Confirm you started it in the directory you expect.
  - If you run `cargo run` with no args, Presto scans the current working directory.
- Confirm your files use a supported extension (see `src/library.rs`).
- Confirm the directory is readable by your user.

## Audio output: “No audio output device”

Presto uses rodio to open the default output stream.

- Ensure your system has an audio device and your session can access it.
- On headless sessions, containers, or SSH shells you may not have a usable audio device.
- If your system uses PipeWire, ensure the PipeWire/Pulse compatibility layer is running.

## Scrubbing/seek feels inaccurate

Scrubbing is implemented by recreating the sink and skipping forward in the decoder stream.

- Some codecs/containers do not support precise seeking.
- Large seeks may be slow because the decoder still has to process data.
- If you need frame-accurate seeking, this design will need to change (likely requiring a different decoding strategy).

## MPRIS: `playerctl` can’t see Presto

- Confirm you are on a desktop session with a user session D-Bus.
- Run `playerctl -l` while Presto is running.
- If you’re on a TTY or remote shell without the user bus, MPRIS won’t be available.

## Media keys don’t work

- First verify MPRIS works via `playerctl -p presto play-pause`.
- If `playerctl` works but media keys don’t, your desktop environment may not be configured to route keys through MPRIS.

## UI layout looks wrong or the popup is clipped

- Use a larger terminal size; the UI has fixed sections.
- The metadata popup is intentionally small and centered within the track list area.

## If you’re filing a bug

Please include:

- OS + audio stack (ALSA/PulseAudio/PipeWire)
- Terminal emulator and whether you use tmux/screen
- A copy/paste of the error output (if any)
- Whether the issue reproduces outside of `cargo run` (i.e., with the built binary)
