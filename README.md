# presto

A simple music player TUI written in Rust, with Vim-like controls.

## Features
- Directory scan of common audio files (`mp3`, `flac`, `wav`, `ogg`)
- Keyboard-driven TUI with Vim-like controls
- `/` filter (type-to-filter track titles)
- MPRIS integration for `playerctl` / media keys

## Getting started
- Build: `cargo build`
- Run: `cargo run -- [music_dir]`
	- If `music_dir` is omitted, it defaults to the current directory

## Docs
Start with [docs/README.md](docs/README.md).

## TODO:

### Short-term 
<details>
<summary>List of covered items</summary>

- [x] Add a proper “Now Playing” line + elapsed time (requires tracking start time / sink position)
- [x] Emit MPRIS PropertiesChanged so status/metadata updates push instantly
- [x] Improve filtering to fuzzy-match + highlight matches in the list
- [x] Implement test coverage
- [x] Bug: desynchronization between current-playing and highlighted song
  (next/prev should relocate the highlighted line)
- [x] Add `gg`/`G` to jump to top/bottom of track list
- [x] Wrap status text in the UI
- [x] Reorganize keybinds to be more vim-like
- [x] Song scrubbing (FF/FB) with `H` / `L`
- [x] Fix: When hitting play-selected on currently playing song, do nothing
- [x] Fix: When no arguments are given start up on current dir
- [x] Metadata window (`K`)
- [x] Fix: filter mode captures keypresses (you can type `j` / `k`)
- [ ] Pad shown data in the UI on the sides with 1 space
- [ ] Bug: `gg` / `G` only works after moving around with cursor

</details>

### Long-term
<details>
<summary>List of covered items</summary>
- [x] Full `playerctl` + MPRIS interface compliance
- [x] Extract full metadata for songs that have it available
- [x] Config file -> Custom rebinding of controls, theming, etc. (Mid-sized task)
- [x] Crossfade + exit delay (need to make it configurable)
- [ ] Listening stats (amount, usage, recent songs, etc.)
- [ ] Restoring previous state after exiting (per-directory)

</details>

## ↓ Current status (!!WARNING!!: VERY LOUD, I'm sorry :/) ↓

https://github.com/user-attachments/assets/34407dda-7599-4ec2-a0af-66889ef6251a
