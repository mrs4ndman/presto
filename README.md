# presto

A simple music player TUI written in Rust, with Vim-like controls.

## Features
- Directory scan of common audio files (`mp3`, `flac`, `wav`, `ogg`)
- Keyboard-driven TUI with Vim-like controls
- `/` filter with word-by-word fuzzy matching
- `Ctrl+e` exits filter input without starting playback
- Right-sid panes for metadata and embedded lyrics (opened up with `K` / `gl`)
- Opt-in lyrics loading via config, with timed-line emphasis for synced lyrics
- MPRIS integration for `playerctl` / media keys
- Per-directory state persistence (selection, filter, shuffle, loop, volume, last track)
- Number-driven movement for `hjkl` skipping / navigation

## Getting started

- Dependencies:
  - [`rodio` requirements](https://github.com/RustAudio/rodio#requirements)
  - `libasound2-dev` (on Ubuntu-based at least)

### `crates.io`
- The version uploaded to `crates.io` is the one on the develop branch:
```bash
cargo install presto
```

### From source
- Build: `cargo build`
- Run: `cargo run -- [music_dir]`
	- If `music_dir` is omitted, it defaults to the current directory

## Docs
Visit the [web version](https://presto.mrs4ndman.dev) or start with
[docs/README.md](docs/README.md) 

## TODO:
Finished TODOs will migrate onto the [changelog](CHANGELOG.md).

### Short-term 
<details>
<summary>List of items to tackle</summary>

- [ ] Enabling re-ordering / disabling some status sections

</details>

### Long-term
<details>
<summary>List of items to tackle / consider</summary>

- [ ] Listening stats (amount, usage, recent songs, etc.)
- [ ] Theming
- [ ] Cross-platform compatibility (config/state paths, media controls, audio backend support)
- [ ] Server-client restructuring

</details>

## ↓ Current status ↓

> [!CAUTION]
> VERY LOUD, will try to record it next time with less volume

https://github.com/user-attachments/assets/34407dda-7599-4ec2-a0af-66889ef6251a
