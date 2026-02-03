
# Architecture

This document is for contributors and maintainers. It describes the core components, how state flows through the program, and the tradeoffs behind a few important design choices.

## Code map

- `src/main.rs`: application startup + event loop + wiring (UI ↔ audio ↔ MPRIS)
- `src/app.rs`: UI model/state (tracks, selection, filter, cursor mode)
- `src/ui.rs`: drawing/layout (ratatui)
- `src/audio.rs`: audio thread + queue semantics (rodio)
- `src/library.rs`: scanning directories + extracting tags/duration
- `src/mpris.rs`: MPRIS D-Bus service (zbus)

## Components

### Library scan (`src/library.rs`)

Responsibility: given a directory, return a sorted `Vec<Track>`.

- Filters by file extension.
- Extracts tags (title/artist/album) and duration when available.
- Precomputes a `Track.display` string used by the UI and filtering.

### App state (`src/app.rs`)

Responsibility: hold all UI state that isn’t “owned” by the audio output.

Key fields (conceptually):

- `tracks`: full library
- `selected`: selected track index
- `filter_mode` + `filter_query`: `/` search mode
- `follow_playback`: cursor mode (follow vs free-roam)
- `shuffle`, `loop_mode`: playback behavior settings
- `metadata_window`: whether the metadata popup is visible

Important invariant: `display_indices()` is the authoritative list for “what you see” and “what actions operate on”. If rendering, navigation, and playback use different filtering/shuffle logic, you will get hard-to-debug mismatches.

### UI (`src/ui.rs`)

Responsibility: render the screen. The UI is stateless beyond what’s in `App`.

- The track list is always rendered.
- The metadata view is a popup overlay, not a separate “screen”.

### Audio thread (`src/audio.rs`)

Responsibility: keep an `OutputStream` alive and respond to `AudioCmd` messages.

Reasons for a dedicated thread:

- Rodio playback should not block the input/render loop.
- The audio thread can be the single owner of the sink and queue state.

Queue semantics:

- The UI sends the current visible list (filtered/unfiltered + shuffle) to the audio thread as the playback queue.
- Next/prev and auto-advance operate on that queue.

Seeking/scrubbing:

- Implemented by recreating the sink and starting a decoder at a skipped position (`Source::skip_duration`).
- This avoids depending on true random access for all formats/decoders.
- Tradeoff: position accuracy varies by codec; large seeks are not instantaneous.

Soft quit:

- Quit fades the sink volume down briefly before stopping.
- The main thread joins the audio thread on quit so shutdown is deterministic.

## MPRIS (`src/mpris.rs`)

The MPRIS service maps external commands (media keys, `playerctl`) into the same control flow as keyboard input. This keeps behavior consistent.

See [MPRIS.md](MPRIS.md) for details and troubleshooting.
