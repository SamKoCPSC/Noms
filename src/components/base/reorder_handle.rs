use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReorderDirection {
    Up,
    Down,
}

/// Return the adjacent position for a move, or `None` at either boundary.
pub fn reorder_target(
    current_index: usize,
    item_count: usize,
    direction: ReorderDirection,
) -> Option<usize> {
    match direction {
        ReorderDirection::Up => current_index.checked_sub(1),
        ReorderDirection::Down => current_index
            .checked_add(1)
            .filter(|target_index| *target_index < item_count),
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct ReorderHandleProps {
    pub current_index: usize,
    pub item_count: usize,
    pub item_label: String,
    #[props(default)]
    pub disabled: bool,
    pub on_reorder: EventHandler<usize>,
}

/// Accessible one-position reorder controls shared by folders and recipes.
#[component]
pub fn ReorderHandle(props: ReorderHandleProps) -> Element {
    let up_target = reorder_target(props.current_index, props.item_count, ReorderDirection::Up);
    let down_target = reorder_target(
        props.current_index,
        props.item_count,
        ReorderDirection::Down,
    );
    let up_label = format!("Move {} up", props.item_label);
    let down_label = format!("Move {} down", props.item_label);

    rsx! {
        div {
            display: "flex",
            gap: "var(--space-xs)",
            aria_label: "Reorder {props.item_label}",
            button {
                r#type: "button",
                class: "btn btn-secondary touch-target",
                aria_label: "{up_label}",
                title: "{up_label}",
                disabled: props.disabled || up_target.is_none(),
                onclick: move |_| {
                    if !props.disabled {
                        if let Some(target_index) = up_target {
                            props.on_reorder.call(target_index);
                        }
                    }
                },
                "↑"
            }
            button {
                r#type: "button",
                class: "btn btn-secondary touch-target",
                aria_label: "{down_label}",
                title: "{down_label}",
                disabled: props.disabled || down_target.is_none(),
                onclick: move |_| {
                    if !props.disabled {
                        if let Some(target_index) = down_target {
                            props.on_reorder.call(target_index);
                        }
                    }
                },
                "↓"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{reorder_target, ReorderDirection};

    #[test]
    fn first_and_last_reorder_boundaries_are_disabled() {
        assert_eq!(reorder_target(0, 3, ReorderDirection::Up), None);
        assert_eq!(reorder_target(2, 3, ReorderDirection::Down), None);
        assert_eq!(reorder_target(0, 0, ReorderDirection::Down), None);
    }

    #[test]
    fn valid_reorder_targets_move_exactly_one_position() {
        assert_eq!(reorder_target(1, 3, ReorderDirection::Up), Some(0));
        assert_eq!(reorder_target(1, 3, ReorderDirection::Down), Some(2));
    }
}
