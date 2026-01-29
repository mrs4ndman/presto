use std::{
    fs::File,
    io::BufReader,
    sync::mpsc::{self, Sender},
    thread,
};

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

use crate::library::Track;

#[derive(Debug)]
pub enum AudioCmd {
    Play(usize),
    Stop,
    TogglePause,
    Next,
    Prev,
    Quit,
}

pub struct AudioPlayer {
    tx: Sender<AudioCmd>,
}

impl AudioPlayer {
    pub fn new(tracks: Vec<Track>) -> Self {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        //
        // Spawn audio thread
        let _audio_handle = thread::spawn(move || {
            let stream =
                OutputStreamBuilder::open_default_stream().expect("ERR: No audio output device");
            // rodio logs to stderr when OutputStream is dropped. That's useful in debugging,
            // but noisy for a TUI app.
            let mut stream = stream;
            stream.log_on_drop(false);

            let mut index: Option<usize> = None;

            let mut paused = true;

            let mut sink: Option<Sink> = None;

            let load_track = |i: usize| {
                let track = &tracks[i];
                let sink = create_sink(&stream, track);
                sink
            };

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCmd::Play(i) => {
                        if let Some(ref s) = sink {
                            s.stop();
                        }
                        let new_sink = load_track(i);
                        new_sink.play();
                        sink = Some(new_sink);
                        index = Some(i);
                        paused = false;
                    }

                    AudioCmd::Stop => {
                        if let Some(ref s) = sink {
                            s.stop();
                        }
                        sink = None;
                        paused = true;
                    }

                    AudioCmd::TogglePause => {
                        if let Some(ref s) = sink {
                            if paused {
                                s.play();
                            } else {
                                s.pause();
                            }
                            paused = !paused;
                        }
                    }

                    AudioCmd::Prev => {
                        if tracks.is_empty() {
                            continue;
                        }

                        let i = index.unwrap_or(0);
                        let prev = if i == 0 { tracks.len() - 1 } else { i - 1 };

                        if let Some(ref s) = sink {
                            s.stop();
                        }
                        let new_sink = load_track(prev);
                        new_sink.play();
                        sink = Some(new_sink);
                        index = Some(prev);
                        paused = false;
                    }
                    AudioCmd::Next => {
                        if tracks.is_empty() {
                            continue;
                        }

                        let i = index.unwrap_or(0);
                        let next = (i + 1) % tracks.len();
                        if let Some(ref s) = sink {
                            s.stop();
                        }
                        let new_sink = load_track(next);
                        new_sink.play();
                        sink = Some(new_sink);
                        index = Some(next);
                        paused = false;
                    }
                    AudioCmd::Quit => {
                        if let Some(ref s) = sink {
                            s.stop();
                        }
                        break;
                    }
                }
            }
        });

        Self { tx }
    }

    pub fn send(&self, cmd: AudioCmd) -> Result<(), mpsc::SendError<AudioCmd>> {
        self.tx.send(cmd)
    }
}
fn create_sink(handle: &OutputStream, track: &Track) -> Sink {
    let file =
        File::open(&track.path).unwrap_or_else(|_| panic!("failed to open {:?}", track.path));

    let source = Decoder::new(BufReader::new(file))
        .unwrap_or_else(|_| panic!("failed to decode {:?}", track.path));

    let sink = Sink::connect_new(handle.mixer());
    sink.append(source);
    sink.pause();
    sink
}
