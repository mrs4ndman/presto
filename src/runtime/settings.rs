use crate::config;

pub fn load_settings() -> config::Settings {
    match config::Settings::load() {
        Ok(s) => {
            if let Err(msg) = s.validate() {
                eprintln!("presto: invalid config, using defaults: {msg}");
                config::Settings::default()
            } else {
                s
            }
        }
        Err(e) => {
            // Config is optional; failures should not prevent the app from starting.
            eprintln!("presto: failed to load config, using defaults: {e}");
            config::Settings::default()
        }
    }
}
