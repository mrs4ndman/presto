# Presto Docs

Presto is a terminal music player built with Rust, `ratatui`, `rodio`, and MPRIS.

This folder documents the current codebase state.

## Start here

- User guide / keybindings: [CONTROLS.md](CONTROLS.md)
- Configuration reference: [CONFIG.md](CONFIG.md)
- Common issues: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

## Contributor docs

- System architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Dev workflow: [DEVELOPMENT.md](DEVELOPMENT.md)
- MPRIS integration: [MPRIS.md](MPRIS.md)
- Engineering notes: [LEARNING.md](LEARNING.md)
- Product direction: [ORGANIZATION.md](ORGANIZATION.md)

## Runtime summary

- Entry point: `src/main.rs` -> `runtime::run()`
- UI draw path: `src/ui/mod.rs` + `src/ui/*`
- Audio ownership: dedicated audio thread in `src/audio/thread.rs`
- Library scan: `src/library/scan.rs`
- MPRIS service: `src/mpris.rs`
- Optional state persistence: `src/runtime/state.rs`


##### THANK YOU FOR TAKING A LOOK ;)
