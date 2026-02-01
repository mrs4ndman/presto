use std::{
    fs::File,
    io::BufReader,
    sync::mpsc::{self, Sender},
    sync::{Arc, Mutex},
    thread,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use rand::seq::SliceRandom;
use rand::thread_rng;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

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
    Quit { fade_out_ms: u64 },
    SeekBy(i32), // seconds, positive or negative
}

pub struct AudioPlayer {
    tx: Sender<AudioCmd>,
    playback: PlaybackHandle,
    order: OrderHandle,
    join: Mutex<Option<JoinHandle<()>>>,
}

fn reorder_queue_in_place(
    queue: &mut Vec<usize>,
    tracks_len: usize,
    shuffle: bool,
    order: &[usize],
) {
    queue.retain(|&i| i < tracks_len);
    if !shuffle {
        queue.sort_unstable();
        return;
    }

    let mut pos_map = vec![usize::MAX; tracks_len];
    for (p, &ti) in order.iter().enumerate() {
        if ti < pos_map.len() {
            pos_map[ti] = p;
        }
    }
    queue.sort_by_key(|&ti| pos_map.get(ti).copied().unwrap_or(usize::MAX));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_queue_unshuffled_sorts_and_filters() {
        let mut q = vec![5, 2, 999, 2, 0];
        reorder_queue_in_place(&mut q, 6, false, &[]);
        assert_eq!(q, vec![0, 2, 2, 5]);
    }

    #[test]
    fn reorder_queue_shuffled_follows_order_positions() {
        // order position: 3->0, 1->1, 0->2, 2->3
        let order = vec![3, 1, 0, 2];
        let mut q = vec![0, 3, 2];
        reorder_queue_in_place(&mut q, 4, true, &order);
        assert_eq!(q, vec![3, 0, 2]);
    }
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
        let audio_handle = thread::spawn(move || {
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
                const CROSSFADE_MS: u64 = 250;
                const CROSSFADE_STEPS: u64 = 10;

                let track = &tracks[i];
                let new_sink = create_sink_at(stream, track, Duration::ZERO);

                // Crossfade if currently playing a sink; otherwise just swap.
                if let Some(old_sink) = sink.as_ref() {
                    if !*paused {
                        old_sink.set_volume(1.0);
                        new_sink.set_volume(0.0);
                        new_sink.play();

                        // Fade volumes in a short blocking loop. This is simple and good enough
                        // for a TUI player; audio continues in rodio's mixer thread.
                        for step in 1..=CROSSFADE_STEPS {
                            let t = (step as f32) / (CROSSFADE_STEPS as f32);
                            old_sink.set_volume(1.0 - t);
                            new_sink.set_volume(t);
                            thread::sleep(Duration::from_millis(CROSSFADE_MS / CROSSFADE_STEPS));
                        }

                        old_sink.stop();
                    } else {
                        old_sink.stop();
                    }
                }

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

            fn fade_out_sink(sink: &Sink, fade_out_ms: u64) {
                if fade_out_ms == 0 {
                    sink.set_volume(0.0);
                    return;
                }
                let steps: u64 = 20;
                let step_ms = (fade_out_ms / steps).max(1);
                sink.set_volume(1.0);
                for step in 1..=steps {
                    let t = step as f32 / steps as f32;
                    sink.set_volume(1.0 - t);
                    thread::sleep(Duration::from_millis(step_ms));
                }
                sink.set_volume(0.0);
            }

            loop {
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(cmd) => match cmd {
                        AudioCmd::SeekBy(secs) => {
                            // Scrubbing: rebuild the current sink and skip into the file.
                            // This uses `Source::skip_duration` (works for common formats).
                            let Some(i) = index else {
                                continue;
                            };
                            if sink.is_none() {
                                continue;
                            }

                            let elapsed =
                                accumulated + started_at.map_or(Duration::ZERO, |st| st.elapsed());
                            let cur = elapsed.as_secs() as i64;
                            let new = (cur + secs as i64).max(0) as u64;
                            let new_elapsed = Duration::from_secs(new);

                            // Stop old sink and replace with a fresh one.
                            if let Some(s) = sink.as_ref() {
                                s.stop();
                            }

                            let track = &tracks[i];
                            let new_sink = create_sink_at(&stream, track, new_elapsed);
                            if paused {
                                new_sink.pause();
                                started_at = None;
                            } else {
                                new_sink.play();
                                started_at = Some(Instant::now());
                            }

                            sink = Some(new_sink);
                            accumulated = new_elapsed;
                            if let Ok(mut info) = playback_info_for_thread.lock() {
                                info.elapsed = new_elapsed;
                            }
                        }
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

                            // Keep the actual playback queue in sync with shuffle state.
                            // We do NOT change queue membership here (that is controlled via SetQueue);
                            // we only reorder the existing queue to match the current shuffled/unshuffled order.
                            if !queue.is_empty() {
                                let current = index;
                                reorder_queue_in_place(&mut queue, tracks.len(), shuffle, &order);

                                if let Some(i) = current {
                                    queue_pos = queue.iter().position(|&x| x == i).unwrap_or(0);
                                } else {
                                    queue_pos = 0;
                                }
                            }
                        }

                        AudioCmd::SetQueue(mut new_queue) => {
                            // If the caller sends an empty queue (e.g. filter has no matches),
                            // just store it; auto-advance/next/prev will become no-ops.
                            // Keep it sane by removing out-of-range indices.
                            new_queue.retain(|&i| i < tracks.len());

                            // IMPORTANT: always order the queue according to the audio thread's
                            // current shuffle order. This prevents a race where the UI computes
                            // display_indices() using a stale order_handle immediately after
                            // toggling shuffle and then overwrites the correct shuffled queue.
                            reorder_queue_in_place(&mut new_queue, tracks.len(), shuffle, &order);

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
                        AudioCmd::Quit { fade_out_ms } => {
                            if let Some(ref s) = sink {
                                // Fade out gently before stopping.
                                fade_out_sink(s, fade_out_ms);
                                s.stop();
                            }
                            // Update shared state so UI/MPRIS don't keep showing Playing.
                            if let Ok(mut info) = playback_info_for_thread.lock() {
                                info.playing = false;
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
            join: Mutex::new(Some(audio_handle)),
        }
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
fn create_sink_at(handle: &OutputStream, track: &Track, start_at: Duration) -> Sink {
    let file =
        File::open(&track.path).unwrap_or_else(|_| panic!("failed to open {:?}", track.path));

    let source = Decoder::new(BufReader::new(file))
        .unwrap_or_else(|_| panic!("failed to decode {:?}", track.path))
        // `skip_duration` is our seeking primitive; even Duration::ZERO is fine.
        .skip_duration(start_at);

    let sink = Sink::connect_new(handle.mixer());
    sink.append(source);
    sink.pause();
    sink
}
