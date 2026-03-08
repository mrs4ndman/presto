use super::*;
use crate::audio::OrderHandle;
use crate::library::Track;
use std::sync::{Arc, Mutex};

fn t(title: &str) -> Track {
    Track {
        path: std::path::PathBuf::new(),
        title: title.into(),
        artist: None,
        album: None,
        duration: None,
        display: title.into(),
    }
}

#[test]
fn fuzzy_match_simple() {
    let title = "Hello World";
    assert!(App::fuzzy_match_positions(title, "he wo").is_some());
    assert!(App::fuzzy_match_positions(title, "ello").is_some());
    assert!(App::fuzzy_match_positions(title, "xyz").is_none());
}

#[test]
fn fuzzy_match_does_not_span_multiple_words_for_one_term() {
    let title = "Hello World";

    assert!(App::fuzzy_match_positions(title, "hw").is_none());
    assert!(App::fuzzy_match_positions(title, "hel wo").is_some());
}

#[test]
fn display_indices_respects_filter_query() {
    let tracks = vec![t("Alpha"), t("Beta"), t("Gamma")];
    let mut app = App::new(tracks);
    app.push_filter_char('a');
    let visible = app.display_indices();
    assert!(!visible.is_empty());
}

#[test]
fn display_indices_respects_order_and_filter() {
    let tracks = vec![t("Alpha"), t("Beta"), t("Gamma"), t("Delta")];

    let mut app = App::new(tracks);
    // custom order: 2,0,3,1
    let order = vec![2usize, 0, 3, 1];
    let oh: OrderHandle = Arc::new(Mutex::new(order.clone()));
    app.set_order_handle(oh);
    app.shuffle = true;

    let disp = app.display_indices();
    assert_eq!(disp, order);

    // apply fuzzy filter 'et' -> matches Delta(3) and Beta(1)
    app.filter_query = "et".into();
    let disp2 = app.display_indices();
    assert_eq!(disp2, vec![3usize, 1usize]);
}

#[test]
fn display_indices_match_word_by_word_queries() {
    let tracks = vec![t("Metallica - Blackened"), t("Black Sabbath - Paranoid")];

    let mut app = App::new(tracks);
    app.filter_query = "me bl".into();

    let disp = app.display_indices();
    assert_eq!(disp, vec![0]);

    app.filter_query = "mtbk".into();

    let disp = app.display_indices();
    assert!(disp.is_empty());
}

#[test]
fn trimming_filter_query_affects_matching() {
    let tracks = vec![t("Black Sabbath - Paranoid")];

    let mut app = App::new(tracks);
    app.filter_query = "Black ".into();
    assert_eq!(app.display_indices(), vec![0]);

    app.filter_query = "   ".into();
    assert_eq!(app.display_indices(), vec![0]);
}

#[test]
fn next_prev_in_view_helpers_work() {
    let tracks = vec![t("Alpha"), t("Beta"), t("Gamma")];

    let mut app = App::new(tracks);
    app.filter_query = "et".into(); // only Beta is visible

    assert_eq!(app.next_in_view_from(0), Some(1));
    assert_eq!(app.prev_in_view_from(0), Some(1));
    assert_eq!(app.next_in_view_from(1), Some(1));
    assert_eq!(app.prev_in_view_from(1), Some(1));
}

#[test]
fn cycle_loop_mode_cycles_three_states() {
    let tracks = vec![t("A")];

    let mut app = App::new(tracks);
    assert_eq!(app.loop_mode, crate::audio::LoopMode::LoopAll);

    app.cycle_loop_mode();
    assert_eq!(app.loop_mode, crate::audio::LoopMode::LoopOne);

    app.cycle_loop_mode();
    assert_eq!(app.loop_mode, crate::audio::LoopMode::NoLoop);

    app.cycle_loop_mode();
    assert_eq!(app.loop_mode, crate::audio::LoopMode::LoopAll);
}

#[test]
fn queue_dirty_is_set_on_filter_changes() {
    let tracks = vec![t("Alpha")];

    let mut app = App::new(tracks);
    // new() starts dirty so initial queue can be synced
    assert!(app.queue_dirty);
    app.clear_queue_dirty();
    assert!(!app.queue_dirty);

    app.push_filter_char('a');
    assert!(app.queue_dirty);
    app.clear_queue_dirty();
    app.pop_filter_char();
    assert!(app.queue_dirty);
}

#[test]
fn initial_volume_percent_sets_current_and_initial() {
    let mut app = App::new(Vec::new());
    let v = app.set_initial_volume_percent(30);
    assert!((v - 0.30).abs() < f32::EPSILON);
    assert!((app.volume() - 0.30).abs() < f32::EPSILON);
}

#[test]
fn initial_volume_percent_clamps_out_of_range() {
    let mut app = App::new(Vec::new());
    let v = app.set_initial_volume_percent(250);
    assert!((v - 1.0).abs() < f32::EPSILON);
    assert!((app.volume() - 1.0).abs() < f32::EPSILON);

    let v = app.set_initial_volume_percent(0);
    assert!((v - 0.0).abs() < f32::EPSILON);
    assert!((app.volume() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn reset_volume_restores_initial_value() {
    let mut app = App::new(Vec::new());
    app.set_initial_volume_percent(75);
    app.set_volume(0.20);
    let v = app.reset_volume_to_initial();
    assert!((v - 0.75).abs() < f32::EPSILON);
    assert!((app.volume() - 0.75).abs() < f32::EPSILON);
}

#[test]
fn volume_percent_rounds_to_nearest_whole() {
    let mut app = App::new(Vec::new());
    app.set_volume(0.444);
    assert_eq!(app.volume_percent(), 44);

    app.set_volume(0.445);
    assert_eq!(app.volume_percent(), 45);
}

#[test]
fn sync_current_track_lyrics_clears_state_when_no_track_is_playing() {
    let mut app = App::new(vec![t("Alpha")]);

    app.sync_current_track_lyrics(Some(0));
    assert_eq!(app.current_track_lyrics_index(), Some(0));

    app.sync_current_track_lyrics(None);
    assert_eq!(app.current_track_lyrics_index(), None);
    assert_eq!(app.current_track_lyrics(), None);
}

#[test]
fn clear_current_track_lyrics_hides_popup() {
    let mut app = App::new(vec![t("Alpha")]);
    app.lyrics_popup = true;

    app.clear_current_track_lyrics();

    assert!(!app.lyrics_popup);
    assert_eq!(app.current_track_lyrics_index(), None);
    assert_eq!(app.current_track_lyrics(), None);
}
