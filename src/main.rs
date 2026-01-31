use std::{env, path::Path};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod audio;
mod library;
mod mpris;
mod ui;

use app::{App, PlaybackState};
use audio::{AudioCmd, AudioPlayer};
use library::scan;
use mpris::{ControlCmd, MprisHandle};
use std::{sync::mpsc, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = env::args().nth(1).unwrap_or("Music".to_string());

    let tracks = scan(Path::new(&dir));
    let audio_player = AudioPlayer::new(tracks.clone());
    let mut app = App::new(tracks);
    app.set_current_dir(dir.clone());
    // Attach playback info handle so UI can show now-playing & elapsed time.
    app.set_playback_handle(audio_player.playback_handle());
    app.set_order_handle(audio_player.order_handle());

    let (control_tx, control_rx) = mpsc::channel::<ControlCmd>();
    let mpris = mpris::spawn_mpris(control_tx.clone());

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let update_mpris = |mpris: &MprisHandle, app: &App| {
        let now_playing_idx = if let Some(ref handle) = app.playback_handle {
            handle.lock().ok().and_then(|info| info.index)
        } else {
            None
        };

        let track = now_playing_idx.and_then(|i| app.tracks.get(i));
        mpris.set_track_metadata(now_playing_idx, track);
        mpris.set_playback(app.playback);
    };

    update_mpris(&mpris, &app);

    // Initialize playback queue + loop mode.
    let _ = audio_player.send(AudioCmd::SetQueue(app.display_indices()));
    let _ = audio_player.send(AudioCmd::SetLoopMode(app.loop_mode));
    app.clear_queue_dirty();

    let mut pending_gg = false;

    let run_result: Result<(), Box<dyn std::error::Error>> = (|| {
        let mut last_mpris_index: Option<usize> = None;
        let mut last_mpris_playback: PlaybackState = app.playback;

        loop {
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
            if playback_index_snapshot != last_mpris_index || app.playback != last_mpris_playback {
                update_mpris(&mpris, &app);
                last_mpris_index = playback_index_snapshot;
                last_mpris_playback = app.playback;
            }

            let display = app.display_indices();
            terminal.draw(|f| ui::draw(f, &app, &display))?;

            while let Ok(cmd) = control_rx.try_recv() {
                match cmd {
                    ControlCmd::Quit => {
                        let _ = audio_player.send(AudioCmd::Quit);
                        return Ok(());
                    }
                    ControlCmd::Play => match app.playback {
                        PlaybackState::Paused => {
                            if !app.filter_mode {
                                app.follow_playback_on();
                            }
                            let _ = audio_player.send(AudioCmd::TogglePause);
                            app.playback = PlaybackState::Playing;
                            update_mpris(&mpris, &app);
                        }
                        PlaybackState::Stopped | PlaybackState::Playing => {
                            if app.has_tracks() {
                                if !app.filter_mode {
                                    app.follow_playback_on();
                                }
                                let _ = audio_player.send(AudioCmd::Play(app.selected));
                                app.playback = PlaybackState::Playing;
                                update_mpris(&mpris, &app);
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
                            update_mpris(&mpris, &app);
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
                        update_mpris(&mpris, &app);
                    }
                    ControlCmd::Stop => {
                        if !app.filter_mode {
                            app.follow_playback_on();
                        }
                        let _ = audio_player.send(AudioCmd::Stop);
                        app.playback = PlaybackState::Stopped;
                        update_mpris(&mpris, &app);
                    }
                    ControlCmd::Next => {
                        if app.has_tracks() {
                            if !app.filter_mode {
                                app.follow_playback_on();
                            }
                            let _ = audio_player.send(AudioCmd::Next);
                            app.playback = PlaybackState::Playing;
                            update_mpris(&mpris, &app);
                        }
                    }
                    ControlCmd::Prev => {
                        if app.has_tracks() {
                            if !app.filter_mode {
                                app.follow_playback_on();
                            }
                            let _ = audio_player.send(AudioCmd::Prev);
                            app.playback = PlaybackState::Playing;
                            update_mpris(&mpris, &app);
                        }
                    }
                }
            }

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if app.filter_mode {
                        pending_gg = false;
                        match key.code {
                            KeyCode::Esc => {
                                app.clear_filter();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Backspace => {
                                app.pop_filter_char();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.follow_playback_off();
                                app.next();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.follow_playback_off();
                                app.prev();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char(c) => {
                                // Keep it simple: filter on printable characters.
                                if !c.is_control() {
                                    app.push_filter_char(c);
                                    update_mpris(&mpris, &app);
                                }
                            }
                            KeyCode::Enter => {
                                // If there are no visible results, do nothing.
                                if app.display_indices().is_empty() {
                                    continue;
                                }

                                app.exit_filter_mode();
                                app.follow_playback_on();
                                app.set_pending_follow_index(app.selected);
                                let _ = audio_player.send(AudioCmd::Play(app.selected));
                                app.playback = PlaybackState::Playing;
                                update_mpris(&mpris, &app);
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => {
                                pending_gg = false;
                                let _ = audio_player.send(AudioCmd::Quit);
                                break;
                            }
                            KeyCode::Char('/') => {
                                pending_gg = false;
                                app.enter_filter_mode();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('s') => {
                                pending_gg = false;
                                // toggle shuffle mode
                                let _ = audio_player.send(AudioCmd::ToggleShuffle);
                                app.toggle_shuffle();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('r') => {
                                pending_gg = false;
                                app.cycle_loop_mode();
                                let _ = audio_player.send(AudioCmd::SetLoopMode(app.loop_mode));
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('g') => {
                                if pending_gg {
                                    pending_gg = false;
                                    let display = app.display_indices();
                                    if let Some(&first) = display.first() {
                                        app.set_selected(first);
                                        update_mpris(&mpris, &app);
                                    }
                                } else {
                                    pending_gg = true;
                                }
                            }
                            KeyCode::Char('G') => {
                                pending_gg = false;
                                let display = app.display_indices();
                                if let Some(&last) = display.last() {
                                    app.set_selected(last);
                                    update_mpris(&mpris, &app);
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                pending_gg = false;
                                app.follow_playback_off();
                                app.next();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                pending_gg = false;
                                app.follow_playback_off();
                                app.prev();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Enter => {
                                pending_gg = false;
                                if app.has_tracks() {
                                    app.follow_playback_on();
                                    app.set_pending_follow_index(app.selected);
                                    let _ = audio_player.send(AudioCmd::Play(app.selected));
                                    app.playback = PlaybackState::Playing;
                                    update_mpris(&mpris, &app);
                                }
                            }
                            KeyCode::Char('p') => {
                                pending_gg = false;
                                // Behave like MPRIS PlayPause.
                                let _ = control_tx.send(ControlCmd::PlayPause);
                            }
                            KeyCode::Char(' ') => {
                                pending_gg = false;
                                // Behave like MPRIS PlayPause.
                                let _ = control_tx.send(ControlCmd::PlayPause);
                            }
                            KeyCode::Char('l') => {
                                pending_gg = false;
                                let _ = control_tx.send(ControlCmd::Next);
                            }
                            KeyCode::Char('h') => {
                                pending_gg = false;
                                let _ = control_tx.send(ControlCmd::Prev);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    run_result
}
