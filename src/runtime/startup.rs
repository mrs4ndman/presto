use crate::app::App;
use crate::audio::{AudioCmd, AudioPlayer, LoopMode};
use crate::config;

pub fn apply_playback_defaults(
    app: &mut App,
    audio_player: &AudioPlayer,
    settings: &config::Settings,
) -> Option<Vec<usize>> {
    // Playback defaults
    app.shuffle = settings.playback.shuffle;
    app.loop_mode = match settings.playback.loop_mode {
        config::LoopModeSetting::NoLoop => LoopMode::NoLoop,
        config::LoopModeSetting::LoopAll => LoopMode::LoopAll,
        config::LoopModeSetting::LoopOne => LoopMode::LoopOne,
    };

    // Initialize playback defaults in the audio thread.
    let mut pending_shuffle_reselect_from: Option<Vec<usize>> = None;
    if app.shuffle {
        // When shuffle becomes active, move cursor to top of the randomized order.
        // We snapshot the current order and then wait until the audio thread publishes a new one.
        pending_shuffle_reselect_from = app
            .order_handle
            .as_ref()
            .and_then(|h| h.lock().ok().map(|v| v.clone()))
            .or_else(|| Some((0..app.tracks.len()).collect()));
        let _ = audio_player.send(AudioCmd::ToggleShuffle);
    }

    let _ = audio_player.send(AudioCmd::SetLoopMode(app.loop_mode));
    let _ = audio_player.send(AudioCmd::SetQueue(app.display_indices()));
    app.clear_queue_dirty();

    pending_shuffle_reselect_from
}
