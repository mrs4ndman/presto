# Architecture

This document explains how Presto is organized and how state flows through the app.

## Code map

- `src/main.rs`: binary entrypoint
- `src/runtime/mod.rs`: startup wiring and teardown
- `src/runtime/event_loop.rs`: input loop and orchestration
- `src/app/model.rs`: UI/app state model
- `src/ui/mod.rs`: render orchestration entrypoint (`ui::draw`)
- `src/ui/text.rs`: text formatting and wrapping helpers
- `src/ui/layout.rs`: frame and side-pane layout calculations
- `src/ui/panes.rs`: ratatui widget rendering
- `src/ui/lyrics.rs`: timed/plain lyrics rendering helpers
- `src/audio/types.rs`: audio command and shared playback types
- `src/audio/player.rs`: audio thread handle and spawn logic
- `src/audio/thread.rs`: audio worker loop
- `src/audio/queue.rs`: queue reorder logic for shuffle
- `src/library/scan.rs`: directory scanning and track extraction
- `src/library/lyrics.rs`: embedded lyrics loading/parsing
- `src/config/schema.rs`: settings schema/defaults
- `src/config/load.rs`: config loading and precedence
- `src/mpris.rs`: MPRIS DBus interface

## Runtime flow

1. Load settings (`config.toml` + env overrides).
2. Resolve target directory and scan tracks.
3. Spawn audio thread and shared handles.
4. Build `App` state and optionally apply persisted directory state.
5. Start terminal UI and MPRIS service.
6. Enter event loop:
   - sync queue if dirty
   - pull playback snapshot from audio thread
   - process MPRIS/media-key control commands
   - process keyboard input
   - render with `ui::draw`
7. On quit, persist state (if enabled), restore terminal, join audio thread.

## Core invariants

- `App::display_indices()` is the canonical visible list.
- Any change that affects visible playback order must mark queue dirty and send `AudioCmd::SetQueue`.
- Audio thread owns sink lifecycle and playback progression.
- UI rendering is derived from `App`; render code should not mutate core playback state.

## Queue semantics

- UI computes the visible queue (`display_indices`) and sends it to audio.
- Audio reorders queue according to current shuffle order.
- `Next`/`Prev` and auto-advance operate on this queue.
- Loop behavior:
  - `NoLoop`: stop at ends
  - `LoopAll`: wrap
  - `LoopOne`: repeat current track

## UI split rationale

The UI module was split to reduce coupling and review risk:

- `mod.rs`: orchestration only
- `text.rs`: pure formatting helpers (easy unit tests)
- `layout.rs`: geometry calculations
- `panes.rs`: widget construction/rendering
- `lyrics.rs`: timed-lyrics windowing and styling

This keeps `ui::draw(...)` stable while making changes local and testable.
