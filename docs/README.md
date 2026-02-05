
# Presto

Presto is a terminal music player (Rust + ratatui) with keyboard-first navigation and MPRIS support.

This page is the entry point for the web documentation (GitHub Pages). Use it as an index into the rest of the docs.

## Quick start

Build:

```sh
cargo build
```

Run (scan current directory):

```sh
cargo run
```

Run with an explicit music directory:

```sh
cargo run -- /path/to/music
```

## Documentation index

Start here:

- Controls and keys: [CONTROLS.md](CONTROLS.md)
- Configuration: [CONFIG.md](CONFIG.md)
- Troubleshooting common issues: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

Implementation and contributor notes:

- Architecture and code map: [ARCHITECTURE.md](ARCHITECTURE.md)
- Development workflow: [DEVELOPMENT.md](DEVELOPMENT.md)
- MPRIS / `playerctl` integration: [MPRIS.md](MPRIS.md)
- Design notes (assumptions, pitfalls, tradeoffs): [ORGANIZATION.md](ORGANIZATION.md)

## Behavior notes (high signal)

- Startup directory: if you don’t pass `music_dir`, Presto scans the current working directory.
- Seeking/scrubbing: implemented by rebuilding the audio sink and skipping forward in the decoder stream; accuracy varies by codec/container.
- Quit: uses a short fade-out to avoid an abrupt stop.

## If you’re changing the code

If you change user-visible behavior, please update at least one of:

- [CONTROLS.md](CONTROLS.md) for keybinds and UX behavior
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) if it affects common failure modes
- [ARCHITECTURE.md](ARCHITECTURE.md) if it changes program structure or invariants
