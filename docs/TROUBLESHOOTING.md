# Troubleshooting

## No tracks found

- Run with an explicit directory: `cargo run -- /path/to/music`
- Confirm extensions are supported by `[library].extensions`
- Confirm directory read permissions

## Audio: no output device

Symptoms include stderr messages from audio thread initialization.

Checks:

- Verify local audio stack is running
- Verify user has device/session access
- Avoid testing first in headless/SSH/container environments without sound plumbing

## Seeking/scrubbing feels imprecise

Current seek implementation recreates sink and uses decoder skip duration.

Implications:

- Precision depends on codec/container
- Large jumps may be slower

## `playerctl` cannot see Presto

- Ensure user session D-Bus is active
- Run `playerctl -l` while Presto is open
- If missing, MPRIS is unavailable in that session type

## Media keys do not work but `playerctl` does

This is usually desktop-environment key routing, not a Presto backend issue.

## UI appears clipped

Use a larger terminal size. Panels are constrained and wrap text, but very narrow windows can still reduce readability.
