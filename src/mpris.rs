use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::Sender};

use async_io::{Timer, block_on};
use zbus::{Connection, interface};
use zvariant::{ObjectPath, OwnedValue, Value};

use crate::app::PlaybackState;
use crate::library::Track;

#[derive(Clone, Debug)]
pub enum ControlCmd {
    Quit,
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Prev,
}

#[derive(Debug, Default)]
struct SharedState {
    playback: PlaybackState,
    title: Option<String>,
    artist: Vec<String>,
    album: Option<String>,
    url: Option<String>,
    length_micros: Option<i64>,
    track_id: Option<ObjectPath<'static>>,
}

pub struct MprisHandle {
    state: Arc<Mutex<SharedState>>,
    notify: std::sync::mpsc::Sender<()>,
}

impl MprisHandle {
    pub fn set_playback(&self, playback: PlaybackState) {
        if let Ok(mut s) = self.state.lock() {
            s.playback = playback;
            let _ = self.notify.send(());
        }
    }

    pub fn set_track_metadata(&self, idx: Option<usize>, track: Option<&Track>) {
        if let Ok(mut s) = self.state.lock() {
            if let Some(t) = track {
                s.title = Some(t.title.clone());
                s.artist = t.artist.clone().into_iter().collect();
                s.album = t.album.clone();
                s.url = Some(t.path.to_string_lossy().to_string());
                s.length_micros = t
                    .duration
                    .map(|d| (d.as_micros().min(i64::MAX as u128)) as i64);
                s.track_id = idx
                    .and_then(|i| {
                        ObjectPath::try_from(format!("/org/mpris/MediaPlayer2/track/{i}")).ok()
                    })
                    .map(|p| p.to_owned());
            } else {
                s.title = None;
                s.artist.clear();
                s.album = None;
                s.url = None;
                s.length_micros = None;
                s.track_id = None;
            }
            let _ = self.notify.send(());
        }
    }
}

struct RootIface {
    tx: Sender<ControlCmd>,
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl RootIface {
    fn raise(&self) {
        // No-op for TUI.
    }

    fn quit(&self) {
        let _ = self.tx.send(ControlCmd::Quit);
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "presto"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec![]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![]
    }
}

struct PlayerIface {
    tx: Sender<ControlCmd>,
    state: Arc<Mutex<SharedState>>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerIface {
    fn next(&self) {
        let _ = self.tx.send(ControlCmd::Next);
    }

    fn previous(&self) {
        let _ = self.tx.send(ControlCmd::Prev);
    }

    fn play(&self) {
        let _ = self.tx.send(ControlCmd::Play);
    }

    fn pause(&self) {
        let _ = self.tx.send(ControlCmd::Pause);
    }

    fn play_pause(&self) {
        let _ = self.tx.send(ControlCmd::PlayPause);
    }

    fn stop(&self) {
        let _ = self.tx.send(ControlCmd::Stop);
    }

    #[zbus(property)]
    fn playback_status(&self) -> &str {
        // NOTE: This returns a &'static str; we map state into static strings.
        let Ok(s) = self.state.lock() else {
            return "Stopped";
        };
        match s.playback {
            PlaybackState::Stopped => "Stopped",
            PlaybackState::Playing => "Playing",
            PlaybackState::Paused => "Paused",
        }
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        // Minimal-but-useful metadata so `playerctl metadata` shows something.
        let mut map = HashMap::new();

        let Ok(s) = self.state.lock() else {
            return map;
        };

        if let Some(track_id) = s.track_id.clone() {
            if let Ok(v) = OwnedValue::try_from(Value::from(track_id)) {
                map.insert("mpris:trackid".to_string(), v);
            }
        }

        let title = s.title.clone().unwrap_or_default();
        if let Ok(v) = OwnedValue::try_from(Value::from(title)) {
            map.insert("xesam:title".to_string(), v);
        }

        if !s.artist.is_empty() {
            if let Ok(v) = OwnedValue::try_from(Value::from(s.artist.clone())) {
                map.insert("xesam:artist".to_string(), v);
            }
        }

        if let Some(album) = s.album.clone() {
            if let Ok(v) = OwnedValue::try_from(Value::from(album)) {
                map.insert("xesam:album".to_string(), v);
            }
        }

        if let Some(url) = s.url.clone() {
            if let Ok(v) = OwnedValue::try_from(Value::from(url)) {
                map.insert("xesam:url".to_string(), v);
            }
        }

        if let Some(len) = s.length_micros {
            if let Ok(v) = OwnedValue::try_from(Value::from(len)) {
                map.insert("mpris:length".to_string(), v);
            }
        }

        map
    }
}

pub fn spawn_mpris(tx: Sender<ControlCmd>) -> MprisHandle {
    let state = Arc::new(Mutex::new(SharedState::default()));
    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<()>();

    let state_for_thread = state.clone();
    std::thread::spawn(move || {
        block_on(async move {
            let path = "/org/mpris/MediaPlayer2";

            let connection = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("MPRIS: failed to connect to session bus: {e}");
                    return;
                }
            };

            if let Err(e) = connection
                .request_name("org.mpris.MediaPlayer2.presto")
                .await
            {
                eprintln!("MPRIS: failed to acquire name: {e}");
                return;
            }

            let object_server = connection.object_server();

            if let Err(e) = object_server.at(path, RootIface { tx: tx.clone() }).await {
                eprintln!("MPRIS: failed to register root iface: {e}");
                return;
            }

            if let Err(e) = object_server
                .at(
                    path,
                    PlayerIface {
                        tx: tx.clone(),
                        state: state_for_thread.clone(),
                    },
                )
                .await
            {
                eprintln!("MPRIS: failed to register player iface: {e}");
                return;
            }

            // Listen for notifications and emit PropertiesChanged when requested.
            loop {
                // Check for notifications with a short timeout so we stay responsive.
                if let Ok(_) = notify_rx.try_recv() {
                    // Build changed properties map.
                    let mut changed: HashMap<String, OwnedValue> = HashMap::new();

                    let (title, artist, album, url, length_micros, track_id, playback_status) =
                        state_for_thread
                            .lock()
                            .ok()
                            .map(|s| {
                                (
                                    s.title.clone().unwrap_or_default(),
                                    s.artist.clone(),
                                    s.album.clone(),
                                    s.url.clone(),
                                    s.length_micros,
                                    s.track_id.clone(),
                                    match s.playback {
                                        PlaybackState::Stopped => "Stopped".to_string(),
                                        PlaybackState::Playing => "Playing".to_string(),
                                        PlaybackState::Paused => "Paused".to_string(),
                                    },
                                )
                            })
                            .unwrap_or_else(|| {
                                (
                                    String::new(),
                                    Vec::new(),
                                    None,
                                    None,
                                    None,
                                    None,
                                    "Stopped".to_string(),
                                )
                            });

                    if let Ok(val) = OwnedValue::try_from(Value::from(playback_status)) {
                        changed.insert("PlaybackStatus".to_string(), val);
                    }

                    // Build Metadata dictionary similar to the `metadata()` property.
                    let mut meta_map: HashMap<String, Value> = HashMap::new();
                    meta_map.insert("xesam:title".to_string(), Value::from(title));
                    if !artist.is_empty() {
                        meta_map.insert("xesam:artist".to_string(), Value::from(artist));
                    }
                    if let Some(album) = album {
                        meta_map.insert("xesam:album".to_string(), Value::from(album));
                    }
                    if let Some(url) = url {
                        meta_map.insert("xesam:url".to_string(), Value::from(url));
                    }
                    if let Some(len) = length_micros {
                        meta_map.insert("mpris:length".to_string(), Value::from(len));
                    }
                    if let Some(track_id) = track_id {
                        meta_map.insert("mpris:trackid".to_string(), Value::from(track_id));
                    }
                    if let Ok(meta_val) = OwnedValue::try_from(Value::from(meta_map)) {
                        changed.insert("Metadata".to_string(), meta_val);
                    }

                    // Emit PropertiesChanged on the well-known Properties interface.
                    let _ = connection
                        .emit_signal(
                            None::<&str>,
                            path,
                            "org.freedesktop.DBus.Properties",
                            "PropertiesChanged",
                            &(
                                "org.mpris.MediaPlayer2.Player".to_string(),
                                changed,
                                Vec::<String>::new(),
                            ),
                        )
                        .await;
                }

                Timer::after(std::time::Duration::from_millis(250)).await;
            }
        });
    });

    MprisHandle {
        state,
        notify: notify_tx,
    }
}

#[cfg(test)]
mod tests;
