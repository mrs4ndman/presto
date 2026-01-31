# Architecture

This is a small TUI music player with three cooperating pieces:

## 1) Library scan
- File: `src/library.rs`
- Responsibility: walk a directory and return a sorted `Vec<Track>`.

Scan notes:
- Presto attempts to extract tags (title/artist/album) and duration.
- A `Track.display` string is precomputed (used for the UI list + filtering).

## 2) App state (UI model)
- File: `src/app.rs`
- Responsibility: holds the list of tracks and selection state.

Key state:
- `selected`: index into `tracks`
- `playback`: `Stopped | Playing | Paused`
- `filter_mode` + `filter_query`: enables `/` filtering
- `follow_playback`: whether the cursor follows the playing track
- `loop_mode`: `NoLoop | LoopAll | LoopOne`
- `shuffle`: whether shuffle is enabled

The UI renders a *view* of `tracks`:
- `App::display_indices()` returns the current visible ordering (shuffle + filter).
- Navigation (`j/k`) moves inside that visible view.

Cursor behavior:
- When `follow_playback` is on, the selected row follows the currently playing track.
- When you move the cursor manually, follow is disabled (“free-roam”).
- External MPRIS controls (media keys / `playerctl`) re-enable follow when not filtering.

## 3) Audio thread
- File: `src/audio.rs`
- Responsibility: keep an audio output stream alive and react to `AudioCmd` messages.

Playback queue:
- The audio thread maintains a `queue` and `queue_pos` which drive next/prev and auto-advance.
- The UI syncs the queue to match the current visible list (filtered/unfiltered + shuffle).
- Shuffle affects both the view and the actual playback queue.

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
