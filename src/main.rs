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
    let dir = env::args().nth(1).unwrap_or("music".to_string());

    let tracks = scan(Path::new(&dir));
    let audio_player = AudioPlayer::new(tracks.clone());
    let mut app = App::new(tracks);

    let (control_tx, control_rx) = mpsc::channel::<ControlCmd>();
    let mpris = mpris::spawn_mpris(control_tx.clone());

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let update_mpris = |mpris: &MprisHandle, app: &App| {
        let title = app.tracks.get(app.selected).map(|t| t.title.clone());
        mpris.set_title(title);
        mpris.set_playback(app.playback);
    };

    update_mpris(&mpris, &app);

    let run_result: Result<(), Box<dyn std::error::Error>> = (|| {
        loop {
            terminal.draw(|f| ui::draw(f, &app))?;

            while let Ok(cmd) = control_rx.try_recv() {
                match cmd {
                    ControlCmd::Quit => {
                        let _ = audio_player.send(AudioCmd::Quit);
                        return Ok(());
                    }
                    ControlCmd::Play => match app.playback {
                        PlaybackState::Paused => {
                            let _ = audio_player.send(AudioCmd::TogglePause);
                            app.playback = PlaybackState::Playing;
                            update_mpris(&mpris, &app);
                        }
                        PlaybackState::Stopped | PlaybackState::Playing => {
                            if app.has_tracks() {
                                let _ = audio_player.send(AudioCmd::Play(app.selected));
                                app.playback = PlaybackState::Playing;
                                update_mpris(&mpris, &app);
                            }
                        }
                    },
                    ControlCmd::Pause => {
                        if app.playback == PlaybackState::Playing {
                            let _ = audio_player.send(AudioCmd::TogglePause);
                            app.playback = PlaybackState::Paused;
                            update_mpris(&mpris, &app);
                        }
                    }
                    ControlCmd::PlayPause => {
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
                        let _ = audio_player.send(AudioCmd::Stop);
                        app.playback = PlaybackState::Stopped;
                        update_mpris(&mpris, &app);
                    }
                    ControlCmd::Next => {
                        if app.has_tracks() {
                            app.next();
                            if app.playback == PlaybackState::Stopped {
                                let _ = audio_player.send(AudioCmd::Play(app.selected));
                            } else {
                                let _ = audio_player.send(AudioCmd::Next);
                            }
                            app.playback = PlaybackState::Playing;
                            update_mpris(&mpris, &app);
                        }
                    }
                    ControlCmd::Prev => {
                        if app.has_tracks() {
                            app.prev();
                            if app.playback == PlaybackState::Stopped {
                                let _ = audio_player.send(AudioCmd::Play(app.selected));
                            } else {
                                let _ = audio_player.send(AudioCmd::Prev);
                            }
                            app.playback = PlaybackState::Playing;
                            update_mpris(&mpris, &app);
                        }
                    }
                }
            }

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if app.filter_mode {
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
                                app.next();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
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
                                app.exit_filter_mode();
                                if !app.filtered_indices().is_empty() {
                                    let _ = audio_player.send(AudioCmd::Play(app.selected));
                                    app.playback = PlaybackState::Playing;
                                }
                                update_mpris(&mpris, &app);
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => {
                                let _ = audio_player.send(AudioCmd::Quit);
                                break;
                            }
                            KeyCode::Char('/') => {
                                app.enter_filter_mode();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.next();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.prev();
                                update_mpris(&mpris, &app);
                            }
                            KeyCode::Enter => {
                                if app.has_tracks() {
                                    let _ = audio_player.send(AudioCmd::Play(app.selected));
                                    app.playback = PlaybackState::Playing;
                                    update_mpris(&mpris, &app);
                                }
                            }
                            KeyCode::Char('p') => {
                                // Behave like MPRIS PlayPause.
                                let _ = control_tx.send(ControlCmd::PlayPause);
                            }
                            KeyCode::Char('n') => {
                                let _ = control_tx.send(ControlCmd::Next);
                            }
                            KeyCode::Char('b') => {
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
