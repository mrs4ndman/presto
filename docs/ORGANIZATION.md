# Organization and Direction

This document captures current product scope and near-term direction.

## Current scope

Presto is a local-file, terminal-first music player with:

- keyboard-centric navigation
- queue-based playback semantics
- optional per-directory state persistence
- MPRIS control integration

## Non-goals (current)

- streaming services
- multi-client server architecture (today's app is single local process)
- heavyweight library database beyond runtime scan + lightweight persisted state

## Design principles

- Keep runtime responsive by isolating audio in its own thread.
- Keep UI logic deterministic and derived from `App` state.
- Prefer small focused modules over monolithic files.
- Keep defaults simple; advanced behavior opt-in via config.

## Near-term improvements

- Status line configurability (show/hide segments)
- Additional tests around UI list-window calculations
- Better diagnostics for audio and DBus startup failures

## Longer-term ideas

- Theming
- richer listening stats
- optional cross-platform polish for config/state and media key behavior
