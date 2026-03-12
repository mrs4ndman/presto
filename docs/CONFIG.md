# Configuration (`config.toml`)

Presto reads settings from TOML with optional environment overrides.

## Location

Default Linux/XDG path:

- `$XDG_CONFIG_HOME/presto/config.toml`
- fallback: `~/.config/presto/config.toml`

Override explicit path:

- `PRESTO_CONFIG_PATH=/absolute/path/config.toml`

## Precedence

1. Environment variables (`PRESTO__SECTION__KEY=value`)
2. `config.toml`
3. Built-in defaults (`src/config/schema.rs`)

Example:

```sh
PRESTO__AUDIO__CROSSFADE_MS=0 presto
```

## Sections

### `[audio]`

- `crossfade_ms` (u64, default `250`)
- `crossfade_steps` (u64, default `10`, must be `>= 1`)
- `quit_fade_out_ms` (u64, default `500`)
- `initial_volume_percent` (u8, default `50`, range `0..=100`)

### `[ui]`

- `follow_playback` (bool, default `true`)
- `lyrics_enabled` (bool, default `false`)
- `show_pending_count` (bool, default `true`)
- `show_relative_numbers` (bool, default `false`)
- `show_current_line_number` (bool, default `false`)
- `header_text` (string)
- `now_playing_track_fields` (array): `display|title|artist|album|filename|path`
- `now_playing_track_separator` (string)
- `now_playing_time_fields` (array): `elapsed|total|remaining`
- `now_playing_time_separator` (string)

### `[controls]`

- `scrub_seconds` (u64, default `5`)
- `scrub_batch_window_ms` (u64, default `250`, `0` disables batching)
- `volume_step_percent` (u8, default `5`)

### `[playback]`

- `shuffle` (bool, default `false`)
- `loop_mode` (string, default `loop-all`)
  - accepted aliases:
    - no loop: `no-loop`, `no_loop`
    - loop all: `loop-all`, `loop_all`, `loopall`, `loop-around`
    - loop one: `loop-one`, `loop_one`, `loopone`, `repeat-one`

### `[library]`

- `extensions` (`["mp3","flac","wav","ogg"]` by default)
- `follow_links` (bool, default `true`)
- `include_hidden` (bool, default `true`)
- `recursive` (bool, default `true`)
- `max_depth` (optional integer)
- `display_fields` (array): same enum as `now_playing_track_fields`
- `display_separator` (string)

### `[state]`

- `enabled` (bool, default `false`)

When enabled, per-directory state is loaded/saved in `state.toml` next to config.

## Example

See [config.example.toml](config.example.toml).
