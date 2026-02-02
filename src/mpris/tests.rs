use super::*;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

fn make_track() -> Track {
    Track {
        path: PathBuf::from("/tmp/music/test.mp3"),
        title: "Test Title".to_string(),
        artist: Some("Test Artist".to_string()),
        album: Some("Test Album".to_string()),
        duration: Some(Duration::from_micros(1_234_567)),
        display: "Test Artist - Test Title".to_string(),
    }
}

#[test]
fn set_track_metadata_sets_and_clears_shared_state() {
    let state = Arc::new(Mutex::new(SharedState::default()));
    let (notify_tx, _notify_rx) = mpsc::channel::<()>();
    let handle = MprisHandle {
        state: state.clone(),
        notify: notify_tx,
    };

    let track = make_track();
    handle.set_track_metadata(Some(7), Some(&track));

    {
        let s = state.lock().unwrap();
        assert_eq!(s.title.as_deref(), Some("Test Title"));
        assert_eq!(s.artist, vec!["Test Artist".to_string()]);
        assert_eq!(s.album.as_deref(), Some("Test Album"));
        assert!(s.url.as_deref().unwrap().contains("/tmp/music/test.mp3"));
        assert_eq!(s.length_micros, Some(1_234_567));
        assert_eq!(
            s.track_id.as_ref().map(|p| p.as_str()),
            Some("/org/mpris/MediaPlayer2/track/7")
        );
    }

    handle.set_track_metadata(None, None);
    {
        let s = state.lock().unwrap();
        assert_eq!(s.title, None);
        assert!(s.artist.is_empty());
        assert_eq!(s.album, None);
        assert_eq!(s.url, None);
        assert_eq!(s.length_micros, None);
        assert!(s.track_id.is_none());
    }
}

#[test]
fn playback_status_maps_state_to_spec_strings() {
    let state = Arc::new(Mutex::new(SharedState::default()));
    let (tx, _rx) = mpsc::channel::<ControlCmd>();
    let iface = PlayerIface {
        tx,
        state: state.clone(),
    };

    {
        let mut s = state.lock().unwrap();
        s.playback = PlaybackState::Stopped;
    }
    assert_eq!(iface.playback_status(), "Stopped");

    {
        let mut s = state.lock().unwrap();
        s.playback = PlaybackState::Playing;
    }
    assert_eq!(iface.playback_status(), "Playing");

    {
        let mut s = state.lock().unwrap();
        s.playback = PlaybackState::Paused;
    }
    assert_eq!(iface.playback_status(), "Paused");
}

#[test]
fn metadata_includes_expected_keys_when_present() {
    let state = Arc::new(Mutex::new(SharedState::default()));
    let (tx, _rx) = mpsc::channel::<ControlCmd>();
    let iface = PlayerIface {
        tx,
        state: state.clone(),
    };

    {
        let mut s = state.lock().unwrap();
        s.title = Some("Title".to_string());
        s.artist = vec!["Artist".to_string()];
        s.album = Some("Album".to_string());
        s.url = Some("file:///tmp/test.mp3".to_string());
        s.length_micros = Some(42);
        s.track_id = ObjectPath::try_from("/org/mpris/MediaPlayer2/track/1")
            .ok()
            .map(|p| p.to_owned());
    }

    let map = iface.metadata();
    for k in [
        "mpris:trackid",
        "xesam:title",
        "xesam:artist",
        "xesam:album",
        "xesam:url",
        "mpris:length",
    ] {
        assert!(map.contains_key(k), "missing key: {k}");
    }
}
