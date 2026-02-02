use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use rodio::{Decoder, OutputStream, Sink, Source};

use crate::library::Track;

pub(super) fn create_sink_at(handle: &OutputStream, track: &Track, start_at: Duration) -> Sink {
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
