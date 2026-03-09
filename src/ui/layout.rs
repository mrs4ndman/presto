use ratatui::layout::{Constraint, Direction, Layout, Rect};

use super::text::wrapped_line_count;

pub(crate) struct RootLayout {
    pub(crate) header: Rect,
    pub(crate) status: Rect,
    pub(crate) main: Rect,
    pub(crate) bottom: Option<Rect>,
}

pub(crate) struct MainLayout {
    pub(crate) list: Rect,
    pub(crate) metadata: Option<Rect>,
    pub(crate) lyrics: Option<Rect>,
}

/// Split the full frame area into header, status, main content, and optional bottom input.
///
/// The status and bottom heights are estimated from wrapped text so those panels can grow
/// when content becomes longer on narrow terminals.
pub(crate) fn root_layout(area: Rect, status_text: &str, bottom_text: Option<&str>) -> RootLayout {
    let bottom_height = if let Some(text) = bottom_text {
        let bottom_content_width = area.width.saturating_sub(3).max(1);
        wrapped_line_count(text, bottom_content_width)
            .saturating_add(2)
            .max(3)
    } else {
        0
    };

    let status_content_width = area.width.saturating_sub(3).max(1);
    let status_height = wrapped_line_count(status_text, status_content_width)
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
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(status_height),
                Constraint::Min(1),
                Constraint::Length(bottom_height),
            ])
            .split(area)
    };

    RootLayout {
        header: chunks[0],
        status: chunks[1],
        main: chunks[2],
        bottom: if bottom_height == 0 {
            None
        } else {
            Some(chunks[3])
        },
    }
}

/// Split the main content area into track list plus optional metadata/lyrics side panes.
///
/// When both side panes are visible, metadata gets a bounded top region and lyrics fills
/// the remaining space below it.
pub(crate) fn main_layout(
    main_area: Rect,
    show_metadata: bool,
    show_lyrics: bool,
    metadata_text: Option<&str>,
) -> MainLayout {
    let mut list_area = main_area;
    let mut meta_area: Option<Rect> = None;
    let mut lyrics_area: Option<Rect> = None;

    if show_metadata || show_lyrics {
        let side_width = (list_area.width * 2 / 5).clamp(32, 60);
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(20),
                Constraint::Length(1),
                Constraint::Length(side_width),
            ])
            .split(list_area);
        list_area = panes[0];

        match (show_metadata, show_lyrics) {
            (true, true) => {
                let sidebar = panes[2];
                let meta = metadata_text.unwrap_or_default();
                let meta_content_width = sidebar.width.saturating_sub(3).max(1);
                let ideal_meta_height = wrapped_line_count(meta, meta_content_width)
                    .saturating_add(2)
                    .max(7);
                let max_meta_height = sidebar.height.saturating_sub(8).max(7);
                let meta_height = ideal_meta_height.min(max_meta_height);
                let stacked = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(meta_height),
                        Constraint::Length(1),
                        Constraint::Min(6),
                    ])
                    .split(sidebar);
                meta_area = Some(stacked[0]);
                lyrics_area = Some(stacked[2]);
            }
            (true, false) => {
                meta_area = Some(panes[2]);
            }
            (false, true) => {
                lyrics_area = Some(panes[2]);
            }
            (false, false) => {}
        }
    }

    MainLayout {
        list: list_area,
        metadata: meta_area,
        lyrics: lyrics_area,
    }
}

/// Compute a centered rectangle with given size constrained to `r`.
pub(crate) fn centered_rect_sized(mut width: u16, mut height: u16, r: Rect) -> Rect {
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
