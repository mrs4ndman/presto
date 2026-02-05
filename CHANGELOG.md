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
