
# Controls

This is the complete keyboard reference for Presto.

## Navigation

- `j`: move down
- `k`: move up
- `gg`: jump to top
- `G`: jump to bottom

## Playback

- `Enter`: play selected track (no-op if it’s already the current track)
- `Space` / `p`: play/pause (maps to MPRIS PlayPause)
- `h`: previous track
- `l`: next track
- `H`: scrub backward (default 5s; configurable)
- `L`: scrub forward (default 5s; configurable)
- `r`: cycle loop mode (LoopAll → LoopOne → NoLoop)
- `s`: toggle shuffle
- `q`: quit (soft fade-out)

Notes:

- Scrub amount is controlled by `controls.scrub_seconds` in the config file.
- Default loop mode is LoopAll (loop-around) unless overridden in config.

## Metadata

- `K`: toggle metadata popup for the selected track

Notes:

- The metadata view is a popup overlay; the track list stays visible underneath.
- The popup is intentionally small. If the path/fields are long, they will wrap.

## Filtering (search)

- `/`: enter filter mode
- While in filter mode:
  - type to filter titles (fuzzy/subsequence match)
  - `Backspace`: delete
  - `Esc`: clear filter and exit filter mode
  - `Enter`: play the selected match and exit filter mode
  - `Ctrl-n` / `Ctrl-j`: move selection down within the filtered results
  - `Ctrl-p` / `Ctrl-k`: move selection up within the filtered results

## Cursor behavior

Presto has two cursor behaviors:

- **Follow playback:** selection tracks the currently playing song.
- **Free-roam:** selection is independent; playback continues.

Moving the cursor manually switches you into free-roam.
