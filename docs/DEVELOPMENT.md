# Development

## Prerequisites

- Rust stable toolchain
- Linux audio stack (ALSA/Pulse/PipeWire)
- User session D-Bus for MPRIS testing

## Common commands

```sh
cargo build
cargo run -- /path/to/music
cargo test
cargo fmt
cargo clippy
```

## Where to start in code

- `src/runtime/event_loop.rs`: keyboard/mpris orchestration
- `src/app/model.rs`: core app state + filtering/navigation
- `src/audio/thread.rs`: playback engine and queue progression
- `src/ui/mod.rs`: render orchestration
- `src/ui/panes.rs`: concrete UI widgets
- `src/config/schema.rs`: settings and defaults
- `src/runtime/state.rs`: persisted per-directory state

## Safe change workflow

1. Change one subsystem at a time.
2. Run `cargo test` after each logical step.
3. If keybindings or behavior changed, update docs:
   - `docs/CONTROLS.md`
   - `docs/CONFIG.md`
   - `docs/TROUBLESHOOTING.md`
4. Run `cargo fmt` before finalizing.

## Debugging tips

### Audio issues

If you see no output device errors, verify local audio access and run outside headless/container sessions first.

### MPRIS issues

Use:

```sh
playerctl -l
playerctl -p presto status
playerctl -p presto metadata
```

If the player is not listed, session D-Bus is usually missing.

### UI behavior issues

- Confirm whether the app is in filter mode vs normal mode.
- Confirm `follow_playback` state when selection appears to jump.
- For list behavior, inspect `display_indices()` and queue-dirty syncing in `event_loop.rs`.
