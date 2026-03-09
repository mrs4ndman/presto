//! UI rendering helpers for the terminal user interface.
//!
//! This module orchestrates rendering and delegates text/layout/pane logic
//! to focused submodules.

mod layout;
mod lyrics;
mod panes;
mod text;

use ratatui::Frame;

use crate::app::App;
use crate::config::{ControlsSettings, UiSettings};

use self::layout::{main_layout, root_layout};
use self::panes::{
    render_bottom_input, render_controls_popup, render_header, render_lyrics_pane,
    render_metadata_pane, render_status, render_track_list,
};
use self::text::{bottom_info_text, metadata_text, status_text};

/// Render the entire UI into the provided `frame` using `app` state and settings.
pub fn draw(
    frame: &mut Frame,
    app: &App,
    display: &[usize],
    ui_settings: &UiSettings,
    controls_settings: &ControlsSettings,
) {
    let bottom_text = bottom_info_text(app, ui_settings);
    let status_text_val = status_text(app, ui_settings);
    let metadata = metadata_text(app);

    let root = root_layout(frame.area(), &status_text_val, bottom_text.as_deref());
    let show_lyrics_pane = app.lyrics_popup && ui_settings.lyrics_enabled;
    let main = main_layout(
        root.main,
        app.metadata_window,
        show_lyrics_pane,
        Some(&metadata),
    );

    render_header(frame, root.header, ui_settings);
    render_status(frame, root.status, status_text_val);
    render_track_list(frame, main.list, app, display, ui_settings);

    if let Some(meta_area) = main.metadata {
        render_metadata_pane(frame, meta_area, metadata);
    }

    if let Some(lyrics_area) = main.lyrics {
        render_lyrics_pane(frame, lyrics_area, app, ui_settings);
    }

    render_controls_popup(frame, app, controls_settings);

    if let (Some(bottom_area), Some(text)) = (root.bottom, bottom_text) {
        render_bottom_input(frame, bottom_area, text);
    }
}
