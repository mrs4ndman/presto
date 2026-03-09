# Controls

## Navigation

- `j` / `k`: move selection down/up
- `gg`: jump to top
- `G`: jump to bottom
- `z` then `z`: jump selection to currently playing track

Numeric count prefixes are supported in normal mode:

- `10j`, `3k`, `2h`, `4l`
- Pending count is shown in UI when `ui.show_pending_count = true`

## Playback

- `Enter`: play selected track
- `Space` or `p`: play/pause
- `h` / `l`: previous/next track
- `H` / `L`: seek backward/forward (`controls.scrub_seconds`)
- `r`: cycle loop mode
- `s`: toggle shuffle
- `q`: quit (soft fade when playing)

## Volume

- `-`: volume down by `controls.volume_step_percent`
- `+`: volume up by `controls.volume_step_percent`
- `=`: reset to configured initial volume

## Panels and overlays

- `K`: toggle metadata side pane
- `g` then `l` (`gl`): toggle lyrics side pane (requires the setting`ui.lyrics_enabled = true` in the TOML file or as an environment variable)
- `g` then `?` (`g?`): toggle controls popup
- `Esc`: close controls popup and lyrics pane, clear pending key/count

## Filter mode

- `/`: enter filter mode
- Typing characters: update filter query
- `Backspace`: delete character
- `Enter`: play selected filtered track and exit filter mode
- `Ctrl+e`: exit filter mode without starting playback
- `Esc`: clear filter and exit filter mode
- `Ctrl+j` / `Ctrl+n`: move down inside filtered results
- `Ctrl+k` / `Ctrl+p`: move up inside filtered results

Filter matching is word-by-word fuzzy matching in order.
