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
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());

    let title = Block::default().title("Presto 🎵").borders(Borders::ALL);
    frame.render_widget(title, chunks[0]);

    let filtered = app.filtered_indices();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&i| ListItem::new(app.tracks[i].title.clone()))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tracks"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let mut state = app_state(app);
    frame.render_stateful_widget(list, chunks[1], &mut state);

    let footer_text = if app.filter_mode || !app.filter_query.is_empty() {
        format!(
            "j/k move • enter play • p pause • n/b next/prev • / filter • q quit\nfilter: {}{}",
            app.filter_query,
            if app.filter_mode {
                " (type, backspace, esc clears)"
            } else {
                ""
            }
        )
    } else {
        "j/k move • enter play • p pause • n/b next/prev • / filter • q quit".to_string()
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    frame.render_widget(footer, chunks[2]);
}

fn app_state(app: &App) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    let filtered = app.filtered_indices();
    if !filtered.is_empty() {
        let pos = filtered
            .iter()
            .position(|&i| i == app.selected)
            .unwrap_or(0);
        state.select(Some(pos));
    }
    state
}
