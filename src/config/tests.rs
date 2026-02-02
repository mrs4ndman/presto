use super::load::{default_config_path, resolve_config_path};
use super::schema::*;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

struct EnvGuard {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, val: &str) -> Self {
        let old = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, val);
        }
        Self { key, old }
    }

    fn remove(key: &'static str) -> Self {
        let old = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.old.take() {
            Some(v) => unsafe {
                std::env::set_var(self.key, v);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}

#[test]
fn resolve_config_path_prefers_presto_config_path() {
    let _lock = env_lock();
    let _g1 = EnvGuard::set("PRESTO_CONFIG_PATH", "/tmp/presto-test-config.toml");
    assert_eq!(
        resolve_config_path().unwrap(),
        std::path::PathBuf::from("/tmp/presto-test-config.toml")
    );
}

#[test]
fn default_config_path_prefers_xdg_config_home() {
    let _lock = env_lock();
    let _g1 = EnvGuard::set("XDG_CONFIG_HOME", "/tmp/xdg-config-home");
    let _g2 = EnvGuard::set("HOME", "/tmp/home-should-not-win");

    let p = default_config_path().unwrap();
    assert_eq!(
        p,
        std::path::PathBuf::from("/tmp/xdg-config-home")
            .join("presto")
            .join("config.toml")
    );
}

#[test]
fn default_config_path_falls_back_to_home_dot_config() {
    let _lock = env_lock();
    let _g1 = EnvGuard::remove("XDG_CONFIG_HOME");
    let _g2 = EnvGuard::set("HOME", "/tmp/home-dir");

    let p = default_config_path().unwrap();
    assert_eq!(
        p,
        std::path::PathBuf::from("/tmp/home-dir")
            .join(".config")
            .join("presto")
            .join("config.toml")
    );
}

#[test]
fn settings_load_from_config_file_and_parse_loop_mode_aliases() {
    let _lock = env_lock();

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    std::fs::write(
        &cfg_path,
        r#"
[playback]
shuffle = true
loop_mode = "repeat-one"

[audio]
crossfade_ms = 0
crossfade_steps = 3
quit_fade_out_ms = 123

[controls]
scrub_seconds = 9

[ui]
follow_playback = false
header_text = "hello"
now_playing_track_fields = ["artist", "title"]
now_playing_track_separator = " • "
now_playing_time_fields = ["elapsed", "remaining"]
now_playing_time_separator = " | "

[library]
extensions = ["mp3"]
recursive = false
include_hidden = false
follow_links = false
display_fields = ["filename"]
display_separator = "::"
"#,
    )
    .unwrap();

    let _g1 = EnvGuard::set("PRESTO_CONFIG_PATH", cfg_path.to_str().unwrap());
    let _g2 = EnvGuard::remove("PRESTO__AUDIO__CROSSFADE_MS");

    let s = Settings::load().unwrap();
    assert!(s.playback.shuffle);
    assert!(matches!(s.playback.loop_mode, LoopModeSetting::LoopOne));
    assert_eq!(s.audio.crossfade_ms, 0);
    assert_eq!(s.audio.crossfade_steps, 3);
    assert_eq!(s.audio.quit_fade_out_ms, 123);
    assert_eq!(s.controls.scrub_seconds, 9);
    assert!(!s.ui.follow_playback);
    assert_eq!(s.ui.header_text, "hello");
    assert_eq!(s.ui.now_playing_track_fields.len(), 2);
    assert!(matches!(s.ui.now_playing_track_fields[0], TrackDisplayField::Artist));
    assert!(matches!(s.ui.now_playing_track_fields[1], TrackDisplayField::Title));
    assert_eq!(s.ui.now_playing_track_separator, " • ");
    assert_eq!(s.ui.now_playing_time_fields.len(), 2);
    assert!(matches!(s.ui.now_playing_time_fields[0], TimeField::Elapsed));
    assert!(matches!(s.ui.now_playing_time_fields[1], TimeField::Remaining));
    assert_eq!(s.ui.now_playing_time_separator, " | ");
    assert_eq!(s.library.extensions, vec!["mp3".to_string()]);
    assert!(!s.library.recursive);
    assert!(!s.library.include_hidden);
    assert!(!s.library.follow_links);
    assert_eq!(s.library.display_separator, "::");
    assert!(matches!(s.library.display_fields[0], TrackDisplayField::Filename));
}

#[test]
fn settings_env_overrides_config_file() {
    let _lock = env_lock();

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    std::fs::write(
        &cfg_path,
        r#"
[audio]
crossfade_ms = 250
"#,
    )
    .unwrap();

    let _g1 = EnvGuard::set("PRESTO_CONFIG_PATH", cfg_path.to_str().unwrap());
    let _g2 = EnvGuard::set("PRESTO__AUDIO__CROSSFADE_MS", "0");

    let s = Settings::load().unwrap();
    assert_eq!(s.audio.crossfade_ms, 0);
}
