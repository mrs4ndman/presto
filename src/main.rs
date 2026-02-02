mod app;
mod audio;
mod config;
mod library;
mod mpris;
mod ui;
mod runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    runtime::run()
}
