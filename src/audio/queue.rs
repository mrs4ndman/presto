//! Helpers to maintain the audio queue ordering.
//!
//! The audio thread keeps a `queue` of track indices which must be
//! kept consistent with the current shuffle `order`. These helpers
//! reorder and sanitize the queue in place.

pub(crate) fn reorder_queue_in_place(
    queue: &mut Vec<usize>,
    tracks_len: usize,
    shuffle: bool,
    order: &[usize],
) {
    // Remove out-of-range indices first.
    queue.retain(|&i| i < tracks_len);
    if !shuffle {
        // Non-shuffle mode: keep a stable ascending order.
        queue.sort_unstable();
        return;
    }

    // Build a position map for quick ordering lookups and sort accordingly.
    let mut pos_map = vec![usize::MAX; tracks_len];
    for (p, &ti) in order.iter().enumerate() {
        if ti < pos_map.len() {
            pos_map[ti] = p;
        }
    }
    queue.sort_by_key(|&ti| pos_map.get(ti).copied().unwrap_or(usize::MAX));
}
