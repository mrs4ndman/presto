pub(crate) fn reorder_queue_in_place(
    queue: &mut Vec<usize>,
    tracks_len: usize,
    shuffle: bool,
    order: &[usize],
) {
    queue.retain(|&i| i < tracks_len);
    if !shuffle {
        queue.sort_unstable();
        return;
    }

    let mut pos_map = vec![usize::MAX; tracks_len];
    for (p, &ti) in order.iter().enumerate() {
        if ti < pos_map.len() {
            pos_map[ti] = p;
        }
    }
    queue.sort_by_key(|&ti| pos_map.get(ti).copied().unwrap_or(usize::MAX));
}
