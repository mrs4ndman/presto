use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::{collections::BTreeMap, sync::LazyLock};

use crate::app::App;

static CONTROLS_MAP: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert("j/k".to_string(), "up/down".to_string());
    map.insert("gg/G".to_string(), "top/bottom".to_string());
    map.insert("enter".to_string(), "play selected song".to_string());
    map.insert("space/p".to_string(), "play/pause".to_string());
    map.insert("h/l".to_string(), "prev/next song".to_string());
    map.insert("/".to_string(), "filter".to_string());
    map.insert("s".to_string(), "shuffle".to_string());
    map.insert("r".to_string(), "loop mode".to_string());
    map.insert("q".to_string(), "quit".to_string());
    map
});

static CONTROLS_TEXT: LazyLock<String> = LazyLock::new(|| {
    // Keep the rendered order stable and human-friendly.
    let order = ["j/k", "h/l", "enter", "space/p", "gg/G", "/", "s", "r", "q"];
    order
        .iter()
        .filter_map(|k| CONTROLS_MAP.get(*k).map(|v| format!("[{}] {}", k, v)))
        .collect::<Vec<String>>()
        .join(" | ")
});

pub fn draw(frame: &mut Frame, app: &App, display: &[usize]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
    // Header
    let header = Paragraph::new(" ~ And presto! It's music ~ ")
        .block(Block::default().borders(Borders::ALL).title(" presto "));
    frame.render_widget(header, chunks[0]);

    // Status box
    let status = {
        let mut parts: Vec<String> = Vec::new();

        // cursor mode
        if app.follow_playback {
            parts.push(" CURSOR: Follow".to_string());
        } else {
            parts.push(" CURSOR: Free-roam".to_string());
        }

        // loop mode
        let loop_text = match app.loop_mode {
            crate::audio::LoopMode::NoLoop => "PLAYBACK: No-loop",
            crate::audio::LoopMode::LoopAll => "PLAYBACK: Loop-around",
            crate::audio::LoopMode::LoopOne => "PLAYBACK: Repeat-one",
        };
        parts.push(loop_text.to_string());

        // filter
        let q = app.filter_query.trim();
        if app.filter_mode || !q.is_empty() {
            let mut filter_part = String::from("FILTER:");
            if !q.is_empty() {
                filter_part.push_str(" ");
                filter_part.push_str(q);
            }
            parts.push(filter_part);
        }

        // playback info
        if let Some(ref h) = app.playback_handle {
            if let Ok(info) = h.lock() {
                let state = if info.playing { "Playing" } else { "Paused" };
                if let Some(idx) = info.index {
                    let track = &app.tracks[idx];
                    let duration = track.duration;

                    let elapsed_secs = info.elapsed.as_secs();
                    let elapsed_m = elapsed_secs / 60;
                    let elapsed_s = elapsed_secs % 60;

                    if let Some(dur) = duration {
                        let total_secs = dur.as_secs();
                        let rem_secs = total_secs.saturating_sub(elapsed_secs);
                        parts.push(format!(
                            "Song: {} [{:02}:{:02} / {:02}:{:02} | -{:02}:{:02}]",
                            track.display,
                            elapsed_m,
                            elapsed_s,
                            total_secs / 60,
                            total_secs % 60,
                            rem_secs / 60,
                            rem_secs % 60,
                        ));
                    } else {
                        parts.push(format!(
                            "Song: {} [{:02}:{:02}]",
                            track.display, elapsed_m, elapsed_s,
                        ));
                    }
                    parts.push(state.to_string());
                } else {
                    parts.push("Stopped".to_string());
                }
            }
        }

        // shuffle
        if app.shuffle {
            parts.push("Shuffle: ON".to_string());
        } else {
            parts.push("Shuffle: OFF".to_string());
        }

        // current dir
        if let Some(dir) = &app.current_dir {
            parts.push(format!("Dir: {}", dir));
        }

        parts.join(" • ")
    };

    let status_par = Paragraph::new(status)
        .slow_blink()
        .block(Block::default().borders(Borders::ALL).title(" status "))
        .wrap(Wrap { trim: true });
    frame.render_widget(status_par, chunks[1]);

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

    let footer = Paragraph::new(CONTROLS_TEXT.as_str())
        .block(Block::default().borders(Borders::ALL).title(" controls "))
        .wrap(Wrap { trim: true });

    frame.render_widget(footer, chunks[3]);
}
