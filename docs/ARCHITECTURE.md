# Architecture

This is a small TUI music player with three cooperating pieces:

## 1) Library scan
- File: `src/library.rs`
- Responsibility: walk a directory and return a sorted `Vec<Track>`.

## 2) App state (UI model)
- File: `src/app.rs`
- Responsibility: holds the list of tracks and selection state.

Key state:
- `selected`: index into `tracks`
- `playback`: `Stopped | Playing | Paused`
- `filter_mode` + `filter_query`: enables `/` filtering

The UI renders a *filtered view* of `tracks`:
- `App::filtered_indices()` returns a list of track indices that match the query.
- `next/prev` move within that filtered index list, then update `selected`.

## 3) Audio thread
- File: `src/audio.rs`
- Responsibility: keep an audio output stream alive and react to `AudioCmd` messages.

Why a thread?
- Audio playback should not block the TUI event loop.
- The TUI sends commands using `AudioPlayer::send(AudioCmd::...)`.

## Program flow
- File: `src/main.rs`
- Starts:
  - scan library
  - spawn audio thread
  - spawn MPRIS D-Bus service
  - run the TUI loop (polls keyboard + receives MPRIS control commands)

The main loop is the “traffic cop”: it translates inputs (keys + MPRIS) into `AudioCmd` and updates `App` state.
