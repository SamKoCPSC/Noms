/// Move an item to another position in the same list.
///
/// Returns `false` when either position is outside the list. Moving an item
/// to its current position is a successful no-op.
pub fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= items.len() || to >= items.len() {
        return false;
    }
    if from == to {
        return true;
    }

    let item = items.remove(from);
    items.insert(to, item);
    true
}

#[cfg(test)]
mod tests {
    use super::move_item;

    #[test]
    fn moves_an_item_forward_to_the_requested_position() {
        let mut items = vec!["first", "second", "third", "fourth"];

        assert!(move_item(&mut items, 0, 2));
        assert_eq!(items, vec!["second", "third", "first", "fourth"]);
    }

    #[test]
    fn moves_an_item_backward_to_the_requested_position() {
        let mut items = vec!["first", "second", "third", "fourth"];

        assert!(move_item(&mut items, 3, 1));
        assert_eq!(items, vec!["first", "fourth", "second", "third"]);
    }

    #[test]
    fn rejects_out_of_bounds_positions_without_changing_the_list() {
        let mut items = vec!["first", "second"];

        assert!(!move_item(&mut items, 2, 0));
        assert!(!move_item(&mut items, 0, 2));
        assert_eq!(items, vec!["first", "second"]);
    }

    #[test]
    fn moving_to_the_current_position_is_a_successful_no_op() {
        let mut items = vec!["first", "second"];

        assert!(move_item(&mut items, 1, 1));
        assert_eq!(items, vec!["first", "second"]);
    }
}
