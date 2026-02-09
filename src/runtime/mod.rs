use std::env;
use std::path::Path;
use std::sync::mpsc;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::app::App;
use crate::audio::{AudioCmd, AudioPlayer};
use crate::library::scan;
use crate::mpris::ControlCmd;

mod event_loop;
mod mpris_sync;
mod settings;
mod startup;
mod state;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let settings = settings::load_settings();

    let dir = env::args().nth(1).unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Music".to_string())
    });

    let tracks = scan(Path::new(&dir), &settings.library);
    let audio_player = AudioPlayer::new(tracks.clone(), settings.audio.clone());
    let mut app = App::new(tracks);

    app.follow_playback = settings.ui.follow_playback;
    app.set_current_dir(dir.clone());
    app.set_playback_handle(audio_player.playback_handle());
    app.set_order_handle(audio_player.order_handle());
    app.set_initial_volume_percent(settings.audio.initial_volume_percent);

    let store = state::StateStore::new_default();
    let persisted_state = match store.load_directory_state(&dir) {
        Ok(state) => state,
        Err(err) => {
            app.set_notice(format!("State load failed: {}", err));
            eprintln!(
                "presto: state_load_failed path=\"{}\" error=\"{}\"",
                err.path().display(),
                err
            );
            None
        }
    };
    state::apply_filter_and_selection(&mut app, persisted_state.as_ref());

    app.shuffle = persisted_state
        .as_ref()
        .and_then(|s| s.shuffle)
        .unwrap_or(settings.playback.shuffle);

    app.loop_mode = persisted_state
        .as_ref()
        .and_then(|s| s.loop_mode)
        .unwrap_or(match settings.playback.loop_mode {
            crate::config::LoopModeSetting::NoLoop => crate::audio::LoopMode::NoLoop,
            crate::config::LoopModeSetting::LoopAll => crate::audio::LoopMode::LoopAll,
            crate::config::LoopModeSetting::LoopOne => crate::audio::LoopMode::LoopOne,
        });

    if let Some(fp) = persisted_state.as_ref().and_then(|s| s.follow_playback) {
        if fp {
            app.follow_playback_on();
        } else {
            app.follow_playback_off();
        }
    }

    let _ = audio_player.send(AudioCmd::SetVolume(app.volume()));

    let (control_tx, control_rx) = mpsc::channel::<ControlCmd>();
    let mpris = crate::mpris::spawn_mpris(control_tx.clone());

    mpris_sync::update_mpris(&mpris, &app);

    let pending_shuffle_reselect_from = startup::apply_playback_defaults(&mut app, &audio_player);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let run_result: Result<(), Box<dyn std::error::Error>> = (|| {
        let mut state = event_loop::EventLoopState::new(&app);
        state.pending_shuffle_reselect_from = pending_shuffle_reselect_from;

        event_loop::run(
            &mut terminal,
            &settings,
            &mut app,
            &audio_player,
            &mpris,
            &control_tx,
            &control_rx,
            &mut state,
        )
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = store.persist_directory_state(&dir, &app) {
        eprintln!(
            "presto: state_persist_failed path=\"{}\" error=\"{}\"",
            e.path().display(),
            e
        );
    }

    run_result
}
