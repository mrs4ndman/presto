//! UI rendering helpers for the terminal user interface.
//!
//! This module contains functions to render the TUI using `ratatui`.

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
fn controls_text(scrub_seconds: u64) -> String {
    // Keep the rendered order stable and human-friendly.
    let order = [
        "j/k", "h/l", "H/L", "-", "+", "=", "enter", "ctrl+e", "space/p", "gg/G", "K", "/",
        "s", "r", "q",
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
fn wrapped_line_count(text: &str, max_width: u16) -> u16 {
    if text.is_empty() || max_width == 0 {
        return 1;
    }

    let mut lines: u16 = 1;
    let mut line_width: u16 = 0;

    for word in text.split_whitespace() {
        let word_width = word.chars().count() as u16;
        if line_width == 0 {
            lines += word_width.saturating_sub(1) / max_width;
            line_width = word_width % max_width;
            if line_width == 0 {
                line_width = max_width.min(word_width);
            }
        } else if line_width + 1 + word_width <= max_width {
            line_width += 1 + word_width;
        } else {
            lines += 1;
            lines += word_width.saturating_sub(1) / max_width;
            line_width = word_width % max_width;
            if line_width == 0 {
                line_width = max_width.min(word_width);
            }
        }
    }

    lines
}

/// Format a `Duration` as `MM:SS`.
fn format_mmss(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// Build the status line text.
fn status_text(app: &App, ui_settings: &UiSettings) -> String {
    let mut parts: Vec<String> = Vec::new();

    if app.follow_playback {
        parts.push(" CURSOR: Follow".to_string());
    } else {
        parts.push(" CURSOR: Free-roam".to_string());
    }

    let loop_text = match app.loop_mode {
        crate::audio::LoopMode::NoLoop => "PLAYBACK: No-loop",
        crate::audio::LoopMode::LoopAll => "PLAYBACK: Loop-around",
        crate::audio::LoopMode::LoopOne => "PLAYBACK: Repeat-one",
    };
    parts.push(loop_text.to_string());

    let q = app.filter_query.trim();
    if app.filter_mode || !q.is_empty() {
        let mut filter_part = String::from("FILTER:");
        if !q.is_empty() {
            filter_part.push_str(" ");
            filter_part.push_str(q);
        }
        parts.push(filter_part);
    }

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

    parts.join(" • ")
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
    let footer_text = controls_text(controls_settings.scrub_seconds);
    let footer_content_width = frame.area().width.saturating_sub(3).max(1);
    let footer_height = wrapped_line_count(&footer_text, footer_content_width)
        .saturating_add(2)
        .max(3);

    let status_text_val = status_text(app, ui_settings);
    let status_content_width = frame.area().width.saturating_sub(3).max(1);
    let status_height = wrapped_line_count(&status_text_val, status_content_width)
        .saturating_add(2)
        .max(3);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(status_height),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(frame.area());

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

    // Main list
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
        let list_height = chunks[2].height as usize;
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

        let visible_items: Vec<ListItem> = display[start..end]
            .iter()
            .map(|&i| {
                let title = &app.tracks[i].display;
                if q.is_empty() {
                    ListItem::new(title.as_str())
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
                        ListItem::new(rendered)
                    } else {
                        ListItem::new(title.as_str())
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
        frame.render_stateful_widget(list, chunks[2], &mut state);
    }

    // Overlay metadata popup (keeps list visible under it)
    if app.metadata_window {
        // Keep the popup inside the list area so it doesn't cover header/status/footer.
        let list_area = chunks[2];
        let popup_area = centered_rect_sized(72, 9, list_area);
        frame.render_widget(Clear, popup_area);

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
        frame.render_widget(meta_paragraph, popup_area);
    }

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" controls ")
                .padding(Padding {
                    left: 1,
                    right: 0,
                    top: 0,
                    bottom: 0,
                }),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(footer, chunks[3]);
}
