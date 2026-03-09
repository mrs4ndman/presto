use ratatui::text::Text;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
};

use crate::app::App;
use crate::config::{ControlsSettings, UiSettings};

use super::layout::centered_rect_sized;
use super::lyrics::lyrics_text;
use super::text::{controls_text, list_item_wrapped, wrapped_line_count};

/// Render the top header banner.
pub(crate) fn render_header(frame: &mut Frame, area: Rect, ui_settings: &UiSettings) {
    let header = Paragraph::new(ui_settings.header_text.as_str())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" presto ")
                .title_alignment(Alignment::Center),
        );
    frame.render_widget(header, area);
}

/// Render the status panel directly under the header.
pub(crate) fn render_status(frame: &mut Frame, area: Rect, status_text: String) {
    let status_par = Paragraph::new(status_text)
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
    frame.render_widget(status_par, area);
}

/// Render the track list, including fuzzy-match highlighting and line-number prefixes.
///
/// Only the visible slice is materialized to keep allocations predictable on large libraries.
pub(crate) fn render_track_list(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    display: &[usize],
    ui_settings: &UiSettings,
) {
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
    let list_height = area.height as usize;
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

    let item_width = area.width.saturating_sub(4).max(1);
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
    frame.render_stateful_widget(list, area, &mut state);
}

/// Render the metadata side pane for the currently selected track.
pub(crate) fn render_metadata_pane(frame: &mut Frame, area: Rect, metadata_text: String) {
    let meta_paragraph = Paragraph::new(metadata_text)
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
    frame.render_widget(meta_paragraph, area);
}

/// Render the lyrics side pane for the current playback context.
pub(crate) fn render_lyrics_pane(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    ui_settings: &UiSettings,
) {
    let content_height = area.height.saturating_sub(2) as usize;
    let content: Text<'static> = lyrics_text(app, ui_settings, content_height);
    let lyrics_paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .padding(Padding {
                    left: 1,
                    right: 0,
                    top: 0,
                    bottom: 0,
                })
                .borders(Borders::ALL)
                .title(" lyrics (gl closes) "),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(lyrics_paragraph, area);
}

/// Render the controls popup overlay when it is enabled.
pub(crate) fn render_controls_popup(
    frame: &mut Frame,
    app: &App,
    controls_settings: &ControlsSettings,
) {
    if !app.controls_popup {
        return;
    }

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

/// Render the bottom input panel (filter and auxiliary info).
pub(crate) fn render_bottom_input(frame: &mut Frame, area: Rect, text: String) {
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

    frame.render_widget(bottom_panel, area);
}
