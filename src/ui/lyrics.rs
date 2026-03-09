use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::time::Duration;

use crate::app::App;
use crate::config::UiSettings;
use crate::library::{Lyrics, TimedLyricLine};

use super::text::now_playing_track_text;

fn current_playback_elapsed(app: &App) -> Option<Duration> {
    app.playback_handle
        .as_ref()
        .and_then(|handle| handle.lock().ok().map(|info| info.elapsed))
}

fn active_timed_lyric_index(lines: &[TimedLyricLine], elapsed: Duration) -> Option<usize> {
    lines.iter().rposition(|line| line.timestamp <= elapsed)
}

/// Build styled lyric lines for timed lyrics around the active playback position.
///
/// Past lines are dimmed, the active line is bolded, and the output is windowed so the
/// pane remains readable even for very long lyric tracks.
pub(crate) fn timed_lyrics_lines(
    lines: &[TimedLyricLine],
    elapsed: Duration,
    max_visible: usize,
) -> Vec<Line<'static>> {
    if lines.is_empty() {
        return vec![Line::from("No timed lyrics found.")];
    }

    let active = active_timed_lyric_index(lines, elapsed);
    let focus = active.unwrap_or(0);
    let window = max_visible.max(5);
    let before = window / 3;
    let after = window.saturating_sub(before + 1);

    let mut start = focus.saturating_sub(before);
    let mut end = (focus + after + 1).min(lines.len());
    if end - start < window {
        start = end.saturating_sub(window);
        end = (start + window).min(lines.len());
    }

    let mut rendered: Vec<Line<'static>> = Vec::new();
    if start > 0 {
        rendered.push(Line::from("..."));
    }

    for (idx, line) in lines[start..end].iter().enumerate() {
        let absolute_idx = start + idx;
        let style = match active {
            Some(active_idx) if absolute_idx == active_idx => {
                Style::default().add_modifier(Modifier::BOLD)
            }
            Some(active_idx) if absolute_idx < active_idx => Style::default().fg(Color::DarkGray),
            _ => Style::default(),
        };
        rendered.push(Line::from(Span::styled(line.text.clone(), style)));
    }

    if end < lines.len() {
        rendered.push(Line::from("..."));
    }

    rendered
}

/// Build the lyrics pane text for the currently playing track.
///
/// Handles plain lyrics, timed lyrics synchronized to playback, and user-facing empty states.
pub(crate) fn lyrics_text(
    app: &App,
    ui_settings: &UiSettings,
    max_visible_lines: usize,
) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    match app.current_track_lyrics_index() {
        Some(idx) => {
            lines.push(Line::from(Span::styled(
                now_playing_track_text(app, idx, ui_settings),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(String::new()));

            match app.current_track_lyrics() {
                Some(Lyrics::Plain(lyrics)) => {
                    lines.extend(lyrics.lines().map(|line| Line::from(line.to_string())));
                }
                Some(Lyrics::Timed(timed)) => {
                    let elapsed = current_playback_elapsed(app).unwrap_or(Duration::ZERO);
                    lines.extend(timed_lyrics_lines(
                        timed,
                        elapsed,
                        max_visible_lines.saturating_sub(2),
                    ));
                }
                None => {
                    lines.push(Line::from("No embedded lyrics found."));
                }
            }
        }
        None => {
            lines.push(Line::from("No track playing."));
        }
    }

    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::timed_lyrics_lines;
    use crate::library::TimedLyricLine;
    use ratatui::style::Color;
    use std::time::Duration;

    #[test]
    fn timed_lyrics_styles_past_lines_as_dark_gray() {
        let lines = vec![
            TimedLyricLine {
                timestamp: Duration::from_secs(0),
                text: "line 1".to_string(),
            },
            TimedLyricLine {
                timestamp: Duration::from_secs(5),
                text: "line 2".to_string(),
            },
            TimedLyricLine {
                timestamp: Duration::from_secs(10),
                text: "line 3".to_string(),
            },
        ];

        let rendered = timed_lyrics_lines(&lines, Duration::from_secs(6), 10);

        let first_style = rendered[0].spans[0].style;
        let second_style = rendered[1].spans[0].style;
        let third_style = rendered[2].spans[0].style;

        assert_eq!(first_style.fg, Some(Color::DarkGray));
        assert!(
            second_style
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
        );
        assert_eq!(third_style.fg, None);
    }
}
