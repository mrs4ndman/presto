use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rand::seq::SliceRandom;
use rand::thread_rng;
use rodio::{OutputStreamBuilder, Sink};

use crate::config::AudioSettings;
use crate::library::Track;

use super::queue::reorder_queue_in_place;
use super::sink::create_sink_at;
use super::types::{AudioCmd, LoopMode, OrderHandle, PlaybackHandle};

pub(super) fn spawn_audio_thread(
    tracks: Vec<Track>,
    rx: Receiver<AudioCmd>,
    playback_info: PlaybackHandle,
    order_handle: OrderHandle,
    audio_settings: AudioSettings,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let stream = OutputStreamBuilder::open_default_stream().expect("ERR: No audio output device");
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
        let info_for_ticker_clone = playback_info.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(500));
            let mut info = info_for_ticker_clone.lock().unwrap();
            if info.playing {
                info.elapsed = info.elapsed + Duration::from_millis(500);
            }
        });

        fn do_play(
            i: usize,
            stream: &rodio::OutputStream,
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
            audio_settings: &AudioSettings,
        ) {
            let crossfade_ms = audio_settings.crossfade_ms;
            let crossfade_steps = audio_settings.crossfade_steps.max(1);

            let track = &tracks[i];
            let new_sink = create_sink_at(stream, track, Duration::ZERO);
            // Keep the default volume sane even if crossfade is disabled.
            new_sink.set_volume(1.0);

            // Crossfade if currently playing a sink; otherwise just swap.
            if let Some(old_sink) = sink.as_ref() {
                if !*paused {
                    if crossfade_ms == 0 {
                        // Crossfade disabled: hard swap.
                        old_sink.stop();
                    } else {
                        old_sink.set_volume(1.0);
                        new_sink.set_volume(0.0);
                        new_sink.play();

                        // Fade volumes in a short blocking loop. This is simple and good enough
                        // for a TUI player; audio continues in rodio's mixer thread.
                        for step in 1..=crossfade_steps {
                            let t = (step as f32) / (crossfade_steps as f32);
                            old_sink.set_volume(1.0 - t);
                            new_sink.set_volume(t);
                            thread::sleep(Duration::from_millis(
                                (crossfade_ms / crossfade_steps).max(1),
                            ));
                        }

                        old_sink.stop();
                    }
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
                        if let Ok(mut info) = playback_info.lock() {
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
                            &playback_info,
                            &queue,
                            &mut queue_pos,
                            shuffle,
                            &order,
                            &mut order_pos,
                            &audio_settings,
                        );
                    }

                    AudioCmd::Stop => {
                        do_stop(
                            &mut sink,
                            &mut index,
                            &mut paused,
                            &mut started_at,
                            &mut accumulated,
                            &playback_info,
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
                                if let Ok(mut info) = playback_info.lock() {
                                    info.playing = true;
                                }
                            } else {
                                // pausing
                                if let Some(st) = started_at {
                                    accumulated += Instant::now() - st;
                                }
                                started_at = None;
                                if let Ok(mut info) = playback_info.lock() {
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
                        if let Ok(mut oh) = order_handle.lock() {
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
                                    &playback_info,
                                    &queue,
                                    &mut queue_pos,
                                    shuffle,
                                    &order,
                                    &mut order_pos,
                                    &audio_settings,
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
                                &playback_info,
                                &queue,
                                &mut queue_pos,
                                shuffle,
                                &order,
                                &mut order_pos,
                                &audio_settings,
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
                                    &playback_info,
                                    &queue,
                                    &mut queue_pos,
                                    shuffle,
                                    &order,
                                    &mut order_pos,
                                    &audio_settings,
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
                                &playback_info,
                                &queue,
                                &mut queue_pos,
                                shuffle,
                                &order,
                                &mut order_pos,
                                &audio_settings,
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
                        if let Ok(mut info) = playback_info.lock() {
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
                                            &playback_info,
                                            &queue,
                                            &mut queue_pos,
                                            shuffle,
                                            &order,
                                            &mut order_pos,
                                            &audio_settings,
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
                                            &playback_info,
                                            &queue,
                                            &mut queue_pos,
                                            shuffle,
                                            &order,
                                            &mut order_pos,
                                            &audio_settings,
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
                                                &playback_info,
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
                                                &playback_info,
                                                &queue,
                                                &mut queue_pos,
                                                shuffle,
                                                &order,
                                                &mut order_pos,
                                                &audio_settings,
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
    })
}
