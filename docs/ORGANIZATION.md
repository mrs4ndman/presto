# Organization + Future plans / Ideas (OLD)

## 1. ***PURPOSE***
- Lightweight TUI music player + library manager (playlists and such)
- Keyboard-centric controls (Vim-bindings first, Emacs as backup)
- Not necessarily meant for remote usage, oriented for people with simple
  workflows / needs.

---

## 2. ***AUDIO BACKEND***
- Audio library: implementation for it, not a wrapper ideally
- Local-files only, no streaming
- Playlist system, but considering queues at the moment

---

## 3. ***DATA MODEL***
- Elements:
    - Track
    - Album
    - Artist
    - Playslist / Queue
- Want to support many formats: MP3, FLAC, etc.
- Caching for entire playlists and song metadata, editing on the fly
- Resizable song pool to build playlists from

---

## 4. ***APPLICATION STATE***
- Multiple-server + multiple-frontends architecture
- Spin up single server on single folders (kinda like doing a single playlist) vs.
  using the entire library (user-defined dirs)
- Clients get the playback state for the specific server they're connecting to
- Modal navigation for the front-ends: keyboard-driven

---

## 5. ***KEYBINDINGS & SETTINGS***
- Vim-based
- User-definable
- Sane defaults with arrows too
- Config file for all settings (TOML probably)
- Theming extensibility

---

## 6. ***FUTURE LAYOUT***

```
presto/
├── presto-core/
│   ├── audio/
│   ├── library/
│   ├── queue/
│   ├── state/
│   └── events/
├── presto-server/
│   ├── main.rs
│   ├── ipc/
│   └── server.rs
├── presto-tui/
│   ├── main.rs
│   ├── app.rs
│   └── ui.rs
└── Cargo.toml (workspace)
```

---

## 7. ***SERVER LIFECYCLE (PERSISTENT)***

```
presto-server
  ├─ check if socket exists
  ├─ if running → exit with message
  ├─ scan music directory
  ├─ build track list
  ├─ start audio thread
  ├─ start IPC listener
  └─ idle
```
