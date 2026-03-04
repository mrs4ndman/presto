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

/// Track a pending single-key sequence (like `gg` or `zz`).
#[derive(Default)]
struct PendingKey {
    expected: Option<char>,
}

impl PendingKey {
    fn clear(&mut self) {
        self.expected = None;
    }

    fn set(&mut self, key: char) {
        self.expected = Some(key);
    }

    fn take_if(&mut self, key: char) -> bool {
        if self.expected == Some(key) {
            self.expected = None;
            true
        } else {
            false
        }
    }
}

/// State tracked by the runtime event loop across iterations.
pub struct EventLoopState {
    /// Optional snapshot of prior order when shuffle was toggled; used to
    /// detect a changed randomized order and reselect the top item.
    pub pending_shuffle_reselect_from: Option<Vec<usize>>,
    /// Internal two-key prefix state used for `gg`/`zz` handling.
    pending_key: PendingKey,
    /// Pending numeric prefix (Vim-like count), e.g. `10j`.
    pending_count: Option<u32>,
    /// Last-known playing index as emitted to MPRIS.
    pub last_mpris_index: Option<usize>,
    /// Last-known playback state as emitted to MPRIS.
    pub last_mpris_playback: PlaybackState,
}

impl EventLoopState {
    /// Construct a new `EventLoopState` seeded from `app`.
    pub fn new(app: &App) -> Self {
        Self {
            pending_shuffle_reselect_from: None,
            pending_key: PendingKey::default(),
            pending_count: None,
            last_mpris_index: None,
            last_mpris_playback: app.playback,
        }
    }

    fn clear_count(&mut self) {
        self.pending_count = None;
    }

    fn push_count_digit(&mut self, digit: u32) {
        let cur = self.pending_count.unwrap_or(0);
        let next = cur.saturating_mul(10).saturating_add(digit);
        // Keep counts bounded to avoid huge loops on accidental key spam.
        self.pending_count = Some(next.min(999));
    }

    fn take_count_or_default(&mut self) -> usize {
        let n = self.pending_count.take().unwrap_or(1);
        n.max(1) as usize
    }
}

#[derive(Debug, PartialEq)]
struct FollowUpdate {
    clear_pending: bool,
    select_index: Option<usize>,
}

/// Compute whether follow-playback should move the cursor this tick.
fn follow_playback_update(
    follow_playback: bool,
    filter_mode: bool,
    pending_follow_index: Option<usize>,
    selected: usize,
    playing_index: Option<usize>,
) -> FollowUpdate {
    if !follow_playback || filter_mode {
        return FollowUpdate {
            clear_pending: false,
            select_index: None,
        };
    }

    let Some(idx) = playing_index else {
        return FollowUpdate {
            clear_pending: false,
            select_index: None,
        };
    };

    if let Some(pending) = pending_follow_index {
        if pending == idx {
            return FollowUpdate {
                clear_pending: true,
                select_index: if selected != idx { Some(idx) } else { None },
            };
        }

        return FollowUpdate {
            clear_pending: false,
            select_index: None,
        };
    }

    FollowUpdate {
        clear_pending: false,
        select_index: if selected != idx { Some(idx) } else { None },
    }
}

/// Decide whether to reselect the first track after a shuffle reorder.
fn shuffle_reselect_target(
    pending_from: Option<&[usize]>,
    new_order: Option<&[usize]>,
    shuffle_enabled: bool,
    filter_mode: bool,
) -> Option<usize> {
    if pending_from.is_none() || !shuffle_enabled || filter_mode {
        return None;
    }

    let new_order = new_order?;
    let changed = pending_from.map(|old| old != new_order).unwrap_or(true);
    if changed && !new_order.is_empty() {
        Some(new_order[0])
    } else {
        None
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
        let new_order =
            if state.pending_shuffle_reselect_from.is_some() && app.shuffle && !app.filter_mode {
                app.order_handle
                    .as_ref()
                    .and_then(|h| h.lock().ok().map(|v| v.clone()))
            } else {
                None
            };

        if let Some(target) = shuffle_reselect_target(
            state.pending_shuffle_reselect_from.as_deref(),
            new_order.as_deref(),
            app.shuffle,
            app.filter_mode,
        ) {
            app.set_selected(target);
            state.pending_shuffle_reselect_from = None;
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
                let follow_update = follow_playback_update(
                    app.follow_playback,
                    app.filter_mode,
                    app.pending_follow_index,
                    app.selected,
                    idx_opt,
                );
                if follow_update.clear_pending {
                    app.clear_pending_follow_index();
                }
                if let Some(new_selected) = follow_update.select_index {
                    app.set_selected(new_selected);
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

        sync_pending_count(state, app);
        let display = app.display_indices();
        terminal.draw(|f| ui::draw(f, app, &display, &settings.ui, &settings.controls))?;
    }

    Ok(())
}

/// Apply a control command from MPRIS/media keys; return true to quit.
fn handle_control_cmd(
    cmd: ControlCmd,
    settings: &config::Settings,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris: &MprisHandle,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Control commands mirror media key/MPRIS actions; return true to quit.
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

struct VolumeControl {
    step_percent: u8,
}

impl VolumeControl {
    /// Create a volume control with a fixed step percentage.
    fn new(step_percent: u8) -> Self {
        Self { step_percent }
    }

    /// Apply a signed step delta and clamp the result to 0.0..=1.0.
    fn apply_delta(&self, current: f32, delta_sign: f32) -> f32 {
        let step = (self.step_percent as f32) / 100.0;
        if step <= 0.0 {
            return current.clamp(0.0, 1.0);
        }

        let delta = delta_sign * step;
        (current + delta).clamp(0.0, 1.0)
    }

    /// Return true if the new value is materially different.
    fn should_update(&self, current: f32, new: f32) -> bool {
        (new - current).abs() >= f32::EPSILON
    }
}

/// Apply a volume delta and send it to the audio thread.
fn adjust_volume(
    app: &mut App,
    audio_player: &AudioPlayer,
    settings: &config::Settings,
    delta_sign: f32,
) {
    // Apply a fixed percentage delta and push the new volume to the audio thread.
    let control = VolumeControl::new(settings.controls.volume_step_percent);
    let new_volume = control.apply_delta(app.volume(), delta_sign);
    if !control.should_update(app.volume(), new_volume) {
        return;
    }

    app.set_volume(new_volume);
    let _ = audio_player.send(AudioCmd::SetVolume(new_volume));
}

/// Reset volume to the initial config value and notify the audio thread.
fn reset_volume(app: &mut App, audio_player: &AudioPlayer) {
    let new_volume = app.reset_volume_to_initial();
    let _ = audio_player.send(AudioCmd::SetVolume(new_volume));
}

/// Clear the accumulated count in both the event loop and app state.
fn clear_pending_count(state: &mut EventLoopState, app: &mut App) {
    state.clear_count();
    app.pending_count = None;
}

/// Sync the event loop's pending count into the app for UI rendering.
fn sync_pending_count(state: &EventLoopState, app: &mut App) {
    app.pending_count = state.pending_count;
}

/// Route key events to the filter or normal handlers.
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
        state.pending_key.clear();
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && !key.modifiers.contains(KeyModifiers::CONTROL) {
                let digit = c.to_digit(10).unwrap_or(0);
                if state.pending_count.is_some() || app.filter_query.trim().is_empty() {
                    state.push_count_digit(digit);
                    sync_pending_count(state, app);
                    return Ok(false);
                }
            }
        }
        return handle_filter_key_event(key, app, audio_player, mpris);
    }
    handle_normal_key_event(key, settings, app, audio_player, mpris, control_tx, state)
}

/// Handle key events while the filter input is active.
fn handle_filter_key_event(
    key: KeyEvent,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris: &MprisHandle,
) -> Result<bool, Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Esc => {
            app.clear_filter();
            update_mpris(mpris, app);
        }
        KeyCode::Backspace => {
            app.pop_filter_char();
            update_mpris(mpris, app);
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.exit_filter_mode();
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

    Ok(false)
}

/// Handle key events in normal (non-filter) mode.
fn handle_normal_key_event(
    key: KeyEvent,
    settings: &config::Settings,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris: &MprisHandle,
    control_tx: &mpsc::Sender<ControlCmd>,
    state: &mut EventLoopState,
) -> Result<bool, Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Esc => {
            if app.controls_popup {
                app.toggle_controls_popup();
            }
            state.pending_key.clear();
            clear_pending_count(state, app);
        }
        KeyCode::Char('q') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            match state.last_mpris_playback {
                PlaybackState::Stopped | PlaybackState::Paused => {
                    return Ok(true);
                }
                PlaybackState::Playing => {
                    audio_player
                        .quit_softly(Duration::from_millis(settings.audio.quit_fade_out_ms));
                    return Ok(true);
                }
            }
        }
        KeyCode::Char(c)
            if c.is_ascii_digit() && !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            let digit = c.to_digit(10).unwrap_or(0);
            if digit != 0 || state.pending_count.is_some() {
                state.pending_key.clear();
                state.push_count_digit(digit);
                sync_pending_count(state, app);
                return Ok(false);
            }
        }
        KeyCode::Char('/') => {
            state.pending_key.clear();
            app.enter_filter_mode();
            update_mpris(mpris, app);
        }
        KeyCode::Char('s') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
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
            state.pending_key.clear();
            clear_pending_count(state, app);
            app.cycle_loop_mode();
            let _ = audio_player.send(AudioCmd::SetLoopMode(app.loop_mode));
            update_mpris(mpris, app);
        }
        KeyCode::Char('z') => {
            clear_pending_count(state, app);
            if state.pending_key.take_if('z') {
                let track_id = app
                    .playback_handle
                    .as_ref()
                    .and_then(|handle_val| handle_val.lock().ok().and_then(|info| info.index));
                if let Some(id) = track_id {
                    app.set_selected(id);
                    update_mpris(mpris, app);
                }
            } else {
                state.pending_key.set('z');
            }
        }
        KeyCode::Char('g') => {
            clear_pending_count(state, app);
            if state.pending_key.take_if('g') {
                app.follow_playback_off();
                let display = app.display_indices();
                if let Some(&first) = display.first() {
                    app.set_selected(first);
                    update_mpris(mpris, app);
                }
            } else {
                state.pending_key.set('g');
            }
        }
        KeyCode::Char('?') => {
            if state.pending_key.take_if('g') {
                clear_pending_count(state, app);
                app.toggle_controls_popup();
            } else {
                state.pending_key.clear();
            }
        }
        KeyCode::Char('G') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            let display = app.display_indices();
            if let Some(&last) = display.last() {
                app.set_selected(last);
                update_mpris(mpris, app);
            }
        }
        KeyCode::Char('j') => {
            state.pending_key.clear();
            let count = state.take_count_or_default();
            app.pending_count = None;
            app.follow_playback_off();
            for _ in 0..count {
                app.next();
            }
            update_mpris(mpris, app);
        }
        KeyCode::Char('k') => {
            state.pending_key.clear();
            let count = state.take_count_or_default();
            app.pending_count = None;
            app.follow_playback_off();
            for _ in 0..count {
                app.prev();
            }
            update_mpris(mpris, app);
        }
        KeyCode::Enter => {
            state.pending_key.clear();
            clear_pending_count(state, app);
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
            state.pending_key.clear();
            clear_pending_count(state, app);
            let _ = control_tx.send(ControlCmd::PlayPause);
        }
        KeyCode::Char('l') => {
            state.pending_key.clear();
            let count = state.take_count_or_default();
            app.pending_count = None;
            for _ in 0..count {
                let _ = control_tx.send(ControlCmd::Next);
            }
        }
        KeyCode::Char('h') => {
            state.pending_key.clear();
            let count = state.take_count_or_default();
            app.pending_count = None;
            for _ in 0..count {
                let _ = control_tx.send(ControlCmd::Prev);
            }
        }
        KeyCode::Char('-') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            adjust_volume(app, audio_player, settings, -1.0);
        }
        KeyCode::Char('+') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            adjust_volume(app, audio_player, settings, 1.0);
        }
        KeyCode::Char('=') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            reset_volume(app, audio_player);
        }
        KeyCode::Char('L') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            let secs = settings.controls.scrub_seconds.min(i32::MAX as u64) as i32;
            let _ = audio_player.send(AudioCmd::SeekBy(secs));
        }
        KeyCode::Char('H') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            let secs = settings.controls.scrub_seconds.min(i32::MAX as u64) as i32;
            let _ = audio_player.send(AudioCmd::SeekBy(-secs));
        }
        KeyCode::Char('K') => {
            state.pending_key.clear();
            clear_pending_count(state, app);
            app.toggle_metadata_window();
            update_mpris(mpris, app);
        }
        KeyCode::Char(_) => {
            // pending should clear on any other printable char
            state.pending_key.clear();
            clear_pending_count(state, app);
        }
        _ => {}
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{VolumeControl, follow_playback_update, shuffle_reselect_target};

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "{} != {}",
            actual,
            expected
        );
    }

    #[test]
    fn volume_control_applies_step_and_clamps() {
        let control = VolumeControl::new(5);
        assert_close(control.apply_delta(0.50, 1.0), 0.55);
        assert_close(control.apply_delta(0.50, -1.0), 0.45);
        assert_close(control.apply_delta(0.02, -1.0), 0.0);
        assert_close(control.apply_delta(0.98, 1.0), 1.0);
    }

    #[test]
    fn volume_control_handles_zero_step() {
        let control = VolumeControl::new(0);
        assert_close(control.apply_delta(1.5, 1.0), 1.0);
        assert_close(control.apply_delta(-0.1, -1.0), 0.0);
    }

    #[test]
    fn shuffle_reselect_target_picks_first_when_changed() {
        let old = vec![2, 1, 0];
        let new_order = vec![1, 2, 0];
        let target = shuffle_reselect_target(
            Some(old.as_slice()),
            Some(new_order.as_slice()),
            true,
            false,
        );
        assert_eq!(target, Some(1));
    }

    #[test]
    fn shuffle_reselect_target_skips_when_unchanged_or_empty() {
        let old = vec![1, 2, 3];
        let target =
            shuffle_reselect_target(Some(old.as_slice()), Some(old.as_slice()), true, false);
        assert_eq!(target, None);

        let empty: Vec<usize> = Vec::new();
        let target =
            shuffle_reselect_target(Some(old.as_slice()), Some(empty.as_slice()), true, false);
        assert_eq!(target, None);
    }

    #[test]
    fn follow_playback_update_respects_pending_and_selection() {
        let update = follow_playback_update(true, false, Some(3), 1, Some(3));
        assert!(update.clear_pending);
        assert_eq!(update.select_index, Some(3));

        let update = follow_playback_update(true, false, Some(4), 1, Some(3));
        assert!(!update.clear_pending);
        assert_eq!(update.select_index, None);

        let update = follow_playback_update(true, false, None, 1, Some(2));
        assert_eq!(update.select_index, Some(2));
    }

    #[test]
    fn follow_playback_update_skips_when_disabled_or_filtered() {
        let update = follow_playback_update(false, false, None, 1, Some(2));
        assert_eq!(update.select_index, None);

        let update = follow_playback_update(true, true, None, 1, Some(2));
        assert_eq!(update.select_index, None);
    }
}
