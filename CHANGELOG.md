## Remarkable things that have happened

| Date (DD-MM-YYYY) | Feature                                                                                | Notes                                                     |
| ----------------- | -------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| 31-01-2026        | Bug: fix desynchronization between current-playing and highlighted song                |                                                           |
| 31-01-2026        | Testing: Implement initial test coverage for the project                               | Has been expanded in the next commits with each feature   |
| 01-02-2026        | Feature: Full `playerctl` + MPRIS interface compliance                                 |                                                           |
| 01-02-2026        | Feature: Added a proper “Now Playing” line + elapsed time                              |                                                           |
| 01-02-2026        | Feature: `gg`/`G` to jump to top/bottom of track list                                  |                                                           |
| 01-02-2026        | Tweak: Emit MPRIS PropertiesChanged so status/metadata updates push instantly          |                                                           |
| 01-02-2026        | Tweak: Improve filtering to fuzzy-match + highlight matches in the list                |                                                           |
| 01-02-2026        | Tweak: Wrap status text in the UI                                                      |                                                           |
| 02-02-2026        | Feature: Song scrubbing (FF/FB) with `H` / `L`                                         | Now configurable through env variables or the config file |
| 02-02-2026        | Feature: Config file -> Usual settings / toggles made configurable within the app      |                                                           |
| 02-02-2026        | Feature: Metadata window (`K`) + extraction from song files                            |                                                           | 
| 02-02-2026        | Feature: Crossfade between songs and on quitting the program                           |                                                           | 
| 02-02-2026        | Tweak: Reorganize keybinds to be more vim-like                                         |                                                           |
| 02-02-2026        | Fix: When hitting play-selected on currently playing song, do nothing                  |                                                           |
| 02-02-2026        | Fix: When no arguments are given start up on current dir                               |                                                           |
| 02-02-2026        | Fix: filter mode should not captures keypresses (you can type `j` / `k`)               |                                                           |
| 04-02-2026        | Bug: `gg` / `G` worked only after moving around the cursor with `j/k`                  |                                                           |
| 04-02-2026        | Bug: Auto-locating to the top after shuffle only worked after moving around the cursor |                                                           |
| 05-02-2026        | Feature: `zz` functionality to reselect & center currently playing song                |                                                           |
| 06-02-2026        | Tweak: Pad shown data in the UI on the sides with 1 space on the left                  |                                                           |
| 09-02-2026        | Feature: Persist per-directory selection, filter, shuffle, loop, follow-playback      | Stored in `state.toml` next to config                      |
| 09-02-2026        | Tweak: Make status/footer wrapping aware of terminal width                             | Prevents corruption on narrow widths                       |
| 09-02-2026        | Tweak: Volume controls remapped (`-` down, `+` up, `=` reset to initial)               | Initial volume uses config default (50%)                   |
| 09-02-2026        | Testing: Expanded unit coverage for volume helpers and state persistence               |                                                           |
| 09-02-2026        | Tweak: Ctrl+E exits filter input without starting playback                             | Filter remains active                                      |
| 09-02-2026        | Fix: `zz` no longer jumps to track 0 when nothing is playing                            | Uses playback index when available                         |
| 09-02-2026        | Tweak: State persistence errors include path and surface as UI notice                  | Logged as structured error output                          |
| 09-02-2026        | Refactor: Extract pure helpers for shuffle reselect and follow playback                | Added targeted unit tests                                  |
| 09-02-2026        | Feature: Metadata view moved to a right-side pane with wrapping                        | Replaces popup; keeps list visible                          |
| 09-02-2026        | Feature: Relative/current line numbers with count prefixes for `h/j/k/l`               | Count shows in input panel; current row can be blank         |
| 09-02-2026        | Feature: Persist last volume and last played track                                     | Applied with soft fallback on missing tracks                |
| 09-02-2026        | Tweak: Status shows absolute directory path                                             | Useful when launched from relative paths                    |
| 09-02-2026        | Docs: Expanded config example and UI/controls documentation                            | Includes state toggle and line number options               |
| 09-02-2026        | Fix: Preserve multi-space display separators in the track list                          | Whitespace is no longer collapsed in wrapped list items      |
| 09-02-2026        | Tweak: Expand config validation and reporting                                           | Flags invalid ranges and empty extension lists              |
| 09-02-2026        | Tweak: Add UI app-state indicator in the status line                                    | Shows FILTER / NAVIGATION / FOLLOWING_PLAYING               |
