use ratatui::text::Line;
use ratatui::widgets::ListItem;
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};

use crate::app::App;
use crate::config::{TimeField, TrackDisplayField, UiSettings};

static CONTROLS_MAP: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert("j/k".to_string(), "up/down".to_string());
    map.insert("gg/G".to_string(), "top/bottom".to_string());
    map.insert("enter".to_string(), "play selected song".to_string());
    map.insert("space/p".to_string(), "play/pause".to_string());
    map.insert("h/l".to_string(), "prev/next song".to_string());
    map.insert("g?".to_string(), "controls".to_string());
    map.insert("gl".to_string(), "lyrics".to_string());
    // H/L is filled dynamically from config.
    map.insert("-".to_string(), "volume down".to_string());
    map.insert("+".to_string(), "volume up".to_string());
    map.insert("=".to_string(), "volume reset".to_string());
    map.insert("ctrl+e".to_string(), "exit filter input".to_string());
    map.insert("/".to_string(), "filter".to_string());
    map.insert("s".to_string(), "shuffle".to_string());
    map.insert("r".to_string(), "loop mode".to_string());
    map.insert("K".to_string(), "metadata".to_string());
    map.insert("q".to_string(), "quit".to_string());
    map
});

/// Render the controls help text, incorporating scrub seconds.
///
/// Uses a fixed order so the footer stays stable even if the map changes.
pub(crate) fn controls_text(scrub_seconds: u64) -> String {
    // Keep the rendered order stable and human-friendly.
    let order = [
        "j/k", "h/l", "H/L", "-", "+", "=", "enter", "ctrl+e", "space/p", "gg/G", "K", "/", "s",
        "r", "gl", "g?", "q",
    ];
    order
        .iter()
        .filter_map(|k| {
            if *k == "H/L" {
                Some(format!("[H/L] scrub -/+{}s", scrub_seconds))
            } else {
                CONTROLS_MAP.get(*k).map(|v| format!("[{}] {}", k, v))
            }
        })
        .collect::<Vec<String>>()
        .join(" | ")
}

/// Estimate how many wrapped lines `text` will occupy given `max_width`.
///
/// This is a lightweight word-wrapping estimator used to reserve layout space.
pub(crate) fn wrapped_line_count(text: &str, max_width: u16) -> u16 {
    wrap_text_lines(text, max_width)
        .len()
        .try_into()
        .unwrap_or(1)
}

/// Wrap text into lines that fit the given width.
pub(crate) fn wrap_text_lines(text: &str, max_width: u16) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let width = max_width as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_len: usize = 0;

    let mut token = String::new();
    let mut token_is_ws: Option<bool> = None;

    fn flush_current(lines: &mut Vec<String>, current: &mut String, current_len: &mut usize) {
        if !current.is_empty() {
            lines.push(std::mem::take(current));
            *current_len = 0;
        }
    }

    fn push_token(
        lines: &mut Vec<String>,
        current: &mut String,
        current_len: &mut usize,
        token: &str,
        is_ws: bool,
        width: usize,
    ) {
        if token.is_empty() {
            return;
        }

        let token_len = token.chars().count();
        if is_ws && *current_len == 0 {
            return;
        }

        if *current_len + token_len <= width {
            current.push_str(token);
            *current_len += token_len;
            return;
        }

        if is_ws {
            flush_current(lines, current, current_len);
            return;
        }

        if *current_len > 0 {
            flush_current(lines, current, current_len);
        }

        if token_len <= width {
            current.push_str(token);
            *current_len = token_len;
            return;
        }

        let mut chunk = String::new();
        let mut chunk_len: usize = 0;
        for ch in token.chars() {
            chunk.push(ch);
            chunk_len += 1;
            if chunk_len == width {
                lines.push(chunk);
                chunk = String::new();
                chunk_len = 0;
            }
        }

        if !chunk.is_empty() {
            current.push_str(&chunk);
            *current_len = chunk_len;
        }
    }

    for ch in text.chars() {
        if ch == '\n' {
            if let Some(is_ws) = token_is_ws {
                let tok = std::mem::take(&mut token);
                push_token(
                    &mut lines,
                    &mut current,
                    &mut current_len,
                    &tok,
                    is_ws,
                    width,
                );
                token_is_ws = None;
            }
            flush_current(&mut lines, &mut current, &mut current_len);
            lines.push(String::new());
            continue;
        }

        let is_ws = ch.is_whitespace();
        match token_is_ws {
            Some(prev_ws) if prev_ws != is_ws => {
                let tok = std::mem::take(&mut token);
                push_token(
                    &mut lines,
                    &mut current,
                    &mut current_len,
                    &tok,
                    prev_ws,
                    width,
                );
                token.push(ch);
                token_is_ws = Some(is_ws);
            }
            None => {
                token.push(ch);
                token_is_ws = Some(is_ws);
            }
            _ => {
                token.push(ch);
            }
        }
    }

    if let Some(is_ws) = token_is_ws {
        let tok = std::mem::take(&mut token);
        push_token(
            &mut lines,
            &mut current,
            &mut current_len,
            &tok,
            is_ws,
            width,
        );
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// Build a wrapped `ListItem` with an optional prefix (line numbers).
pub(crate) fn list_item_wrapped<T: Into<String>>(
    prefix: &str,
    text: T,
    max_width: u16,
) -> ListItem<'static> {
    let text = if prefix.is_empty() {
        text.into()
    } else {
        format!("{}{}", prefix, text.into())
    };
    if max_width <= 2 {
        return ListItem::new(text);
    }

    let indent_len = prefix.chars().count();
    let indent = " ".repeat(indent_len);
    let first_width = max_width;
    let next_width = max_width.saturating_sub(indent_len as u16).max(1);

    let mut lines = wrap_text_lines(&text, first_width);
    if lines.len() > 1 {
        let mut wrapped: Vec<String> = Vec::new();
        if let Some(first) = lines.first().cloned() {
            wrapped.push(first);
        }
        for line in lines.iter().skip(1) {
            let mut sub_lines = wrap_text_lines(line, next_width);
            for sub in sub_lines.drain(..) {
                wrapped.push(format!("{}{}", indent, sub));
            }
        }
        lines = wrapped;
    }

    let line_items: Vec<Line> = lines.into_iter().map(Line::from).collect();
    ListItem::new(line_items)
}

/// Build the input panel content (filter + count), or none when empty.
pub(crate) fn bottom_info_text(app: &App, ui_settings: &UiSettings) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();

    let q = app.filter_query.trim();
    if app.filter_mode || !q.is_empty() {
        if q.is_empty() {
            lines.push("Filter: (empty)".to_string());
        } else {
            lines.push(format!("Filter: {}", q));
        }
    }

    if ui_settings.show_pending_count {
        if let Some(count) = app.pending_count {
            lines.push(format!("Count: {}", count));
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Build the "now playing" track text according to `ui` settings.
pub(crate) fn now_playing_track_text(app: &App, track_index: usize, ui: &UiSettings) -> String {
    let track = &app.tracks[track_index];
    let mut parts: Vec<String> = Vec::new();

    for f in &ui.now_playing_track_fields {
        match f {
            TrackDisplayField::Display => {
                if !track.display.trim().is_empty() {
                    parts.push(track.display.clone());
                }
            }
            TrackDisplayField::Title => {
                if !track.title.trim().is_empty() {
                    parts.push(track.title.clone());
                }
            }
            TrackDisplayField::Artist => {
                if let Some(a) = track
                    .artist
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    parts.push(a.to_string());
                }
            }
            TrackDisplayField::Album => {
                if let Some(a) = track
                    .album
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    parts.push(a.to_string());
                }
            }
            TrackDisplayField::Filename => {
                if let Some(stem) = track.path.file_stem().and_then(|s| s.to_str()) {
                    if !stem.trim().is_empty() {
                        parts.push(stem.to_string());
                    }
                }
            }
            TrackDisplayField::Path => {
                parts.push(track.path.display().to_string());
            }
        }
    }

    if parts.is_empty() {
        track.display.clone()
    } else {
        parts.join(&ui.now_playing_track_separator)
    }
}

/// Build the now-playing time text (elapsed/total/remaining) per `UiSettings`.
pub(crate) fn now_playing_time_text(
    elapsed: Duration,
    total: Option<Duration>,
    ui: &UiSettings,
) -> Option<String> {
    if ui.now_playing_time_fields.is_empty() {
        return None;
    }

    let mut parts: Vec<String> = Vec::new();
    for f in &ui.now_playing_time_fields {
        match f {
            TimeField::Elapsed => parts.push(format_mmss(elapsed)),
            TimeField::Total => {
                if let Some(t) = total {
                    parts.push(format_mmss(t));
                }
            }
            TimeField::Remaining => {
                if let Some(t) = total {
                    let rem = t.saturating_sub(elapsed);
                    parts.push(format!("-{}", format_mmss(rem)));
                }
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(&ui.now_playing_time_separator))
    }
}

/// Build the status line text combining playback and volume data.
pub(crate) fn status_text(app: &App, ui_settings: &UiSettings) -> String {
    let mut parts: Vec<String> = Vec::new();

    let state = if app.filter_mode {
        "FILTER"
    } else if app.follow_playback {
        "FOLLOWING_PLAYING"
    } else {
        "NAVIGATION"
    };
    parts.push(format!("STATE: {}", state));

    if app.follow_playback {
        parts.push("CURSOR: Follow".to_string());
    } else {
        parts.push("CURSOR: Free-roam".to_string());
    }

    let loop_text = match app.loop_mode {
        crate::audio::LoopMode::NoLoop => "PLAYBACK: No-loop",
        crate::audio::LoopMode::LoopAll => "PLAYBACK: Loop-around",
        crate::audio::LoopMode::LoopOne => "PLAYBACK: Repeat-one",
    };
    parts.push(loop_text.to_string());

    if let Some(ref h) = app.playback_handle {
        if let Ok(info) = h.lock() {
            let state = if info.playing { "Playing" } else { "Paused" };
            if let Some(idx) = info.index {
                let track = &app.tracks[idx];
                let song = now_playing_track_text(app, idx, ui_settings);
                let time = now_playing_time_text(info.elapsed, track.duration, ui_settings);
                if let Some(time) = time {
                    parts.push(format!("Song: {} [{}]", song, time));
                } else {
                    parts.push(format!("Song: {}", song));
                }
                parts.push(state.to_string());
            } else {
                parts.push("Stopped".to_string());
            }
        }
    }

    if app.shuffle {
        parts.push("Shuffle: ON".to_string());
    } else {
        parts.push("Shuffle: OFF".to_string());
    }

    parts.push(format!("Vol: {}%", app.volume_percent()));

    if let Some(dir) = &app.current_dir {
        parts.push(format!("Dir: {}", dir));
    }

    if let Some(notice) = &app.notice {
        parts.push(format!("Notice: {}", notice));
    }

    parts.push("Help: g?".to_string());

    parts.join(" • ")
}

/// Format an optional duration, rounding up partial seconds, showing total seconds.
fn format_duration_mmss_ceil(d: Option<Duration>) -> String {
    let Some(d) = d else {
        return "-".to_string();
    };

    let mut total_secs = d.as_secs();
    if d.subsec_nanos() > 0 {
        total_secs = total_secs.saturating_add(1);
    }

    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{}:{:02} ({}s)", minutes, seconds, total_secs)
}

/// Build the metadata pane content for the currently selected track.
pub(crate) fn metadata_text(app: &App) -> String {
    let selected = app.tracks.get(app.selected);
    if let Some(track) = selected {
        let dur = format_duration_mmss_ceil(track.duration);
        format!(
            "Selected Track\n\nTitle: {}\nArtist: {}\nAlbum: {}\nDuration: {}\nPath: {}",
            track.title,
            track.artist.as_deref().unwrap_or("-"),
            track.album.as_deref().unwrap_or("-"),
            dur,
            track.path.display()
        )
    } else {
        "Selected Track\n\nNo track selected".to_string()
    }
}

/// Format a `Duration` as `MM:SS`.
fn format_mmss(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::{bottom_info_text, wrap_text_lines};
    use crate::{app::App, config::UiSettings};

    #[test]
    fn wrap_text_preserves_multiple_spaces() {
        let lines = wrap_text_lines("Artist  Title", 50);
        assert_eq!(lines, vec!["Artist  Title".to_string()]);
    }

    #[test]
    fn bottom_info_hides_count_when_disabled() {
        let mut app = App::new(Vec::new());
        let mut ui = UiSettings::default();
        app.pending_count = Some(12);
        ui.show_pending_count = false;

        assert_eq!(bottom_info_text(&app, &ui), None);
    }

    #[test]
    fn bottom_info_keeps_filter_when_count_is_disabled() {
        let mut app = App::new(Vec::new());
        let mut ui = UiSettings::default();
        app.filter_mode = true;
        app.filter_query = "black sabbath".to_string();
        app.pending_count = Some(12);
        ui.show_pending_count = false;

        assert_eq!(
            bottom_info_text(&app, &ui),
            Some("Filter: black sabbath".to_string())
        );
    }
}
