use std::{fs::File, io::BufReader, sync::mpsc, thread};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rodio::{Decoder, OutputStreamBuilder, Sink};

enum AudioCmd {
    PlayPause,
    Stop,
    Quit,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CHANGE THIS PATH
    let audio_path = "/home/mrsandman/Music/peak/ACDC - Back In Black.mp3";

    // Channel to audio thread
    let (tx, rx) = mpsc::channel::<AudioCmd>();

    // Spawn audio thread
    let audio_handle = thread::spawn(move || {
        let mut stream =
            OutputStreamBuilder::open_default_stream().expect("no audio output device");

        let sink = Sink::connect_new(stream.mixer());

        let file = File::open(audio_path).expect("failed to open audio file");
        let source = Decoder::new(BufReader::new(file)).expect("failed to decode audio");

        sink.append(source);
        sink.pause(); // start paused

        let mut paused = true;

        while let Ok(cmd) = rx.recv() {
            match cmd {
                AudioCmd::PlayPause => {
                    if paused {
                        sink.play();
                    } else {
                        sink.pause();
                    }
                    paused = !paused;
                }
                AudioCmd::Stop => {
                    sink.stop();
                    break;
                }
                AudioCmd::Quit => {
                    sink.stop();
                    break;
                }
            }
        }
    });

    // Keyboard handling
    enable_raw_mode()?;
    println!("Instructions: p = play/pause | q = quit");

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('p') => {
                    tx.send(AudioCmd::PlayPause)?;
                }
                KeyCode::Char('q') => {
                    tx.send(AudioCmd::Quit)?;
                    break;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    audio_handle.join().unwrap();

    Ok(())
}
