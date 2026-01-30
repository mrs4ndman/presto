# presto

A simple music player TUI written in Rust.

## Features
- Directory scan of common audio files (`mp3`, `flac`, `wav`, `ogg`)
- Keyboard-driven TUI
- `/` filter (type-to-filter track titles)
- MPRIS integration for `playerctl` / media keys

## Getting started
- Build: `cargo build`
- Run: `cargo run -- [music_dir]`
	- If `music_dir` is omitted, it defaults to `music`

## Docs
Start with [docs/README.md](docs/README.md).

## TODO:
- [ ] Add a proper “Now Playing” line + elapsed time (requires tracking start time / sink position).
- [ ] Emit MPRIS PropertiesChanged so status/metadata updates push instantly.
- [ ] Improve filtering to fuzzy-match + highlight matches in the list.
- [ ] Full `playerctl` interface compliance
- [ ] Implement test coverage
- [ ] Bug: desynchronization between current-playing and highlighted song
  (next/prev should relocate the highlighted line)
