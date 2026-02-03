# Configuration (`config.toml`)

Presto supports an optional `config.toml` file for user-tunable settings.

## Location

By default on Linux (XDG), Presto looks here:

- `$XDG_CONFIG_HOME/presto/config.toml`
- fallback: `~/.config/presto/config.toml`

You can also override the path entirely with:

- `PRESTO_CONFIG_PATH=/some/path/config.toml`

## Precedence

Highest wins:

1. Environment variables (prefix `PRESTO__`, `__` separates nested fields)
2. `config.toml`
3. Built-in defaults

Example environment override:

```sh
PRESTO__AUDIO__CROSSFADE_MS=0 presto
```

## Schema

### `[playback]`

- `shuffle` (bool): start with shuffle enabled
- `loop_mode` (string): one of `no-loop`, `loop-all`/`loop-around`, `loop-one`/`repeat-one`

Defaults:

- `shuffle = false`
- `loop_mode = "loop-all"` (loop-around)

### `[audio]`

- `crossfade_ms` (u64, milliseconds): crossfade when switching tracks (`0` disables)
- `crossfade_steps` (u64): number of fade steps (must be `>= 1`)
- `quit_fade_out_ms` (u64, milliseconds): fade out on quit (`0` disables)

### `[controls]`

- `scrub_seconds` (u64): seconds to seek when pressing `H` / `L`

### `[ui]`

- `follow_playback` (bool): start in follow-playback mode
- `header_text` (string): the text rendered in the top "presto" box
- `now_playing_track_fields` (array of strings): which fields to show for the status "Song:" label
	- allowed: `display`, `title`, `artist`, `album`, `filename`, `path`
- `now_playing_track_separator` (string): how to join those fields
- `now_playing_time_fields` (array of strings): which time fields to show (and order)
	- allowed: `elapsed`, `total`, `remaining`
- `now_playing_time_separator` (string): how to join those time fields

### `[library]`

- `extensions` (array of strings): audio extensions (without dot)
- `follow_links` (bool): follow symlinks while scanning
- `include_hidden` (bool): scan dotfiles/directories
- `recursive` (bool): recurse into subdirectories
- `max_depth` (int or omitted): max directory depth
- `display_fields` (array of strings): how to build the track list label (`Track.display`)
	- allowed: `artist`, `title`, `album`, `filename`, `path` (and `display` acts like artist+title)
- `display_separator` (string): joiner used for `display_fields`

## Example

See `docs/config.example.toml` for a full example.
