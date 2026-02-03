use crate::app::App;
use crate::mpris::MprisHandle;

pub fn update_mpris(mpris: &MprisHandle, app: &App) {
    let now_playing_idx = if let Some(ref handle) = app.playback_handle {
        handle.lock().ok().and_then(|info| info.index)
    } else {
        None
    };

    let track = now_playing_idx.and_then(|i| app.tracks.get(i));
    mpris.set_track_metadata(now_playing_idx, track);
    mpris.set_playback(app.playback);
}
