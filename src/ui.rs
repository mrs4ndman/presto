//! UI rendering helpers for the terminal user interface.
//!
//! This module contains functions to render the TUI using `ratatui`.

use ratatui::text::Line;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};

use crate::app::App;
use crate::config::{ControlsSettings, TimeField, TrackDisplayField, UiSettings};

static CONTROLS_MAP: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert("j/k".to_string(), "up/down".to_string());
    map.insert("gg/G".to_string(), "top/bottom".to_string());
    map.insert("enter".to_string(), "play selected song".to_string());
    map.insert("space/p".to_string(), "play/pause".to_string());
    map.insert("h/l".to_string(), "prev/next song".to_string());
    map.insert("g?".to_string(), "controls".to_string());
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
fn controls_text(scrub_seconds: u64) -> String {
    // Keep the rendered order stable and human-friendly.
    let order = [
        "j/k", "h/l", "H/L", "-", "+", "=", "enter", "ctrl+e", "space/p", "gg/G", "K", "/", "s",
        "r", "g?", "q",
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
fn wrapped_line_count(text: &str, max_width: u16) -> u16 {
    wrap_text_lines(text, max_width)
        .len()
        .try_into()
        .unwrap_or(1)
}

/// Wrap text into lines that fit the given width.
fn wrap_text_lines(text: &str, max_width: u16) -> Vec<String> {
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
fn list_item_wrapped<T: Into<String>>(prefix: &str, text: T, max_width: u16) -> ListItem<'static> {
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

/// Format a `Duration` as `MM:SS`.
fn format_mmss(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// Build the status line text combining playback and volume data.
fn status_text(app: &App, ui_settings: &UiSettings) -> String {
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

#[cfg(test)]
mod tests {
    use super::wrap_text_lines;

    #[test]
    fn wrap_text_preserves_multiple_spaces() {
        let lines = wrap_text_lines("Artist  Title", 50);
        assert_eq!(lines, vec!["Artist  Title".to_string()]);
    }
}

/// Build the input panel content (filter + count), or none when empty.
fn bottom_info_text(app: &App) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();

    let q = app.filter_query.trim();
    if app.filter_mode || !q.is_empty() {
        if q.is_empty() {
            lines.push("Filter: (empty)".to_string());
        } else {
            lines.push(format!("Filter: {}", q));
        }
    }

    if let Some(count) = app.pending_count {
        lines.push(format!("Count: {}", count));
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Build the "now playing" track text according to `ui` settings.
fn now_playing_track_text(app: &App, track_index: usize, ui: &UiSettings) -> String {
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
fn now_playing_time_text(
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

/// Compute a centered rectangle with given size constrained to `r`.
fn centered_rect_sized(mut width: u16, mut height: u16, r: Rect) -> Rect {
    // Keep the popup smaller and avoid covering the entire UI.
    width = width.min(r.width.saturating_sub(2)).max(10);
    height = height.min(r.height.saturating_sub(2)).max(5);

    let x = r.x + (r.width.saturating_sub(width) / 2);
    let y = r.y + (r.height.saturating_sub(height) / 2);
    Rect {
        x,
        y,
        width,
        height,
    }
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

/// Render the entire UI into the provided `frame` using `app` state and settings.
pub fn draw(
    frame: &mut Frame,
    app: &App,
    display: &[usize],
    ui_settings: &UiSettings,
    controls_settings: &ControlsSettings,
) {
    let bottom_text = bottom_info_text(app);
    let bottom_height = if let Some(ref text) = bottom_text {
        let bottom_content_width = frame.area().width.saturating_sub(3).max(1);
        wrapped_line_count(text, bottom_content_width)
            .saturating_add(2)
            .max(3)
    } else {
        0
    };

    let status_text_val = status_text(app, ui_settings);
    let status_content_width = frame.area().width.saturating_sub(3).max(1);
    let status_height = wrapped_line_count(&status_text_val, status_content_width)
        .saturating_add(2)
        .max(3);

    let chunks = if bottom_height == 0 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(status_height),
                Constraint::Min(1),
            ])
            .split(frame.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(status_height),
                Constraint::Min(1),
                Constraint::Length(bottom_height),
            ])
            .split(frame.area())
    };

    // Header
    let header = Paragraph::new(ui_settings.header_text.as_str())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" presto ")
                .title_alignment(Alignment::Center),
        );
    frame.render_widget(header, chunks[0]);

    // Status box (wrap-aware)
    let status_par = Paragraph::new(status_text_val)
        .slow_blink()
        .block(
            Block::bordered()
                .padding(Padding {
                    left: 1,
                    right: 0,
                    top: 0,
                    bottom: 0,
                })
                .title(" status "),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(status_par, chunks[1]);

    // Main list (and optional metadata pane)
    let mut list_area = chunks[2];
    let mut meta_area: Option<Rect> = None;
    if app.metadata_window {
        let meta_width = (list_area.width * 2 / 5).clamp(32, 60);
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(20),
                Constraint::Length(1),
                Constraint::Length(meta_width),
            ])
            .split(list_area);
        list_area = panes[0];
        meta_area = Some(panes[2]);
    }

    {
        let q = app.filter_query.trim();
        let query_lower = if q.is_empty() {
            None
        } else if app.uses_lower_titles() {
            Some(q.to_ascii_lowercase())
        } else {
            None
        };

        // Center the selected item when possible by creating a visible window.
        // Important: only build ListItems for the visible window (avoid allocating the entire list).
        let total = display.len();
        let list_height = list_area.height as usize;
        let sel_pos = display.iter().position(|&i| i == app.selected).unwrap_or(0);
        let (start, end, selected_pos_in_visible) = if total <= list_height || list_height == 0 {
            (0, total, sel_pos)
        } else {
            let half = list_height / 2;
            let mut start = if sel_pos > half { sel_pos - half } else { 0 };
            if start + list_height > total {
                start = total - list_height;
            }
            (start, start + list_height, sel_pos - start)
        };

        let item_width = list_area.width.saturating_sub(4).max(1);
        let show_relative = ui_settings.show_relative_numbers;
        let show_current = ui_settings.show_current_line_number;
        let number_width = total.to_string().len().max(1);

        let visible_items: Vec<ListItem> = display[start..end]
            .iter()
            .enumerate()
            .map(|(offset, &i)| {
                let absolute_pos = start + offset;
                let prefix = if show_relative || show_current {
                    if show_relative && show_current {
                        let val = if absolute_pos == sel_pos {
                            absolute_pos + 1
                        } else if absolute_pos >= sel_pos {
                            absolute_pos - sel_pos
                        } else {
                            sel_pos - absolute_pos
                        };
                        format!("{:>width$} ", val, width = number_width)
                    } else if show_relative {
                        if absolute_pos == sel_pos {
                            " ".repeat(number_width + 1)
                        } else {
                            let val = if absolute_pos >= sel_pos {
                                absolute_pos - sel_pos
                            } else {
                                sel_pos - absolute_pos
                            };
                            format!("{:>width$} ", val, width = number_width)
                        }
                    } else {
                        let val = absolute_pos + 1;
                        format!("{:>width$} ", val, width = number_width)
                    }
                } else {
                    String::new()
                };
                let title = &app.tracks[i].display;
                if q.is_empty() {
                    list_item_wrapped(&prefix, title.as_str(), item_width)
                } else {
                    let positions = match query_lower.as_deref() {
                        Some(ql) => app.fuzzy_match_positions_for_track_lower(i, ql),
                        None => App::fuzzy_match_positions(title, q),
                    };

                    if let Some(positions) = positions {
                        let mut rendered = String::new();
                        let mut pos_iter = positions.into_iter();
                        let mut next_pos = pos_iter.next();

                        for (ci, ch) in title.chars().enumerate() {
                            if next_pos == Some(ci) {
                                for up in ch.to_uppercase() {
                                    rendered.push(up);
                                }
                                next_pos = pos_iter.next();
                            } else {
                                rendered.push(ch);
                            }
                        }
                        list_item_wrapped(&prefix, rendered, item_width)
                    } else {
                        list_item_wrapped(&prefix, title.as_str(), item_width)
                    }
                }
            })
            .collect();

        let list = List::new(visible_items)
            .block(Block::default().borders(Borders::ALL).title(" tracks "))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        let mut state = ratatui::widgets::ListState::default();
        if total > 0 {
            state.select(Some(selected_pos_in_visible));
        }
        frame.render_stateful_widget(list, list_area, &mut state);
    }

    // Metadata side pane
    if let Some(meta_area) = meta_area {
        let track = app.tracks.get(app.selected);
        let meta = if let Some(track) = track {
            let dur = format_duration_mmss_ceil(track.duration);
            format!(
                "Title: {}\nArtist: {}\nAlbum: {}\nDuration: {}\nPath: {}",
                track.title,
                track.artist.as_deref().unwrap_or("-"),
                track.album.as_deref().unwrap_or("-"),
                dur,
                track.path.display()
            )
        } else {
            "No track selected".to_string()
        };
        let meta_paragraph = Paragraph::new(meta)
            .block(
                Block::default()
                    .padding(Padding {
                        left: 1,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    })
                    .borders(Borders::ALL)
                    .title(" metadata (K closes) "),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(meta_paragraph, meta_area);
    }

    // Controls popup (g?)
    if app.controls_popup {
        let popup_text = controls_text(controls_settings.scrub_seconds);
        let max_width = frame.area().width.saturating_sub(4).max(30);
        let popup_width = ((max_width as u32 * 2) / 3) as u16;
        let popup_width = popup_width.clamp(30, 80).min(max_width);
        let popup_content_width = popup_width.saturating_sub(4).max(1);
        let popup_height = wrapped_line_count(&popup_text, popup_content_width)
            .saturating_add(4)
            .max(7);
        let popup_area = centered_rect_sized(popup_width, popup_height, frame.area());
        frame.render_widget(Clear, popup_area);

        let popup = Paragraph::new(popup_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" controls (g? closes) ")
                    .padding(Padding {
                        left: 1,
                        right: 1,
                        top: 1,
                        bottom: 1,
                    }),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(popup, popup_area);
    }

    if let Some(text) = bottom_text {
        let bottom_panel = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" input ")
                    .padding(Padding {
                        left: 1,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(bottom_panel, chunks[3]);
    }
}
