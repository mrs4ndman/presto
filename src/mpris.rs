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
}

impl MprisHandle {
    pub fn set_playback(&self, playback: PlaybackState) {
        if let Ok(mut s) = self.state.lock() {
            s.playback = playback;
        }
    }

    pub fn set_title(&self, title: Option<String>) {
        if let Ok(mut s) = self.state.lock() {
            s.title = title;
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
                        tx,
                        state: state_for_thread,
                    },
                )
                .await
            {
                eprintln!("MPRIS: failed to register player iface: {e}");
                return;
            }

            // Keep the service alive.
            loop {
                Timer::after(std::time::Duration::from_secs(3600)).await;
            }
        });
    });

    MprisHandle { state }
}
