use std::{
    fs::File,
    io::BufReader,
    sync::mpsc::{self, Sender},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use rand::seq::SliceRandom;
use rand::thread_rng;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

use crate::library::Track;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LoopMode {
    /// Do not wrap at the end of the current queue.
    NoLoop,
    /// Wrap around to the start of the current queue.
    LoopAll,
    /// Repeat the current song when it ends.
    LoopOne,
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::NoLoop
    }
}

#[derive(Debug)]
pub enum AudioCmd {
    Play(usize),
    Stop,
    TogglePause,
    ToggleShuffle,
    SetQueue(Vec<usize>),
    SetLoopMode(LoopMode),
    Next,
    Prev,
    Quit,
}

pub struct AudioPlayer {
    tx: Sender<AudioCmd>,
    playback: PlaybackHandle,
    order: OrderHandle,
}

impl AudioPlayer {
    pub fn playback_handle(&self) -> PlaybackHandle {
        self.playback.clone()
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackInfo {
    pub index: Option<usize>,
    pub elapsed: Duration,
    pub playing: bool,
}

impl Default for PlaybackInfo {
    fn default() -> Self {
        Self {
            index: None,
            elapsed: Duration::ZERO,
            playing: false,
        }
    }
}

pub type PlaybackHandle = Arc<Mutex<PlaybackInfo>>;
pub type OrderHandle = Arc<Mutex<Vec<usize>>>;

impl AudioPlayer {
    pub fn new(tracks: Vec<Track>) -> Self {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        let playback_info: PlaybackHandle = Arc::new(Mutex::new(PlaybackInfo::default()));
        let order_handle: OrderHandle = Arc::new(Mutex::new((0..tracks.len()).collect()));
        //
        // Spawn audio thread
        let playback_info_for_thread = playback_info.clone();
        let order_handle_for_thread = order_handle.clone();
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

            // Track start time and accumulated elapsed when paused.
            let mut started_at: Option<Instant> = None;
            let mut accumulated = Duration::ZERO;

            // Shuffle/order state
            let mut shuffle = false;
            let mut order: Vec<usize> = (0..tracks.len()).collect();
            let mut order_pos: usize = 0;

            // Current playback queue (usually the visible list: filtered/unfiltered + shuffle order).
            let mut queue: Vec<usize> = (0..tracks.len()).collect();
            let mut queue_pos: usize = 0;

            let mut loop_mode: LoopMode = LoopMode::default();

            // Spawn a ticker thread to update playback_info.elapsed periodically.
            let info_for_ticker_clone = playback_info_for_thread.clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(500));
                    let mut info = info_for_ticker_clone.lock().unwrap();
                    if info.playing {
                        info.elapsed = info.elapsed + Duration::from_millis(500);
                    }
                }
            });

            fn do_play(
                i: usize,
                stream: &OutputStream,
                tracks: &Vec<Track>,
                sink: &mut Option<Sink>,
                index: &mut Option<usize>,
                paused: &mut bool,
                started_at: &mut Option<Instant>,
                accumulated: &mut Duration,
                playback_info: &PlaybackHandle,
                queue: &Vec<usize>,
                queue_pos: &mut usize,
                shuffle: bool,
                order: &Vec<usize>,
                order_pos: &mut usize,
            ) {
                if let Some(s) = sink.as_ref() {
                    s.stop();
                }

                let track = &tracks[i];
                let new_sink = create_sink(stream, track);
                new_sink.play();
                *sink = Some(new_sink);
                *index = Some(i);
                *paused = false;
                *started_at = Some(Instant::now());
                *accumulated = Duration::ZERO;

                if let Some(pos) = queue.iter().position(|&x| x == i) {
                    *queue_pos = pos;
                }
                if shuffle {
                    if let Some(pos) = order.iter().position(|&x| x == i) {
                        *order_pos = pos;
                    }
                }

                if let Ok(mut info) = playback_info.lock() {
                    info.index = Some(i);
                    info.elapsed = Duration::ZERO;
                    info.playing = true;
                }
            }

            fn do_stop(
                sink: &mut Option<Sink>,
                index: &mut Option<usize>,
                paused: &mut bool,
                started_at: &mut Option<Instant>,
                accumulated: &mut Duration,
                playback_info: &PlaybackHandle,
            ) {
                if let Some(s) = sink.as_ref() {
                    s.stop();
                }
                *sink = None;
                *index = None;
                *paused = true;
                *started_at = None;
                *accumulated = Duration::ZERO;
                if let Ok(mut info) = playback_info.lock() {
                    info.index = None;
                    info.elapsed = Duration::ZERO;
                    info.playing = false;
                }
            }

            // Keep a reference to playback_info inside this thread so we can update it.
            let _playback_info_thread = playback_info_for_thread.clone();

            use std::sync::mpsc::RecvTimeoutError;
            loop {
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(cmd) => match cmd {
                        AudioCmd::Play(i) => {
                            // Ensure queue_pos points at the played index if present.
                            if let Some(pos) = queue.iter().position(|&x| x == i) {
                                queue_pos = pos;
                            } else {
                                queue = vec![i];
                                queue_pos = 0;
                            }
                            do_play(
                                i,
                                &stream,
                                &tracks,
                                &mut sink,
                                &mut index,
                                &mut paused,
                                &mut started_at,
                                &mut accumulated,
                                &playback_info_for_thread,
                                &queue,
                                &mut queue_pos,
                                shuffle,
                                &order,
                                &mut order_pos,
                            );
                        }

                        AudioCmd::Stop => {
                            do_stop(
                                &mut sink,
                                &mut index,
                                &mut paused,
                                &mut started_at,
                                &mut accumulated,
                                &playback_info_for_thread,
                            );
                        }

                        AudioCmd::TogglePause => {
                            if let Some(ref s) = sink {
                                if paused {
                                    s.play();
                                } else {
                                    s.pause();
                                }
                                if paused {
                                    // unpausing
                                    started_at = Some(Instant::now());
                                    if let Ok(mut info) = playback_info_for_thread.lock() {
                                        info.playing = true;
                                    }
                                } else {
                                    // pausing
                                    if let Some(st) = started_at {
                                        accumulated += Instant::now() - st;
                                    }
                                    started_at = None;
                                    if let Ok(mut info) = playback_info_for_thread.lock() {
                                        info.playing = false;
                                    }
                                }
                                paused = !paused;
                            }
                        }

                        AudioCmd::ToggleShuffle => {
                            shuffle = !shuffle;
                            if shuffle {
                                order.shuffle(&mut thread_rng());
                            } else {
                                order = (0..tracks.len()).collect();
                            }
                            // update shared order handle so UI can read current order
                            if let Ok(mut oh) = order_handle_for_thread.lock() {
                                *oh = order.clone();
                            }
                            if let Some(i) = index {
                                if let Some(pos) = order.iter().position(|&x| x == i) {
                                    order_pos = pos;
                                }
                            }
                        }

                        AudioCmd::SetQueue(mut new_queue) => {
                            // If the caller sends an empty queue (e.g. filter has no matches),
                            // just store it; auto-advance/next/prev will become no-ops.
                            // Keep it sane by removing out-of-range indices.
                            new_queue.retain(|&i| i < tracks.len());
                            queue = new_queue;
                            if let Some(i) = index {
                                if let Some(pos) = queue.iter().position(|&x| x == i) {
                                    queue_pos = pos;
                                } else {
                                    queue_pos = 0;
                                }
                            } else {
                                queue_pos = 0;
                            }
                        }

                        AudioCmd::SetLoopMode(m) => {
                            loop_mode = m;
                        }

                        AudioCmd::Prev => {
                            if tracks.is_empty() || queue.is_empty() {
                                continue;
                            }

                            // Manual prev respects LoopAll wrap, but does not repeat-one.
                            let cur_pos = if index.is_some() { queue_pos } else { 0 };

                            if cur_pos == 0 {
                                if loop_mode == LoopMode::LoopAll {
                                    queue_pos = queue.len() - 1;
                                    do_play(
                                        queue[queue_pos],
                                        &stream,
                                        &tracks,
                                        &mut sink,
                                        &mut index,
                                        &mut paused,
                                        &mut started_at,
                                        &mut accumulated,
                                        &playback_info_for_thread,
                                        &queue,
                                        &mut queue_pos,
                                        shuffle,
                                        &order,
                                        &mut order_pos,
                                    );
                                }
                                // NoLoop: do nothing
                            } else {
                                queue_pos -= 1;
                                do_play(
                                    queue[queue_pos],
                                    &stream,
                                    &tracks,
                                    &mut sink,
                                    &mut index,
                                    &mut paused,
                                    &mut started_at,
                                    &mut accumulated,
                                    &playback_info_for_thread,
                                    &queue,
                                    &mut queue_pos,
                                    shuffle,
                                    &order,
                                    &mut order_pos,
                                );
                            }
                        }
                        AudioCmd::Next => {
                            if tracks.is_empty() || queue.is_empty() {
                                continue;
                            }

                            // Manual next respects LoopAll wrap, but does not repeat-one.
                            let cur_pos = if index.is_some() { queue_pos } else { 0 };

                            if cur_pos + 1 >= queue.len() {
                                if loop_mode == LoopMode::LoopAll {
                                    queue_pos = 0;
                                    do_play(
                                        queue[queue_pos],
                                        &stream,
                                        &tracks,
                                        &mut sink,
                                        &mut index,
                                        &mut paused,
                                        &mut started_at,
                                        &mut accumulated,
                                        &playback_info_for_thread,
                                        &queue,
                                        &mut queue_pos,
                                        shuffle,
                                        &order,
                                        &mut order_pos,
                                    );
                                }
                                // NoLoop: do nothing
                            } else {
                                queue_pos += 1;
                                do_play(
                                    queue[queue_pos],
                                    &stream,
                                    &tracks,
                                    &mut sink,
                                    &mut index,
                                    &mut paused,
                                    &mut started_at,
                                    &mut accumulated,
                                    &playback_info_for_thread,
                                    &queue,
                                    &mut queue_pos,
                                    shuffle,
                                    &order,
                                    &mut order_pos,
                                );
                            }
                        }
                        AudioCmd::Quit => {
                            if let Some(ref s) = sink {
                                s.stop();
                            }
                            break;
                        }
                    },
                    Err(RecvTimeoutError::Timeout) => {
                        // periodic check for auto-advance
                        if let Some(ref s) = sink {
                            if !paused && s.empty() {
                                match loop_mode {
                                    LoopMode::LoopOne => {
                                        if let Some(i) = index {
                                            do_play(
                                                i,
                                                &stream,
                                                &tracks,
                                                &mut sink,
                                                &mut index,
                                                &mut paused,
                                                &mut started_at,
                                                &mut accumulated,
                                                &playback_info_for_thread,
                                                &queue,
                                                &mut queue_pos,
                                                shuffle,
                                                &order,
                                                &mut order_pos,
                                            );
                                        }
                                    }
                                    LoopMode::LoopAll => {
                                        if !queue.is_empty() {
                                            if queue_pos + 1 >= queue.len() {
                                                queue_pos = 0;
                                            } else {
                                                queue_pos += 1;
                                            }
                                            do_play(
                                                queue[queue_pos],
                                                &stream,
                                                &tracks,
                                                &mut sink,
                                                &mut index,
                                                &mut paused,
                                                &mut started_at,
                                                &mut accumulated,
                                                &playback_info_for_thread,
                                                &queue,
                                                &mut queue_pos,
                                                shuffle,
                                                &order,
                                                &mut order_pos,
                                            );
                                        }
                                    }
                                    LoopMode::NoLoop => {
                                        if !queue.is_empty() {
                                            if queue_pos + 1 >= queue.len() {
                                                do_stop(
                                                    &mut sink,
                                                    &mut index,
                                                    &mut paused,
                                                    &mut started_at,
                                                    &mut accumulated,
                                                    &playback_info_for_thread,
                                                );
                                            } else {
                                                queue_pos += 1;
                                                do_play(
                                                    queue[queue_pos],
                                                    &stream,
                                                    &tracks,
                                                    &mut sink,
                                                    &mut index,
                                                    &mut paused,
                                                    &mut started_at,
                                                    &mut accumulated,
                                                    &playback_info_for_thread,
                                                    &queue,
                                                    &mut queue_pos,
                                                    shuffle,
                                                    &order,
                                                    &mut order_pos,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        // Return the AudioPlayer with the playback and order handles so callers can read elapsed/time and order.
        Self {
            tx,
            playback: playback_info,
            order: order_handle,
        }
    }

    pub fn order_handle(&self) -> OrderHandle {
        self.order.clone()
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
