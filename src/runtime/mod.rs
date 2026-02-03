use std::env;
use std::path::Path;
use std::sync::mpsc;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::app::App;
use crate::audio::AudioPlayer;
use crate::library::scan;
use crate::mpris::ControlCmd;

mod event_loop;
mod mpris_sync;
mod settings;
mod startup;

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

    let (control_tx, control_rx) = mpsc::channel::<ControlCmd>();
    let mpris = crate::mpris::spawn_mpris(control_tx.clone());

    mpris_sync::update_mpris(&mpris, &app);

    let pending_shuffle_reselect_from =
        startup::apply_playback_defaults(&mut app, &audio_player, &settings);

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

    run_result
}
