use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::config::AudioSettings;
use crate::library::Track;

use super::thread::spawn_audio_thread;
use super::types::{AudioCmd, OrderHandle, PlaybackHandle, PlaybackInfo};

pub struct AudioPlayer {
    tx: Sender<AudioCmd>,
    playback: PlaybackHandle,
    order: OrderHandle,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl AudioPlayer {
    pub fn new(tracks: Vec<Track>, audio_settings: AudioSettings) -> Self {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        let playback_info: PlaybackHandle = Arc::new(Mutex::new(PlaybackInfo::default()));
        let order_handle: OrderHandle = Arc::new(Mutex::new((0..tracks.len()).collect()));

        let audio_handle = spawn_audio_thread(
            tracks,
            rx,
            playback_info.clone(),
            order_handle.clone(),
            audio_settings,
        );

        Self {
            tx,
            playback: playback_info,
            order: order_handle,
            join: Mutex::new(Some(audio_handle)),
        }
    }

    pub fn playback_handle(&self) -> PlaybackHandle {
        self.playback.clone()
    }

    pub fn order_handle(&self) -> OrderHandle {
        self.order.clone()
    }

    pub fn send(&self, cmd: AudioCmd) -> Result<(), mpsc::SendError<AudioCmd>> {
        self.tx.send(cmd)
    }

    pub fn quit_softly(&self, fade_out: Duration) {
        let _ = self.send(AudioCmd::Quit {
            fade_out_ms: fade_out.as_millis() as u64,
        });

        if let Ok(mut j) = self.join.lock() {
            if let Some(h) = j.take() {
                let _ = h.join();
            }
        }
    }
}
