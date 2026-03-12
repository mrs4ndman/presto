use std::{env, path::PathBuf};

use super::schema::Settings;

/// Configuration loading helpers.
///
/// `Settings::load` tries environment variables first (prefix `PRESTO__`), then an
/// optional config file and falls back to struct defaults.
impl Settings {
    /// Load settings from environment and optional config file.
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

    /// Perform basic validation checks on loaded settings.
    pub fn validate(&self) -> Result<(), String> {
        let mut errors: Vec<String> = Vec::new();

        if self.audio.crossfade_steps == 0 {
            errors.push("audio.crossfade_steps must be >= 1".to_string());
        }
        if self.audio.initial_volume_percent > 100 {
            errors.push("audio.initial_volume_percent must be between 0 and 100".to_string());
        }
        if self.controls.scrub_seconds == 0 {
            errors.push("controls.scrub_seconds must be >= 1".to_string());
        }
        if self.controls.scrub_batch_window_ms > i32::MAX as u64 {
            errors.push("controls.scrub_batch_window_ms must be <= 2147483647".to_string());
        }
        if self.controls.volume_step_percent == 0 {
            errors.push("controls.volume_step_percent must be >= 1".to_string());
        }
        if self.controls.volume_step_percent > 100 {
            errors.push("controls.volume_step_percent must be <= 100".to_string());
        }
        if let Some(depth) = self.library.max_depth {
            if depth == 0 {
                errors.push("library.max_depth must be >= 1".to_string());
            }
        }

        let trimmed_exts: Vec<&str> = self
            .library
            .extensions
            .iter()
            .map(|e| e.trim().trim_start_matches('.'))
            .filter(|e| !e.is_empty())
            .collect();

        if trimmed_exts.is_empty() {
            errors.push(
                "library.extensions must include at least one non-empty extension".to_string(),
            );
        } else if trimmed_exts.len() != self.library.extensions.len() {
            errors.push("library.extensions must not contain empty entries".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("invalid config:\n- {}", errors.join("\n- ")))
        }
    }
}

/// Resolve the config path from `PRESTO_CONFIG_PATH` or XDG defaults.
pub fn resolve_config_path() -> Option<PathBuf> {
    if let Some(p) = env::var_os("PRESTO_CONFIG_PATH") {
        let p = PathBuf::from(p);
        return Some(p);
    }
    default_config_path()
}

/// Compute the default config path under `$XDG_CONFIG_HOME/presto/config.toml`
/// or `~/.config/presto/config.toml` when `XDG_CONFIG_HOME` is not set.
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
