use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
    // Header
    let header = Paragraph::new("~ And presto! It's music ~")
        .block(Block::default().borders(Borders::ALL).title(" presto "));
    frame.render_widget(header, chunks[0]);

    // Status box
    let status = {
        let mut parts: Vec<String> = Vec::new();

        // cursor mode
        if app.follow_playback {
            parts.push("CURSOR: Follow".to_string());
        } else {
            parts.push("CURSOR: Free-roam".to_string());
        }

        // loop mode
        let loop_text = match app.loop_mode {
            crate::audio::LoopMode::NoLoop => "Playback: No-loop",
            crate::audio::LoopMode::LoopAll => "Playback: Loop",
            crate::audio::LoopMode::LoopOne => "Playback: Repeat-one",
        };
        parts.push(loop_text.to_string());

        // filter
        let q = app.filter_query.trim();
        if !q.is_empty() {
            parts.push(format!("Filter: {}", q));
        }

        // playback info
        if let Some(ref h) = app.playback_handle {
            if let Ok(info) = h.lock() {
                let state = if info.playing { "Playing" } else { "Paused" };
                if let Some(idx) = info.index {
                    let t = &app.tracks[idx].title;
                    parts.push(format!(
                        "{} [{:02}:{:02}]",
                        t,
                        info.elapsed.as_secs() / 60,
                        info.elapsed.as_secs() % 60
                    ));
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
        .block(Block::default().borders(Borders::ALL).title(" status "))
        .wrap(Wrap { trim: true });
    frame.render_widget(status_par, chunks[1]);

    let display = app.display_indices();
    let items_all: Vec<ListItem> = display
        .iter()
        .map(|&i| {
            let title = app.tracks[i].title.clone();
            if app.filter_query.is_empty() {
                ListItem::new(title)
            } else {
                if let Some(positions) = App::fuzzy_match_positions(&title, &app.filter_query) {
                    let mut rendered = String::new();
                    for (ci, ch) in title.chars().enumerate() {
                        if positions.contains(&ci) {
                            for up in ch.to_uppercase() {
                                rendered.push(up);
                            }
                        } else {
                            rendered.push(ch);
                        }
                    }
                    ListItem::new(rendered)
                } else {
                    ListItem::new(title)
                }
            }
        })
        .collect();

    // Center the selected item when possible by creating a visible window
    let total = items_all.len();
    let list_height = chunks[2].height as usize;
    let (visible_items, selected_pos_in_visible) = if total <= list_height || list_height == 0 {
        (
            items_all,
            display.iter().position(|&i| i == app.selected).unwrap_or(0),
        )
    } else {
        let sel_pos = display.iter().position(|&i| i == app.selected).unwrap_or(0);
        let half = list_height / 2;
        let mut start = if sel_pos > half { sel_pos - half } else { 0 };
        if start + list_height > total {
            start = total - list_height;
        }
        let slice: Vec<ListItem> = items_all[start..start + list_height].to_vec();
        (slice, sel_pos - start)
    };

    let list = List::new(visible_items)
        .block(Block::default().borders(Borders::ALL).title(" Tracks "))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ratatui::widgets::ListState::default();
    if total > 0 {
        state.select(Some(selected_pos_in_visible));
    }
    frame.render_stateful_widget(list, chunks[2], &mut state);

    let controls = "[j/k] move • [enter] play selected song • [p] play/pause • [n/b] next/prev • [/] filter • [s] shuffle • [l] loop mode • [q] quit";

    let footer = Paragraph::new(controls)
        .block(Block::default().borders(Borders::ALL).title(" controls "))
        .wrap(Wrap { trim: true });

    frame.render_widget(footer, chunks[3]);
}
