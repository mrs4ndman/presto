use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::app::{App, PlaybackState};
use crate::audio::{AudioCmd, AudioPlayer};
use crate::config;
use crate::mpris::ControlCmd;
use crate::mpris::MprisHandle;
use crate::runtime::mpris_sync::update_mpris;
use crate::ui;

/// State tracked by the runtime event loop across iterations.
pub struct EventLoopState {
    /// Optional snapshot of prior order when shuffle was toggled; used to
    /// detect a changed randomized order and reselect the top item.
    pub pending_shuffle_reselect_from: Option<Vec<usize>>,
    /// Internal two-key prefix state used for `gg` handling.
    pub pending_gg: bool,
    /// Last-known playing index as emitted to MPRIS.
    pub last_mpris_index: Option<usize>,
    /// Last-known playback state as emitted to MPRIS.
    pub last_mpris_playback: PlaybackState,
    pending_zz: bool,
}

impl EventLoopState {
    /// Construct a new `EventLoopState` seeded from `app`.
    pub fn new(app: &App) -> Self {
        Self {
            pending_shuffle_reselect_from: None,
            pending_gg: false,
            pending_zz: false,
            last_mpris_index: None,
            last_mpris_playback: app.playback,
        }
    }
}

/// Main terminal event loop: handles input, UI drawing, sync with the audio
/// thread and MPRIS. Returns `Ok(())` when shutdown is requested.
pub fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    settings: &config::Settings,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris: &MprisHandle,
    control_tx: &mpsc::Sender<ControlCmd>,
    control_rx: &mpsc::Receiver<ControlCmd>,
    state: &mut EventLoopState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // If shuffle just turned on, reselect the first track in the new randomized order.
        if state.pending_shuffle_reselect_from.is_some() && app.shuffle && !app.filter_mode {
            if let Some(ref oh) = app.order_handle {
                let new_order = oh.lock().ok().map(|v| v.clone());
                if let Some(v) = new_order {
                    let changed = state
                        .pending_shuffle_reselect_from
                        .as_ref()
                        .map(|old| old.as_slice() != v.as_slice())
                        .unwrap_or(true);
                    if changed && !v.is_empty() {
                        app.set_selected(v[0]);
                        state.pending_shuffle_reselect_from = None;
                    }
                }
            }
        }

        // Keep audio thread's queue in sync with the current visible list.
        if app.queue_dirty {
            let _ = audio_player.send(AudioCmd::SetQueue(app.display_indices()));
            app.clear_queue_dirty();
        }

        // Sync playback state from audio thread; optionally follow now-playing.
        // Clone the Arc handle to avoid borrowing `app` immutably across mutations.
        let mut playback_index_snapshot: Option<usize> = None;
        if let Some(handle) = app.playback_handle.as_ref().cloned() {
            if let Ok(info) = handle.lock() {
                let idx_opt = info.index;
                let is_playing = info.playing;
                drop(info);

                playback_index_snapshot = idx_opt;
                if let Some(idx) = idx_opt {
                    if app.follow_playback && !app.filter_mode {
                        if let Some(pending) = app.pending_follow_index {
                            if pending == idx {
                                app.clear_pending_follow_index();
                                if app.selected != idx {
                                    app.set_selected(idx);
                                }
                            }
                        } else if app.selected != idx {
                            app.set_selected(idx);
                        }
                    }
                }
                app.playback = if is_playing {
                    PlaybackState::Playing
                } else {
                    PlaybackState::Paused
                };
            }
        }

        // Keep MPRIS in sync even when playback changes come from XF86/media keys or auto-advance.
        if playback_index_snapshot != state.last_mpris_index
            || app.playback != state.last_mpris_playback
        {
            update_mpris(mpris, app);
            state.last_mpris_index = playback_index_snapshot;
            state.last_mpris_playback = app.playback;
        }

        let display = app.display_indices();
        terminal.draw(|f| ui::draw(f, app, &display, &settings.ui, &settings.controls))?;

        while let Ok(cmd) = control_rx.try_recv() {
            if handle_control_cmd(cmd, settings, app, audio_player, mpris)? {
                return Ok(());
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key_event(key, settings, app, audio_player, mpris, control_tx, state)? {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn handle_control_cmd(
    cmd: ControlCmd,
    settings: &config::Settings,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris: &MprisHandle,
) -> Result<bool, Box<dyn std::error::Error>> {
    match cmd {
        ControlCmd::Quit => {
            audio_player.quit_softly(Duration::from_millis(settings.audio.quit_fade_out_ms));
            return Ok(true);
        }
        ControlCmd::Play => match app.playback {
            PlaybackState::Paused => {
                if !app.filter_mode {
                    app.follow_playback_on();
                }
                let _ = audio_player.send(AudioCmd::TogglePause);
                app.playback = PlaybackState::Playing;
                update_mpris(mpris, app);
            }
            PlaybackState::Stopped | PlaybackState::Playing => {
                if app.has_tracks() {
                    if !app.filter_mode {
                        app.follow_playback_on();
                    }
                    let _ = audio_player.send(AudioCmd::Play(app.selected));
                    app.playback = PlaybackState::Playing;
                    update_mpris(mpris, app);
                }
            }
        },
        ControlCmd::Pause => {
            if app.playback == PlaybackState::Playing {
                if !app.filter_mode {
                    app.follow_playback_on();
                }
                let _ = audio_player.send(AudioCmd::TogglePause);
                app.playback = PlaybackState::Paused;
                update_mpris(mpris, app);
            }
        }
        ControlCmd::PlayPause => {
            if !app.filter_mode {
                app.follow_playback_on();
            }
            match app.playback {
                PlaybackState::Stopped => {
                    if app.has_tracks() {
                        let _ = audio_player.send(AudioCmd::Play(app.selected));
                        app.playback = PlaybackState::Playing;
                    }
                }
                PlaybackState::Playing => {
                    let _ = audio_player.send(AudioCmd::TogglePause);
                    app.playback = PlaybackState::Paused;
                }
                PlaybackState::Paused => {
                    let _ = audio_player.send(AudioCmd::TogglePause);
                    app.playback = PlaybackState::Playing;
                }
            }
            update_mpris(mpris, app);
        }
        ControlCmd::Stop => {
            if !app.filter_mode {
                app.follow_playback_on();
            }
            let _ = audio_player.send(AudioCmd::Stop);
            app.playback = PlaybackState::Stopped;
            update_mpris(mpris, app);
        }
        ControlCmd::Next => {
            if app.has_tracks() {
                if !app.filter_mode {
                    app.follow_playback_on();
                }
                let _ = audio_player.send(AudioCmd::Next);
                app.playback = PlaybackState::Playing;
                update_mpris(mpris, app);
            }
        }
        ControlCmd::Prev => {
            if app.has_tracks() {
                if !app.filter_mode {
                    app.follow_playback_on();
                }
                let _ = audio_player.send(AudioCmd::Prev);
                app.playback = PlaybackState::Playing;
                update_mpris(mpris, app);
            }
        }
    }

    Ok(false)
}

fn handle_key_event(
    key: KeyEvent,
    settings: &config::Settings,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris: &MprisHandle,
    control_tx: &mpsc::Sender<ControlCmd>,
    state: &mut EventLoopState,
) -> Result<bool, Box<dyn std::error::Error>> {
    if app.filter_mode {
        state.pending_gg = false;
        match key.code {
            KeyCode::Esc => {
                app.clear_filter();
                update_mpris(mpris, app);
            }
            KeyCode::Backspace => {
                app.pop_filter_char();
                update_mpris(mpris, app);
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.follow_playback_off();
                app.next();
                update_mpris(mpris, app);
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.follow_playback_off();
                app.prev();
                update_mpris(mpris, app);
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.follow_playback_off();
                app.next();
                update_mpris(mpris, app);
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.follow_playback_off();
                app.prev();
                update_mpris(mpris, app);
            }
            KeyCode::Char(c) => {
                if !c.is_control() {
                    app.push_filter_char(c);
                    update_mpris(mpris, app);
                }
            }
            KeyCode::Enter => {
                if app.display_indices().is_empty() {
                    return Ok(false);
                }

                app.exit_filter_mode();
                app.follow_playback_on();
                app.set_pending_follow_index(app.selected);
                let _ = audio_player.send(AudioCmd::Play(app.selected));
                app.playback = PlaybackState::Playing;
                update_mpris(mpris, app);
            }
            _ => {}
        }

        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') => {
            state.pending_gg = false;
            audio_player.quit_softly(Duration::from_millis(settings.audio.quit_fade_out_ms));
            return Ok(true);
        }
        KeyCode::Char('/') => {
            state.pending_gg = false;
            app.enter_filter_mode();
            update_mpris(mpris, app);
        }
        KeyCode::Char('s') => {
            state.pending_gg = false;
            let turning_on = !app.shuffle;
            if turning_on {
                state.pending_shuffle_reselect_from = app
                    .order_handle
                    .as_ref()
                    .and_then(|h| h.lock().ok().map(|v| v.clone()))
                    .or_else(|| Some((0..app.tracks.len()).collect()));
            }
            let _ = audio_player.send(AudioCmd::ToggleShuffle);
            app.toggle_shuffle();
            if !app.shuffle {
                let display = app.display_indices();
                if let Some(&first) = display.first() {
                    app.set_selected(first);
                }
                state.pending_shuffle_reselect_from = None;
            }
            update_mpris(mpris, app);
        }
        KeyCode::Char('r') => {
            state.pending_gg = false;
            app.cycle_loop_mode();
            let _ = audio_player.send(AudioCmd::SetLoopMode(app.loop_mode));
            update_mpris(mpris, app);
        }
        KeyCode::Char('z') => {
            if state.pending_zz {
                state.pending_zz = false;
                let handle = &app.playback_handle;
                let mut track_id = 0;
                if let Some(handle_val) = handle {
                    if let Ok(info) = handle_val.lock() {
                        if let Some(id) = info.index {
                            track_id = id;
                        }
                    }
                 app.set_selected(track_id);
                    update_mpris(mpris, app);
                }
            } else {
                state.pending_zz = true;
            }
        }
        KeyCode::Char('g') => {
            if state.pending_gg {
                state.pending_gg = false;
                app.follow_playback_off();
                let display = app.display_indices();
                if let Some(&first) = display.first() {
                    app.set_selected(first);
                    update_mpris(mpris, app);
                }
            } else {
                state.pending_gg = true;
            }
        }
        KeyCode::Char('G') => {
            state.pending_gg = false;
            let display = app.display_indices();
            if let Some(&last) = display.last() {
                app.set_selected(last);
                update_mpris(mpris, app);
            }
        }
        KeyCode::Char('j') => {
            state.pending_gg = false;
            app.follow_playback_off();
            app.next();
            update_mpris(mpris, app);
        }
        KeyCode::Char('k') => {
            state.pending_gg = false;
            app.follow_playback_off();
            app.prev();
            update_mpris(mpris, app);
        }
        KeyCode::Enter => {
            state.pending_gg = false;
            if app.has_tracks() {
                let is_playing_selected = app.playback == PlaybackState::Playing
                    && app
                        .playback_handle
                        .as_ref()
                        .and_then(|h| h.lock().ok().and_then(|info| info.index))
                        .map(|idx| idx == app.selected)
                        .unwrap_or(false);
                if !is_playing_selected {
                    app.follow_playback_on();
                    app.set_pending_follow_index(app.selected);
                    let _ = audio_player.send(AudioCmd::Play(app.selected));
                    app.playback = PlaybackState::Playing;
                    update_mpris(mpris, app);
                }
            }
        }
        KeyCode::Char('p') | KeyCode::Char(' ') => {
            state.pending_gg = false;
            let _ = control_tx.send(ControlCmd::PlayPause);
        }
        KeyCode::Char('l') => {
            state.pending_gg = false;
            let _ = control_tx.send(ControlCmd::Next);
        }
        KeyCode::Char('h') => {
            state.pending_gg = false;
            let _ = control_tx.send(ControlCmd::Prev);
        }
        KeyCode::Char('L') => {
            state.pending_gg = false;
            let secs = settings.controls.scrub_seconds.min(i32::MAX as u64) as i32;
            let _ = audio_player.send(AudioCmd::SeekBy(secs));
        }
        KeyCode::Char('H') => {
            state.pending_gg = false;
            let secs = settings.controls.scrub_seconds.min(i32::MAX as u64) as i32;
            let _ = audio_player.send(AudioCmd::SeekBy(-secs));
        }
        KeyCode::Char('K') => {
            state.pending_gg = false;
            app.toggle_metadata_window();
            update_mpris(mpris, app);
        }
        KeyCode::Char(_) => {
            // g pending should clear on any other printable char
            state.pending_gg = false;
        }
        _ => {}
    }

    Ok(false)
}
