use super::queue::reorder_queue_in_place;

#[test]
fn reorder_queue_unshuffled_sorts_and_filters() {
    let mut q = vec![5, 2, 999, 2, 0];
    reorder_queue_in_place(&mut q, 6, false, &[]);
    assert_eq!(q, vec![0, 2, 2, 5]);
}

#[test]
fn reorder_queue_shuffled_follows_order_positions() {
    // order position: 3->0, 1->1, 0->2, 2->3
    let order = vec![3, 1, 0, 2];
    let mut q = vec![0, 3, 2];
    reorder_queue_in_place(&mut q, 4, true, &order);
    assert_eq!(q, vec![3, 0, 2]);
}
