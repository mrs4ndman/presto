use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::Sender};

use async_io::{Timer, block_on};
use zbus::{Connection, interface};
use zvariant::{OwnedValue, Value};

use crate::app::PlaybackState;

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

    pub fn set_title(&self, title: Option<String>) {
        if let Ok(mut s) = self.state.lock() {
            s.title = title;
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
        // Minimal metadata so `playerctl metadata` shows something.
        let mut map = HashMap::new();
        let title = self
            .state
            .lock()
            .ok()
            .and_then(|s| s.title.clone())
            .unwrap_or_else(|| "".to_string());

        let title_value = OwnedValue::try_from(Value::from(title)).unwrap_or_else(|_| {
            OwnedValue::try_from(Value::from(String::new())).expect("OwnedValue conversion")
        });

        map.insert("xesam:title".to_string(), title_value);
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

                    let title = state_for_thread
                        .lock()
                        .ok()
                        .and_then(|s| s.title.clone())
                        .unwrap_or_else(|| "".to_string());

                    let playback_status = state_for_thread
                        .lock()
                        .ok()
                        .map(|s| match s.playback {
                            PlaybackState::Stopped => "Stopped".to_string(),
                            PlaybackState::Playing => "Playing".to_string(),
                            PlaybackState::Paused => "Paused".to_string(),
                        })
                        .unwrap_or_else(|| "Stopped".to_string());

                    if let Ok(val) = OwnedValue::try_from(Value::from(playback_status)) {
                        changed.insert("PlaybackStatus".to_string(), val);
                    }

                    // Build Metadata dictionary similar to the `metadata()` property.
                    let mut meta_map: HashMap<String, Value> = HashMap::new();
                    meta_map.insert("xesam:title".to_string(), Value::from(title));
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
