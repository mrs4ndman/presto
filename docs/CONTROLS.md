
# Controls

This is the complete keyboard reference for Presto.

## Navigation

- `j`: move down
- `k`: move up
- `gg`: jump to top
- `G`: jump to bottom

Count prefixes:

- Prefix a number to repeat movement: `10j`, `3k`, `2h`, `4l`.
- The current count shows in the bottom input panel while you type it, unless `ui.show_pending_count = false`.

## Playback

- `Enter`: play selected track (no-op if it’s already the current track)
- `Space` / `p`: play/pause (maps to MPRIS PlayPause)
- `h`: previous track
- `l`: next track
- `H`: scrub backward (default 5s; configurable)
- `L`: scrub forward (default 5s; configurable)
- `-`: volume down
- `+`: volume up
- `=`: reset volume to the initial level
- `r`: cycle loop mode (LoopAll → LoopOne → NoLoop)
- `s`: toggle shuffle
- `q`: quit (soft fade-out)

Notes:

- Scrub amount is controlled by `controls.scrub_seconds` in the config file.
- Volume step is controlled by `controls.volume_step_percent`; the starting level comes from
  `audio.initial_volume_percent`.
- Default loop mode is LoopAll (loop-around) unless overridden in config.

## Metadata

- `K`: toggle metadata popup for the selected track

Notes:

- The metadata view is a right-side pane; the track list stays visible on the left.
- Long fields wrap within the pane.

## Lyrics

- `gl`: toggle the lyrics pane for the currently playing track

Notes:

- Lyrics loading is disabled by default; enable it with `ui.lyrics_enabled = true`.
- When timed lyrics are available, the current line is highlighted in bold.
- If metadata and lyrics are both open, they stack in the right rail.

## Controls popup

- `g?`: toggle the controls popup
- `Esc`: close the popup

## Filtering (search)

- `/`: enter filter mode
- While in filter mode:
  - type to filter titles using word-by-word fuzzy matching
  - `Backspace`: delete
  - `Esc`: clear filter and exit filter mode
  - `Enter`: play the selected match and exit filter mode
  - `Ctrl-e`: exit filter mode without playing (filter stays active)
  - `Ctrl-n` / `Ctrl-j`: move selection down within the filtered results
  - `Ctrl-p` / `Ctrl-k`: move selection up within the filtered results

Notes:

- Each typed term must match within a title word, in order.
- If the filter is empty, typing digits starts a count prefix instead of inserting digits.

## Line numbers

Line numbers for the track list are configurable in `config.toml`:

- Relative only (blank on the current row)
- Absolute current line only
- Hybrid (current line number + relative for other rows)

## Cursor behavior

Presto has two cursor behaviors:

- **Follow playback:** selection tracks the currently playing song.
- **Free-roam:** selection is independent; playback continues.

Moving the cursor manually switches you into free-roam.
