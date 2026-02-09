# presto

A simple music player TUI written in Rust, with Vim-like controls.

## Features
- Directory scan of common audio files (`mp3`, `flac`, `wav`, `ogg`)
- Keyboard-driven TUI with Vim-like controls
- `/` filter (type-to-filter track titles)
- `Ctrl+e` exits filter input without starting playback
- MPRIS integration for `playerctl` / media keys
- Per-directory state persistence (selection, filter, shuffle, loop, volume, last track)

## Getting started
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

- [ ] Extending config validation in the loader and error handling there
- [ ] Add app-state identifier: FILTER - NAVIGATION - FOLLOWING_PLAYING - etc.

</details>

### Long-term
<details>
<summary>List of items to tackle</summary>

- [ ] Listening stats (amount, usage, recent songs, etc.)
- [ ] Theming
- [ ] Cross-platform compatibility

</details>

## ↓ Current status ↓

> [!CAUTION]
> VERY LOUD, will try to record it next time with less volume

https://github.com/user-attachments/assets/34407dda-7599-4ec2-a0af-66889ef6251a
