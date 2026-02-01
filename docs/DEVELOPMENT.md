# Development

This page is for contributors working on Presto locally.

## Prerequisites

- Rust toolchain (stable)
- ALSA/PulseAudio/PipeWire audio output (depending on your Linux setup)
- A working user session D-Bus if you want to exercise MPRIS

## Common commands

Build:

```sh
cargo build
```

Run (scan current directory):

```sh
cargo run
```

Run with a directory:

```sh
cargo run -- /path/to/music
```

Run tests:

```sh
cargo test
```

Format:

```sh
cargo fmt
```

Lint:

```sh
cargo clippy
```

## Where to start in the code

- `src/main.rs`: event loop and glue code
- `src/app.rs`: application state and navigation/filtering logic
- `src/ui.rs`: ratatui layout and rendering
- `src/audio.rs`: audio thread, queue semantics, crossfade/seek behavior
- `src/mpris.rs`: MPRIS D-Bus interface implementation

## Debugging tips

### Audio issues

- If you get `ERR: No audio output device`, rodio could not open a default output device.
  - Check that an output device exists and your user can access it.
  - If you run in a container/SSH session, you may not have an audio device.

### MPRIS issues

- If Presto runs but `playerctl -l` does not show it:
  - You are likely missing a user session bus (common on headless shells).

### TUI rendering/input

- If input feels unresponsive, make sure you’re not in a terminal multiplexer mode that eats key chords.
- Run with a larger terminal size when debugging layout problems; the UI intentionally uses fixed vertical sections.

## Design constraints (practical)

- Presto keeps the audio thread as the owner of the rodio sink.
- The UI thread is intentionally dumb: it renders from `App` and sends commands.
- Seeking is implemented via sink rebuild + decoder skipping, which is simple and works widely, but isn’t perfect for all codecs.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the deeper explanation.
