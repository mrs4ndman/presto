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
    assert!(App::fuzzy_match_positions(title, "hw").is_some());
    assert!(App::fuzzy_match_positions(title, "ello").is_some());
    assert!(App::fuzzy_match_positions(title, "xyz").is_none());
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
fn display_indices_uses_fuzzy_not_substring_only() {
    let tracks = vec![t("Metallica - Blackened"), t("Black Sabbath - Paranoid")];

    let mut app = App::new(tracks);
    // Fuzzy query: letters appear in order but not necessarily contiguously
    app.filter_query = "mtbk".into();

    let disp = app.display_indices();
    assert_eq!(disp, vec![0]);
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
