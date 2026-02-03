use std::{env, path::PathBuf};

use super::schema::Settings;

impl Settings {
    pub fn load() -> Result<Self, ::config::ConfigError> {
        let config_path = resolve_config_path();

        let mut builder = ::config::Config::builder();

        if let Some(path) = &config_path {
            builder = builder.add_source(::config::File::from(path.as_path()).required(false));
        }

        builder = builder.add_source(
            ::config::Environment::with_prefix("PRESTO")
                .separator("__")
                .try_parsing(true),
        );

        let cfg = builder.build()?;
        let settings: Settings = cfg.try_deserialize()?;
        Ok(settings)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.audio.crossfade_steps == 0 {
            return Err("audio.crossfade_steps must be >= 1".to_string());
        }
        Ok(())
    }
}

pub fn resolve_config_path() -> Option<PathBuf> {
    if let Some(p) = env::var_os("PRESTO_CONFIG_PATH") {
        let p = PathBuf::from(p);
        return Some(p);
    }
    default_config_path()
}

pub fn default_config_path() -> Option<PathBuf> {
    let config_home = if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(xdg))
    } else if let Some(home) = env::var_os("HOME") {
        Some(PathBuf::from(home).join(".config"))
    } else {
        None
    };

    config_home.map(|d| d.join("presto").join("config.toml"))
}
