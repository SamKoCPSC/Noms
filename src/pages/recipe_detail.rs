//! Single recipe detail page.
//!
//! Fetches recipe data and tags via server functions, renders typed vectors
//! of ingredients, steps (with recursive sub-steps), and equipment directly.
//!
//! Publicly accessible: tries `get_public_recipe` first (no auth needed),
//! falls back to authenticated `get_recipe` for private/unlisted recipes.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use std::collections::VecDeque;

#[cfg(target_arch = "wasm32")]
use crate::api::recipe::delete_recipe;
use crate::api::recipe::{
    compare_recipe_versions_api, fork_recipe, get_fork_attribution, get_public_recipe,
    get_public_recipe_tags_by_id, get_recipe, get_recipe_lineage, get_recipe_owner_username,
    get_recipe_tags, get_recipe_version_api, get_recipe_versions_api, restore_version,
    update_recipe_visibility,
};
use crate::auth::context::use_auth;
use crate::components::base::{
    Button, ButtonVariant, Card, EditedRecipeImage, LoadingSpinner, PageHeader,
};
use crate::types::{
    ForkAttribution, Recipe, RecipeEquipment, RecipeIngredient, RecipeLineageBranch,
    RecipeLineageNode, RecipeLineageView, RecipeStep, RecipeVersionChange, RecipeVersionComparison,
    RecipeVersionSummary, RecipeVersionView,
};
#[cfg(target_arch = "wasm32")]
use crate::utils::recipe_scaler::{
    format_amount, parse_amount, IngredientRef, ScaleCalculator, ScaleMode, ScaledIngredient,
};

const LINEAGE_DESCENDANT_DISPLAY_DEPTH: usize = 8;
const LINEAGE_INITIAL_VISIBLE_DESCENDANTS: usize = 10;
const LINEAGE_DESCENDANT_EXPANSION_SIZE: usize = 5;

fn fork_sign_in_url(recipe_id: &str) -> String {
    format!("/login?redirect_uri=/recipes/{recipe_id}")
}

// ── Date formatting helper ───────────────────────────────────────────────────

/// Format a UTC datetime as a relative time string (e.g. "3 days ago").
fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);
    if diff.num_days() > 365 {
        format!("{} years ago", diff.num_days() / 365)
    } else if diff.num_days() > 30 {
        format!("{} months ago", diff.num_days() / 30)
    } else if diff.num_days() > 0 {
        format!("{} days ago", diff.num_days())
    } else if diff.num_hours() > 0 {
        format!("{} hours ago", diff.num_hours())
    } else if diff.num_minutes() > 0 {
        format!("{} minutes ago", diff.num_minutes())
    } else {
        "Just now".to_string()
    }
}

fn format_change_path(path: &str) -> String {
    let mut labels = Vec::<String>::new();
    for token in path.split('/').filter(|token| !token.is_empty()) {
        let token = token.replace("~1", "/").replace("~0", "~");
        if let Ok(index) = token.parse::<usize>() {
            if let Some(label) = labels.last_mut() {
                label.push_str(&format!(" #{}", index + 1));
            } else {
                labels.push(format!("Item #{}", index + 1));
            }
            continue;
        }

        let label = match token.as_str() {
            "prep_time_minutes" => "Prep time".to_string(),
            "cook_time_minutes" => "Cook time".to_string(),
            "instructions" => "Steps".to_string(),
            "sub_steps" => "Sub-steps".to_string(),
            "thumb_url" => "Thumbnail".to_string(),
            "file_name" => "File name".to_string(),
            "file_size" => "File size".to_string(),
            _ => {
                let words = token.replace('_', " ");
                let mut characters = words.chars();
                match characters.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                    None => String::new(),
                }
            }
        };
        labels.push(label);
    }

    if labels.is_empty() {
        "Recipe".to_string()
    } else {
        labels.join(" · ")
    }
}

fn format_change_value(value: Option<&String>) -> String {
    match value {
        None => "—".to_string(),
        Some(value) => value.clone(),
    }
}

fn visibility_selection_differs_from_recipe(
    selected_visibility: Signal<String>,
    recipe_visibility: &str,
) -> bool {
    selected_visibility.peek().as_str() != recipe_visibility
}

#[component]
fn VersionComparisonPanel(
    title: String,
    description: String,
    changes: Vec<RecipeVersionChange>,
) -> Element {
    rsx! {
        section {
            class: "neumo-inset",
            padding: "var(--space-md)",
            h3 { margin_bottom: "var(--space-xs)", "{title}" }
            p {
                color: "var(--text-secondary)",
                margin_bottom: "var(--space-md)",
                "{description}"
            }
            if changes.is_empty() {
                p { "No content differences between these versions." }
            } else {
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-md)",
                    for change in &changes {
                        {
                            let field = format_change_path(&change.path);
                            let before = format_change_value(change.old_value.as_ref());
                            let after = format_change_value(change.value.as_ref());
                            let operation = match change.op.as_str() {
                                "add" => "Added",
                                "remove" => "Removed",
                                _ => "Changed",
                            };
                            let is_added = change.op == "add";
                            let is_removed = change.op == "remove";
                            let operation_color = if is_added {
                                "var(--success)"
                            } else if is_removed {
                                "var(--error)"
                            } else {
                                "var(--warning)"
                            };
                            let operation_background = if is_added {
                                "var(--success-bg)"
                            } else if is_removed {
                                "var(--error-bg)"
                            } else {
                                "color-mix(in srgb, var(--warning) 14%, transparent)"
                            };
                            rsx! {
                                div {
                                    padding: "var(--space-sm)",
                                    border: "1px solid var(--border-color)",
                                    border_radius: "var(--radius-md)",
                                    div {
                                        display: "flex",
                                        justify_content: "space-between",
                                        align_items: "baseline",
                                        gap: "var(--space-sm)",
                                        margin_bottom: "var(--space-sm)",
                                        strong { "{field}" }
                                        span {
                                            color: operation_color,
                                            background: operation_background,
                                            font_size: "var(--font-size-sm)",
                                            font_weight: "700",
                                            padding: "2px var(--space-sm)",
                                            border_radius: "var(--radius-full)",
                                            "{operation}"
                                        }
                                    }
                                    div {
                                        display: "grid",
                                        grid_template_columns: "repeat(auto-fit, minmax(220px, 1fr))",
                                        gap: "var(--space-md)",
                                        if !is_added {
                                            div {
                                                background: "var(--error-bg)",
                                                border_left: "4px solid var(--error)",
                                                border_radius: "var(--radius-sm)",
                                                padding: "var(--space-sm)",
                                                span {
                                                    display: "block",
                                                    color: "var(--error)",
                                                    font_size: "var(--font-size-sm)",
                                                    font_weight: "700",
                                                    margin_bottom: "var(--space-xs)",
                                                    "− Before"
                                                }
                                                pre {
                                                    margin: "0",
                                                    white_space: "pre-wrap",
                                                    overflow_wrap: "anywhere",
                                                    font_family: "inherit",
                                                    "{before}"
                                                }
                                            }
                                        }
                                        if !is_removed {
                                            div {
                                                background: "var(--success-bg)",
                                                border_left: "4px solid var(--success)",
                                                border_radius: "var(--radius-sm)",
                                                padding: "var(--space-sm)",
                                                span {
                                                    display: "block",
                                                    color: "var(--success)",
                                                    font_size: "var(--font-size-sm)",
                                                    font_weight: "700",
                                                    margin_bottom: "var(--space-xs)",
                                                    "+ After"
                                                }
                                                pre {
                                                    margin: "0",
                                                    white_space: "pre-wrap",
                                                    overflow_wrap: "anywhere",
                                                    font_family: "inherit",
                                                    "{after}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::{
        branch_has_hidden_direct_children, collapse_revealed_descendant_paths,
        descendant_branch_has_more, descendant_paths_breadth_first, format_change_path,
        format_change_value, next_hidden_descendant_paths, LINEAGE_DESCENDANT_DISPLAY_DEPTH,
        LINEAGE_DESCENDANT_EXPANSION_SIZE, LINEAGE_INITIAL_VISIBLE_DESCENDANTS,
    };
    use crate::types::{RecipeLineageBranch, RecipeLineageNode};

    fn lineage_branch(children: Vec<RecipeLineageBranch>, truncated: bool) -> RecipeLineageBranch {
        named_lineage_branch("Branch", children, truncated)
    }

    fn named_lineage_branch(
        title: &str,
        children: Vec<RecipeLineageBranch>,
        truncated: bool,
    ) -> RecipeLineageBranch {
        RecipeLineageBranch {
            node: RecipeLineageNode {
                recipe_id: None,
                title: title.to_string(),
                owner_username: None,
                image_url: None,
                forked_from_version_number: None,
                fork_message: None,
                is_deleted: false,
                is_redacted: false,
            },
            children,
            truncated,
        }
    }

    #[test]
    fn version_history_is_the_final_recipe_detail_section() {
        let source = include_str!("recipe_detail.rs");
        let commentary_position = source
            .rfind("// ── CARD 7: Commentary")
            .expect("commentary section marker");
        let history_position = source
            .rfind("h2 { \"Version history\" }")
            .expect("version history heading");

        assert!(history_position > commentary_position);
    }

    #[test]
    fn version_history_links_to_a_full_page_snapshot() {
        let source = include_str!("recipe_detail.rs");
        let route_link = ["to: crate::Route::RecipeVersion", "Detail"].concat();
        let page_component = ["pub fn RecipeVersion", "Detail"].concat();
        let embedded_selection_state = ["let mut selected_", "version"].concat();

        assert!(source.contains(&route_link));
        assert!(source.contains(&page_component));
        assert!(!source.contains(&embedded_selection_state));
    }

    #[test]
    fn version_history_offers_an_inline_difference_viewer() {
        let source = include_str!("recipe_detail.rs");
        let detail_component = ["pub fn Recipe", "Detail"].concat();
        let version_component = ["pub fn RecipeVersion", "Detail"].concat();
        let detail_page = source
            .split(&detail_component)
            .nth(1)
            .expect("recipe detail page")
            .split(&version_component)
            .next()
            .expect("recipe detail page body");
        let button_label = ["View", " differences"].concat();
        let expanded_state = ["expanded_version", "_difference"].concat();
        let comparison_api = ["compare_recipe_", "versions_api"].concat();

        assert!(detail_page.contains(&button_label));
        assert!(detail_page.contains(&expanded_state));
        assert!(detail_page.contains("VersionComparisonPanel"));
        assert!(detail_page.contains(&comparison_api));
    }

    #[test]
    fn lineage_nodes_render_recipe_thumbnails_with_a_placeholder_fallback() {
        let source = include_str!("recipe_detail.rs");
        let component_marker = ["fn LineageRecipe", "Node("].concat();
        let component = source
            .split(&component_marker)
            .nth(1)
            .expect("lineage recipe node component");

        assert!(component.contains("node.image_url"));
        assert!(component.contains("recipe-lineage__thumbnail"));
        assert!(component.contains("recipe-lineage__thumbnail-placeholder"));
    }

    #[test]
    fn version_history_entries_render_snapshot_thumbnails_with_a_placeholder_fallback() {
        let source = include_str!("recipe_detail.rs");
        let history = source
            .split("h2 { \"Version history\" }")
            .nth(1)
            .expect("version history section");

        assert!(history.contains("version.image_url"));
        assert!(history.contains("recipe-version-history__thumbnail"));
        assert!(history.contains("recipe-version-history__thumbnail-placeholder"));
    }

    #[test]
    fn version_history_actions_wrap_within_the_card() {
        let source = include_str!("recipe_detail.rs");
        let owner_history = ["if is_owner && !versions", ".is_empty()"].concat();
        let version_history = source
            .split(&owner_history)
            .nth(1)
            .expect("version history")
            .split("/// Full-page, read-only")
            .next()
            .expect("version history body");

        assert!(version_history.contains("width: \"100%\""));
        assert!(version_history.contains("flex: \"0 0 100%\""));
        assert!(version_history.contains("justify_content: \"flex-start\""));
        assert!(version_history.contains("flex_wrap: \"wrap\""));
    }

    #[test]
    fn changing_visibility_does_not_trigger_server_value_resynchronization() {
        let source = include_str!("recipe_detail.rs");
        let comparison_helper = source
            .split("fn visibility_selection_differs_from_recipe")
            .nth(1)
            .expect("visibility selection comparison helper")
            .split("#[component]")
            .next()
            .expect("visibility selection comparison helper body");
        let synchronization_effect = source
            .rsplit("// Tags (signal-based, loaded conditionally based on ownership)")
            .next()
            .expect("visibility synchronization follows tag signal declarations")
            .split("// Load tags once recipe is available")
            .next()
            .expect("visibility synchronization effect");

        assert!(comparison_helper.contains("selected_visibility.peek()"));
        assert!(synchronization_effect.contains("visibility_selection_differs_from_recipe"));
        assert!(!synchronization_effect.contains("selected_visibility()"));
    }

    #[test]
    fn version_differences_use_git_style_removed_and_added_colors() {
        let source = include_str!("recipe_detail.rs");
        let comparison_panel = source
            .split("fn VersionComparisonPanel")
            .nth(1)
            .expect("comparison panel")
            .split("#[cfg(test)]")
            .next()
            .expect("comparison panel body");
        let removed_background = ["var(--", "error-bg)"].concat();
        let added_background = ["var(--", "success-bg)"].concat();

        assert!(comparison_panel.contains(&removed_background));
        assert!(comparison_panel.contains(&added_background));
        assert!(comparison_panel.contains("− Before"));
        assert!(comparison_panel.contains("+ After"));
    }

    #[test]
    fn historical_snapshot_uses_the_recipe_detail_page_layout() {
        let source = include_str!("recipe_detail.rs");
        let page_component = ["pub fn RecipeVersion", "Detail"].concat();
        let historical_page = source
            .split(&page_component)
            .nth(1)
            .expect("historical recipe page");

        assert!(historical_page.contains("class: \"recipe-detail container\""));
        assert!(historical_page.contains("Historical version"));
        assert!(historical_page.contains("Back to current recipe"));
    }

    #[test]
    fn historical_snapshot_subscribes_to_the_resource_result_before_loading_return() {
        let source = include_str!("recipe_detail.rs");
        let page_component = ["pub fn RecipeVersion", "Detail"].concat();
        let historical_page = source
            .split(&page_component)
            .nth(1)
            .expect("historical recipe page");
        let result_read = historical_page
            .find("version_resource.read()")
            .expect("version result read");
        let loading_return = historical_page
            .find("if version_resource.pending()")
            .expect("loading return");

        assert!(result_read < loading_return);
    }

    #[test]
    fn signed_out_fork_control_links_to_login_and_returns_to_the_recipe() {
        let source = include_str!("recipe_detail.rs");
        let detail_component_marker = ["pub fn Recipe", "Detail"].concat();
        let detail_component = source
            .split(&detail_component_marker)
            .nth(1)
            .expect("recipe detail component")
            .split("/// Full-page, read-only")
            .next()
            .expect("recipe detail component body");

        assert!(detail_component.contains("if auth.is_authenticated"));
        assert!(detail_component.contains("href: fork_sign_in_url"));
        assert!(source.contains("fn fork_sign_in_url"));
    }

    #[test]
    fn signed_out_missing_recipe_preserves_the_public_not_found_error() {
        let source = include_str!("recipe_detail.rs");
        let detail_component_marker = ["pub fn Recipe", "Detail"].concat();
        let recipe_resource = source
            .split(&detail_component_marker)
            .nth(1)
            .expect("recipe detail component")
            .split("// Owner username resource")
            .next()
            .expect("recipe resource body");

        assert!(recipe_resource.contains("Err(public_error) if auth.is_authenticated"));
        assert!(recipe_resource.contains("Err(public_error) => Err(public_error)"));
    }

    #[test]
    fn change_paths_are_readable_and_use_one_based_item_numbers() {
        assert_eq!(
            format_change_path("/ingredients/1/name"),
            "Ingredients #2 · Name"
        );
        assert_eq!(
            format_change_path("/instructions/0/sub_steps/1/text"),
            "Steps #1 · Sub-steps #2 · Text"
        );
        assert_eq!(format_change_path("/prep_time_minutes"), "Prep time");
        assert_eq!(format_change_path("/tags/0"), "Tags #1");
    }

    #[test]
    fn change_values_show_plain_scalars_and_structured_content() {
        assert_eq!(format_change_value(None), "—");
        assert_eq!(format_change_value(Some(&"Not set".to_string())), "Not set");
        assert_eq!(
            format_change_value(Some(&"Sea salt".to_string())),
            "Sea salt"
        );
        assert_eq!(format_change_value(Some(&"(empty)".to_string())), "(empty)");
        assert_eq!(format_change_value(Some(&"4".to_string())), "4");
        assert_eq!(
            format_change_value(Some(
                &"{\n  \"amount\": \"1\",\n  \"name\": \"salt\",\n  \"unit\": \"tsp\"\n}"
                    .to_string()
            )),
            "{\n  \"amount\": \"1\",\n  \"name\": \"salt\",\n  \"unit\": \"tsp\"\n}"
        );
    }

    #[test]
    fn historical_page_offers_builtin_and_arbitrary_version_comparisons() {
        let source = include_str!("recipe_detail.rs");
        let current_heading = ["Compared with", " current version"].concat();
        let original_heading = ["Compared with", " original version"].concat();
        let custom_heading = ["Compare any", " two versions"].concat();
        let comparison_api = ["compare_recipe_", "versions_api"].concat();

        assert!(source.contains(&current_heading));
        assert!(source.contains(&original_heading));
        assert!(source.contains(&custom_heading));
        assert!(source.contains(&comparison_api));
    }

    #[test]
    fn arbitrary_version_selectors_are_locked_while_comparing() {
        let source = include_str!("recipe_detail.rs");
        let page_component = ["pub fn RecipeVersion", "Detail"].concat();
        let historical_page = source
            .split(&page_component)
            .nth(1)
            .expect("historical recipe page");

        assert_eq!(
            historical_page.matches("disabled: is_comparing()").count(),
            2
        );
    }

    #[test]
    fn recipe_details_offer_an_accessible_bidirectional_fork_lineage() {
        let source = include_str!("recipe_detail.rs");

        assert!(source.contains("get_recipe_lineage"));
        assert!(source.contains("Fork lineage"));
        assert!(source.contains("Ancestors"));
        assert!(source.contains("Descendants"));
        assert!(source.contains("recipe-lineage__current"));
        assert!(source.contains("More recipes forked from this branch"));
        assert!(!source.contains("aria_label: \"Expand descendant branches\""));
    }

    #[test]
    fn recipe_details_link_to_the_dedicated_version_graph() {
        let source = include_str!("recipe_detail.rs");

        assert!(source.contains("View version graph"));
        assert!(source.contains("Route::RecipeVersionGraph"));
    }

    #[test]
    fn descendant_lineage_stops_after_eight_visible_levels() {
        assert_eq!(LINEAGE_DESCENDANT_DISPLAY_DEPTH, 8);

        let deeper_branch = lineage_branch(Vec::new(), false);
        let branch_with_deeper_recipes = lineage_branch(vec![deeper_branch], false);

        assert!(!descendant_branch_has_more(&branch_with_deeper_recipes, 7));
        assert!(descendant_branch_has_more(&branch_with_deeper_recipes, 8));
    }

    #[test]
    fn descendant_visibility_is_allocated_breadth_first() {
        let descendants = vec![
            named_lineage_branch(
                "A",
                vec![
                    named_lineage_branch(
                        "A1",
                        vec![named_lineage_branch("A1a", Vec::new(), false)],
                        false,
                    ),
                    named_lineage_branch("A2", Vec::new(), false),
                ],
                false,
            ),
            named_lineage_branch(
                "B",
                vec![named_lineage_branch("B1", Vec::new(), false)],
                false,
            ),
        ];

        assert_eq!(
            descendant_paths_breadth_first(&descendants, LINEAGE_DESCENDANT_DISPLAY_DEPTH),
            vec![
                vec![0],
                vec![1],
                vec![0, 0],
                vec![0, 1],
                vec![1, 0],
                vec![0, 0, 0],
            ]
        );
    }

    #[test]
    fn descendant_visibility_starts_at_ten() {
        assert_eq!(LINEAGE_INITIAL_VISIBLE_DESCENDANTS, 10);
        assert_eq!(LINEAGE_DESCENDANT_EXPANSION_SIZE, 5);
    }

    #[test]
    fn branch_expansion_reveals_at_most_five_subtree_recipes_breadth_first() {
        let all_paths = vec![
            vec![0],
            vec![1],
            vec![0, 0],
            vec![0, 1],
            vec![0, 2],
            vec![1, 0],
            vec![1, 1],
            vec![1, 2],
            vec![1, 3],
            vec![1, 4],
            vec![1, 5],
            vec![0, 0, 0],
        ];
        let visible_paths = vec![vec![0], vec![1], vec![0, 0], vec![1, 0]];

        assert_eq!(
            next_hidden_descendant_paths(&all_paths, &visible_paths, &[1]),
            vec![vec![1, 1], vec![1, 2], vec![1, 3], vec![1, 4], vec![1, 5]]
        );
        assert_eq!(
            next_hidden_descendant_paths(&all_paths, &visible_paths, &[0]),
            vec![vec![0, 1], vec![0, 2], vec![0, 0, 0]]
        );
    }

    #[test]
    fn branch_controls_only_mark_visible_nodes_with_hidden_direct_children() {
        let branch = named_lineage_branch(
            "Parent",
            vec![
                named_lineage_branch("Child 1", Vec::new(), false),
                named_lineage_branch("Child 2", Vec::new(), false),
            ],
            false,
        );

        assert!(branch_has_hidden_direct_children(
            &branch,
            &[0],
            &[vec![0], vec![0, 0]],
        ));
        assert!(!branch_has_hidden_direct_children(
            &branch,
            &[0],
            &[vec![0], vec![0, 0], vec![0, 1]],
        ));
    }

    #[test]
    fn collapsing_one_branch_preserves_reveals_in_other_branches() {
        let revealed_paths = vec![vec![0, 1], vec![0, 1, 0], vec![1, 1]];

        assert_eq!(
            collapse_revealed_descendant_paths(&revealed_paths, &[0]),
            vec![vec![1, 1]]
        );
    }

    #[test]
    fn branch_expansion_controls_identify_their_specific_subtree() {
        let source = include_str!("recipe_detail.rs");
        let descendant_component_marker = ["fn Lineage", "Descendant("].concat();
        let lineage_component_marker = ["fn Fork", "Lineage"].concat();
        let component = source
            .split(&descendant_component_marker)
            .nth(1)
            .expect("lineage descendant component")
            .split(&lineage_component_marker)
            .next()
            .expect("lineage descendant component body");

        assert!(component.contains("id: branch_subtree_id.clone()"));
        assert_eq!(
            component
                .matches("aria_controls: branch_subtree_id.clone()")
                .count(),
            2
        );
    }

    #[test]
    fn descendant_lineage_reports_backend_truncation_at_any_level() {
        let truncated_branch = lineage_branch(Vec::new(), true);

        assert!(descendant_branch_has_more(&truncated_branch, 1));
    }

    #[test]
    fn accessible_lineage_recipes_use_the_entire_card_as_a_link() {
        let source = include_str!("recipe_detail.rs");
        let component_marker = ["fn LineageRecipe", "Node("].concat();
        let component = source
            .split(&component_marker)
            .nth(1)
            .expect("lineage recipe node component")
            .split("fn descendant_branch_has_more")
            .next()
            .expect("lineage recipe node body");

        assert!(component.contains("recipe-lineage__node--linked"));
        assert!(component.contains("if let Some(recipe_id) = recipe_id"));
        assert!(component.contains("a {\n                href:"));
        assert!(component.contains("href: format!(\"/recipes/{recipe_id}\")"));
        assert!(component.contains("aria_label: format!(\"Open recipe: {}\", node.title)"));
        assert!(component.contains("class: linked_node_class"));
        assert!(component.contains("else {\n            article {"));
        assert!(!component.contains("class: \"recipe-lineage__node-link\""));
    }
}

// ── Recursive Step Rendering ─────────────────────────────────────────────────

/// Roman numeral for a given 1-based index (up to 10).
fn roman_numeral(n: usize) -> String {
    match n {
        1 => "i".to_string(),
        2 => "ii".to_string(),
        3 => "iii".to_string(),
        4 => "iv".to_string(),
        5 => "v".to_string(),
        6 => "vi".to_string(),
        7 => "vii".to_string(),
        8 => "viii".to_string(),
        9 => "ix".to_string(),
        10 => "x".to_string(),
        _ => n.to_string(),
    }
}

/// Build a hierarchical step label.
/// Level 0: "1.", "2."
/// Level 1: "1a.", "1b."
/// Level 2+: "1a-i.", "1a-ii."
fn build_step_label(path: &[usize]) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for (i, &idx) in path.iter().enumerate() {
        let zero_idx = idx; // 0-based index from the parent's sub_steps vec
        match i {
            0 => parts.push(format!("{}", zero_idx + 1)),
            1 => parts.push(format!("{}", ((b'a' as usize) + zero_idx) as u8 as char)),
            _ => parts.push(roman_numeral(zero_idx + 1)),
        }
    }
    format!("{}.", parts.join("-"))
}

/// Recursive component that renders a `RecipeStep` and its nested sub-steps
/// with hierarchical numbering.
#[component]
fn StepNode(step: RecipeStep, path: Vec<usize>, level: usize) -> Element {
    let label = build_step_label(&path);
    let indent = if level == 0 {
        0.0
    } else {
        (level - 1) as f64 * 20.0
    };
    let photos = step.display_photos();

    rsx! {
        li {
            padding: "var(--space-xs) 0",
            margin_left: "{indent}px",
            font_size: "14px",
            color: "var(--text-primary)",
            line_height: "1.6",
            strong { font_weight: "600", margin_right: "var(--space-xs)", "{label}" }
            "{step.text}"

            if !photos.is_empty() {
                div {
                    display: "grid",
                    grid_template_columns: "repeat(auto-fit, minmax(140px, 1fr))",
                    gap: "var(--space-sm)",
                    margin_top: "var(--space-sm)",
                    for (photo_idx, photo) in photos.iter().enumerate() {
                        EditedRecipeImage {
                            key: "{photo_idx}",
                            image: photo.clone(),
                            class: "recipe-step-photo",
                            loading: "lazy",
                            fallback_fit: "cover".to_string(),
                            alt: "Step photo".to_string(),
                            on_error: move |_| {},
                        }
                    }
                }
            }

            if !step.sub_steps.is_empty() {
                ol {
                    padding_left: "0",
                    margin_top: "var(--space-xs)",
                    list_style: "none",
                    for (idx, sub) in step.sub_steps.iter().enumerate() {
                        StepNode { step: sub.clone(), path: [path.clone(), vec![idx]].concat(), level: level + 1 }
                    }
                }
            }
        }
    }
}

// ── Recipe Scaler Component (CP12) ───────────────────────────────────────────

/// Collapsible recipe scaling widget.
///
/// Supports two modes:
/// - Multiplier: user enters a direct multiplier (e.g. 2x, 0.5x)
/// - Target Ingredient: user picks an ingredient and desired amount
#[cfg(target_arch = "wasm32")]
#[component]
fn RecipeScaler(
    ingredients: Vec<RecipeIngredient>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
) -> Element {
    let mut is_expanded = use_signal(|| false);

    // Convert RecipeIngredient -> IngredientRef for ScaleCalculator
    let refs: Vec<IngredientRef> = ingredients
        .iter()
        .map(|ing| IngredientRef {
            amount: ing.amount.clone(),
            unit: ing.unit.clone(),
            name: ing.name.clone(),
        })
        .collect();

    let mut calculator = use_signal(|| {
        ScaleCalculator::new(refs.clone(), servings, prep_time_minutes, cook_time_minutes)
    });
    let mut multiplier_input = use_signal(String::new);
    let mut target_amount_input = use_signal(String::new);
    let mut target_ingredient_idx = use_signal(|| 0_usize);

    // Read calculator state early (clone to own the data)
    let current_mode = calculator.read().mode().clone();
    let current_multiplier = calculator.read().multiplier();

    // Preset multipliers — pre-compute all data for rsx! (no `let` allowed inside rsx!)
    let preset_data: Vec<(f64, &'static str, String)> = [
        (0.5, "½x"),
        (1.0, "1x"),
        (2.0, "2x"),
        (3.0, "3x"),
        (4.0, "4x"),
    ]
    .iter()
    .map(|(mult, label)| {
        let is_active = matches!(current_mode, ScaleMode::Multiplier(_))
            && (current_multiplier - mult).abs() < 0.001;
        let class = if is_active {
            "recipe-scaler__preset recipe-scaler__preset--active".to_string()
        } else {
            "recipe-scaler__preset".to_string()
        };
        (*mult, *label, class)
    })
    .collect();

    // Mode toggle handler
    let on_set_multiplier = move |_| {
        let val = multiplier_input.read().clone();
        if let Some(m) = parse_amount(&val) {
            calculator.with_mut(|calc| calc.set_multiplier(m));
        }
    };

    let on_set_target = move |_| {
        let val = target_amount_input.read().clone();
        if let Some(target) = parse_amount(&val) {
            let idx = target_ingredient_idx();
            calculator.with_mut(|calc| calc.set_target_ingredient(idx, target));
        }
    };

    let on_select_ingredient = move |e: Event<dioxus::events::FormData>| {
        if let Ok(idx) = e.value().parse::<usize>() {
            target_ingredient_idx.set(idx);
        }
    };

    let on_reset = move |_| {
        multiplier_input.set(String::new());
        target_amount_input.set(String::new());
        target_ingredient_idx.set(0);
        calculator.with_mut(|calc| calc.reset());
    };

    // Scaled ingredients — use ScaleCalculator's method (handles non-numeric correctly)
    let scaled_ingredients: Vec<ScaledIngredient> = calculator.read().scaled_ingredients();
    let scaled_servings = calculator.read().scaled_servings();
    let scaled_prep = calculator.read().scaled_prep_time();
    let scaled_cook = calculator.read().scaled_cook_time();
    let show_scaled = (current_multiplier - 1.0).abs() > 0.001;

    // Pre-compute display strings for scaled ingredients (rsx! can't handle multi-arg format strings)
    let scaled_ingredient_display: Vec<String> = scaled_ingredients
        .iter()
        .map(|ing| {
            if !ing.unit.is_empty() {
                format!("- {} {} {}", ing.amount, ing.unit, ing.name)
            } else if !ing.amount.is_empty() {
                format!("- {} {}", ing.amount, ing.name)
            } else {
                format!("- {}", ing.name)
            }
        })
        .collect();

    // Pre-computed option labels for target ingredient select
    let option_labels: Vec<String> = refs
        .iter()
        .map(|r| {
            let amt = parse_amount(&r.amount).unwrap_or(0.0);
            format!("- {} ({})", r.name, format_amount(amt))
        })
        .collect();

    rsx! {
        div { class: "recipe-scaler",
            // Toggle button
            button {
                class: "recipe-scaler__toggle",
                onclick: move |_| is_expanded.set(!is_expanded()),
                if is_expanded() {
                    "▼"
                } else {
                    "▶"
                }
                " Scale Recipe"
                span {
                    class: "recipe-scaler__badge",
                    "×{current_multiplier:.1}"
                }
            }

            // Collapsible body
            if is_expanded() {
                div { class: "recipe-scaler__body",
                    // Mode: Multiplier
                    div {
                        label {
                            display: "block",
                            font_size: "13px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-xs)",
                            "Multiplier"
                        }
                        div {
                            display: "flex",
                            gap: "var(--space-sm)",
                            align_items: "center",
                            input {
                                class: "recipe-scaler__input",
                                r#type: "text",
                                placeholder: "e.g. 2",
                                value: multiplier_input.read().clone(),
                                oninput: move |e| multiplier_input.set(e.value()),
                            }
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: on_set_multiplier,
                                "Apply"
                            }
                        }
                        // Presets
                        div { class: "recipe-scaler__presets",
                            for (mult, label, class) in &preset_data {
                                button {
                                    class: class.clone(),
                                    onclick: {
                                        let mult_val = *mult;
                                        move |_| {
                                            multiplier_input.set(format!("{mult_val}"));
                                            calculator.with_mut(|calc| calc.set_multiplier(mult_val));
                                        }
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }

                    // Mode: Target Ingredient
                    div {
                        margin_top: "var(--space-md)",
                        label {
                            display: "block",
                            font_size: "13px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-xs)",
                            "Or scale by target ingredient"
                        }
                        select {
                            class: "recipe-scaler__select",
                            onchange: on_select_ingredient,
                            for (idx, label) in option_labels.iter().enumerate() {
                                option { value: idx.to_string(), "{label}" }
                            }
                        }
                        div {
                            display: "flex",
                            gap: "var(--space-sm)",
                            align_items: "center",
                            margin_top: "var(--space-xs)",
                            input {
                                class: "recipe-scaler__input",
                                r#type: "text",
                                placeholder: "desired amount",
                                value: target_amount_input.read().clone(),
                                oninput: move |e| target_amount_input.set(e.value()),
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: on_set_target,
                                "Apply"
                            }
                        }
                    }

                    // Scaled meta info
                    if show_scaled {
                        div {
                            margin_top: "var(--space-md)",
                            padding_top: "var(--space-sm)",
                            border_top: "1px solid var(--surface)",
                            display: "flex",
                            flex_wrap: "wrap",
                            gap: "var(--space-sm)",
                            font_size: "13px",
                            color: "var(--text-secondary)",

                            if let Some(s) = scaled_servings {
                                span { "🍽 {s} servings" }
                            }
                            if let Some(p) = scaled_prep {
                                span { "⏱ Prep: {p} min" }
                            }
                            if let Some(c) = scaled_cook {
                                span { "🔥 Cook: {c} min" }
                            }
                        }
                    }

                    // Scaled ingredients
                    if show_scaled {
                        div {
                            margin_top: "var(--space-md)",
                            h3 {
                                font_size: "16px",
                                color: "var(--text-primary)",
                                margin_bottom: "var(--space-xs)",
                                "Scaled Ingredients"
                            }
                            ul {
                                list_style: "none",
                                padding: "0",
                                margin: "0",
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-xs)",
                                for text in &scaled_ingredient_display {
                                    li {
                                        padding: "var(--space-xs) var(--space-sm)",
                                        font_size: "14px",
                                        color: "var(--text-primary)",
                                        "{text}"
                                    }
                                }
                            }
                        }
                    }

                    // Reset button
                    button {
                        class: "recipe-scaler__reset",
                        onclick: on_reset,
                        "Reset to original"
                    }
                }
            }
        }
    }
}

// ── Helper: conditional RecipeScaler rendering ───────────────────────────────

/// Render the RecipeScaler widget on WASM; empty element on server.
fn render_recipe_scaler(
    ingredients: &[RecipeIngredient],
    #[allow(unused_variables)] prep_time_minutes: Option<i32>,
    #[allow(unused_variables)] cook_time_minutes: Option<i32>,
    #[allow(unused_variables)] servings: Option<i32>,
) -> Element {
    if ingredients.is_empty() {
        return rsx! {};
    }
    #[cfg(target_arch = "wasm32")]
    {
        rsx! {
            RecipeScaler {
                ingredients: ingredients.to_vec(),
                prep_time_minutes,
                cook_time_minutes,
                servings,
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rsx! {}
    }
}

// ── Helper: conditional Equipment rendering ────────────────────────────

/// Render the equipment section if present.
fn render_equipment(equipment: &[RecipeEquipment]) -> Element {
    if equipment.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "recipe-detail__equipment-section",
            h2 { class: "recipe-detail__section-title", "Equipment" }
            ul { class: "recipe-detail__list",
                for item in equipment {
                    li { class: "recipe-detail__list-item", "• {item.name}" }
                }
            }
        }
    }
}

// ── Image Slider ─────────────────────────────────────────────────────────────

fn render_image_slider(images: Vec<crate::types::RecipeImage>) -> Element {
    if images.is_empty() {
        return rsx! {};
    }

    let mut current_index = use_signal(|| 0);
    let total = images.len();

    // Pre-compute reactive values for rsx! (no closures allowed in attributes)
    let current_image = images[current_index()].clone();
    let show_left_arrow = current_index() > 0;
    let show_right_arrow = current_index() < total - 1;
    let dot_classes: Vec<String> = (0..total)
        .map(|i| {
            if i == current_index() {
                "recipe-image-slider__dot recipe-image-slider__dot--active".to_string()
            } else {
                "recipe-image-slider__dot".to_string()
            }
        })
        .collect();

    rsx! {
        div { class: "recipe-image-slider",
            div { class: "recipe-image-slider__viewport",
                EditedRecipeImage {
                    image: current_image,
                    class: "recipe-image-slider__image",
                    alt: "Recipe image".to_string(),
                    loading: "eager".to_string(),
                    fallback_fit: "contain".to_string(),
                    on_error: move |_| {},
                }
            }

            if total > 1 {
                if show_left_arrow {
                    button {
                        class: "recipe-image-slider__arrow recipe-image-slider__arrow--left",
                        onclick: move |_| {
                            current_index.set(current_index().saturating_sub(1));
                        },
                        "<"
                    }
                }

                if show_right_arrow {
                    button {
                        class: "recipe-image-slider__arrow recipe-image-slider__arrow--right",
                        onclick: move |_| {
                            current_index.set((current_index() + 1).min(total - 1));
                        },
                        ">"
                    }
                }

                div { class: "recipe-image-slider__dots",
                    for (i, cls) in dot_classes.iter().enumerate() {
                        button {
                            class: "{cls}",
                            onclick: move |_| { current_index.set(i); },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LineageRecipeNode(node: RecipeLineageNode, is_current: bool) -> Element {
    let node_class = if is_current {
        "recipe-lineage__node recipe-lineage__current"
    } else if node.is_redacted {
        "recipe-lineage__node recipe-lineage__node--redacted"
    } else if node.is_deleted {
        "recipe-lineage__node recipe-lineage__node--deleted"
    } else {
        "recipe-lineage__node"
    };
    let status = if node.is_redacted {
        Some("Private")
    } else if node.is_deleted {
        Some("Deleted")
    } else if is_current {
        Some("Current")
    } else {
        None
    };
    let recipe_id = node.recipe_id;
    let linked_node_class = format!("{node_class} recipe-lineage__node--linked");
    let node_content = rsx! {
        if let Some(image_url) = &node.image_url {
            img {
                class: "recipe-lineage__thumbnail",
                src: image_url.clone(),
                alt: "",
                loading: "lazy",
            }
        } else {
            div {
                class: "recipe-lineage__thumbnail recipe-lineage__thumbnail-placeholder",
                aria_hidden: "true",
                "🍽"
            }
        }
        div { class: "recipe-lineage__node-content",
            div { class: "recipe-lineage__node-heading",
                strong { class: "recipe-lineage__node-title", "{node.title}" }
                if let Some(status) = status {
                    span { class: "recipe-lineage__status", "{status}" }
                }
            }
            if let Some(username) = &node.owner_username {
                span { class: "recipe-lineage__owner", "@{username}" }
            }
            if let Some(version_number) = node.forked_from_version_number {
                span { class: "recipe-lineage__fork-point", "Forked from v{version_number}" }
            }
            if let Some(message) = &node.fork_message {
                if !message.trim().is_empty() {
                    p { class: "recipe-lineage__message", "{message}" }
                }
            }
        }
    };

    rsx! {
        if let Some(recipe_id) = recipe_id {
            a {
                href: format!("/recipes/{recipe_id}"),
                class: linked_node_class,
                aria_label: format!("Open recipe: {}", node.title),
                {node_content}
            }
        } else {
            article {
                class: node_class,
                {node_content}
            }
        }
    }
}

fn descendant_branch_has_more(branch: &RecipeLineageBranch, depth: usize) -> bool {
    branch.truncated || (depth >= LINEAGE_DESCENDANT_DISPLAY_DEPTH && !branch.children.is_empty())
}

fn descendant_paths_breadth_first(
    descendants: &[RecipeLineageBranch],
    maximum_depth: usize,
) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    let mut pending = VecDeque::new();

    for (root_index, branch) in descendants.iter().enumerate() {
        pending.push_back((vec![root_index], branch, 1));
    }

    while let Some((path, branch, depth)) = pending.pop_front() {
        paths.push(path.clone());
        if depth >= maximum_depth {
            continue;
        }

        for (child_index, child) in branch.children.iter().enumerate() {
            let mut child_path = path.clone();
            child_path.push(child_index);
            pending.push_back((child_path, child, depth + 1));
        }
    }

    paths
}

fn path_is_strict_descendant(path: &[usize], ancestor_path: &[usize]) -> bool {
    path.len() > ancestor_path.len() && path.starts_with(ancestor_path)
}

fn next_hidden_descendant_paths(
    all_paths: &[Vec<usize>],
    visible_paths: &[Vec<usize>],
    ancestor_path: &[usize],
) -> Vec<Vec<usize>> {
    all_paths
        .iter()
        .filter(|path| {
            path_is_strict_descendant(path, ancestor_path) && !visible_paths.contains(path)
        })
        .take(LINEAGE_DESCENDANT_EXPANSION_SIZE)
        .cloned()
        .collect()
}

fn branch_has_hidden_direct_children(
    branch: &RecipeLineageBranch,
    branch_path: &[usize],
    visible_paths: &[Vec<usize>],
) -> bool {
    branch.children.iter().enumerate().any(|(index, _)| {
        let mut child_path = branch_path.to_vec();
        child_path.push(index);
        !visible_paths.contains(&child_path)
    })
}

fn collapse_revealed_descendant_paths(
    revealed_paths: &[Vec<usize>],
    branch_path: &[usize],
) -> Vec<Vec<usize>> {
    revealed_paths
        .iter()
        .filter(|path| !path_is_strict_descendant(path, branch_path))
        .cloned()
        .collect()
}

#[component]
fn LineageDescendant(
    branch: RecipeLineageBranch,
    depth: usize,
    path: Vec<usize>,
    visible_paths: Vec<Vec<usize>>,
    all_descendant_paths: Vec<Vec<usize>>,
    initial_visible_paths: Vec<Vec<usize>>,
    mut revealed_descendant_paths: Signal<Vec<Vec<usize>>>,
) -> Element {
    let branch_subtree_id = format!(
        "fork-lineage-descendants-{}",
        path.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("-"),
    );
    let visible_children = if depth < LINEAGE_DESCENDANT_DISPLAY_DEPTH {
        branch
            .children
            .iter()
            .enumerate()
            .filter_map(|(child_index, child)| {
                let mut child_path = path.clone();
                child_path.push(child_index);
                visible_paths
                    .contains(&child_path)
                    .then(|| (child.clone(), child_path))
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let has_more = descendant_branch_has_more(&branch, depth);

    rsx! {
        li { class: "recipe-lineage__branch",
            LineageRecipeNode { node: branch.node.clone(), is_current: false }
            div {
                id: branch_subtree_id.clone(),
                class: "recipe-lineage__subtree",
                if !visible_children.is_empty() {
                    ul { class: "recipe-lineage__children",
                        for (child, child_path) in visible_children {
                            LineageDescendant {
                                branch: child,
                                depth: depth + 1,
                                path: child_path,
                                visible_paths: visible_paths.clone(),
                                all_descendant_paths: all_descendant_paths.clone(),
                                initial_visible_paths: initial_visible_paths.clone(),
                                revealed_descendant_paths,
                            }
                        }
                    }
                }
            }
            {
                let branch_has_hidden_children = branch_has_hidden_direct_children(
                    &branch,
                    &path,
                    &visible_paths,
                );
                let branch_next_paths = next_hidden_descendant_paths(
                    &all_descendant_paths,
                    &visible_paths,
                    &path,
                );
                let branch_next_batch_size = branch_next_paths.len();
                let branch_recipe_noun = if branch_next_batch_size == 1 {
                    "recipe"
                } else {
                    "recipes"
                };
                let branch_has_revealed_descendants = revealed_descendant_paths()
                    .iter()
                    .any(|revealed_path| path_is_strict_descendant(revealed_path, &path));
                let branch_title = branch.node.title.clone();
                let expand_label = format!(
                    "Show {branch_next_batch_size} more {branch_recipe_noun} forked from {branch_title}",
                );
                let collapse_label = format!("Collapse descendants of {branch_title}");
                let path_for_collapse = path.clone();

                rsx! {
                    if branch_has_hidden_children || branch_has_revealed_descendants {
                        div { class: "recipe-lineage__branch-controls",
                            if branch_has_hidden_children {
                                button {
                                    r#type: "button",
                                    class: "recipe-lineage__control-button recipe-lineage__control-button--branch",
                                    aria_label: expand_label,
                                    aria_controls: branch_subtree_id.clone(),
                                    aria_expanded: branch_has_revealed_descendants,
                                    onclick: move |_| {
                                        let mut revealed_paths = revealed_descendant_paths();
                                        for path in &branch_next_paths {
                                            if !initial_visible_paths.contains(path)
                                                && !revealed_paths.contains(path)
                                            {
                                                revealed_paths.push(path.clone());
                                            }
                                        }
                                        revealed_descendant_paths.set(revealed_paths);
                                    },
                                    "Show {branch_next_batch_size} more"
                                }
                            }
                            if branch_has_revealed_descendants {
                                button {
                                    r#type: "button",
                                    class: "recipe-lineage__control-button recipe-lineage__control-button--branch recipe-lineage__control-button--secondary",
                                    aria_label: collapse_label,
                                    aria_controls: branch_subtree_id.clone(),
                                    onclick: move |_| {
                                        revealed_descendant_paths.set(
                                            collapse_revealed_descendant_paths(
                                                &revealed_descendant_paths(),
                                                &path_for_collapse,
                                            ),
                                        );
                                    },
                                    "Collapse"
                                }
                            }
                        }
                    }
                }
            }
            if has_more {
                span {
                    class: "recipe-lineage__truncated",
                    role: "status",
                    "More recipes forked from this branch"
                }
            }
        }
    }
}

#[component]
fn ForkLineage(lineage: RecipeLineageView) -> Element {
    let mut revealed_descendant_paths = use_signal(Vec::<Vec<usize>>::new);
    let all_descendant_paths =
        descendant_paths_breadth_first(&lineage.descendants, LINEAGE_DESCENDANT_DISPLAY_DEPTH);
    let total_descendant_count = all_descendant_paths.len();
    let initial_visible_paths = all_descendant_paths
        .iter()
        .take(LINEAGE_INITIAL_VISIBLE_DESCENDANTS)
        .cloned()
        .collect::<Vec<_>>();
    let mut visible_paths = initial_visible_paths.clone();
    for path in revealed_descendant_paths() {
        if !visible_paths.contains(&path) {
            visible_paths.push(path);
        }
    }
    let displayed_descendant_count = visible_paths.len();
    let visible_roots = lineage
        .descendants
        .iter()
        .enumerate()
        .filter_map(|(root_index, branch)| {
            let path = vec![root_index];
            visible_paths
                .contains(&path)
                .then(|| (branch.clone(), path))
        })
        .collect::<Vec<_>>();
    let hidden_descendant_count = total_descendant_count - displayed_descendant_count;
    let root_next_paths = next_hidden_descendant_paths(&all_descendant_paths, &visible_paths, &[]);
    let root_next_batch_size = root_next_paths.len();
    let root_recipe_noun = if root_next_batch_size == 1 {
        "recipe"
    } else {
        "recipes"
    };
    let can_collapse = !revealed_descendant_paths().is_empty();
    let root_title = lineage.current.title.clone();
    let root_expand_label =
        format!("Show {root_next_batch_size} more {root_recipe_noun} forked from {root_title}",);

    rsx! {
        Card {
            section {
                class: "recipe-lineage",
                aria_labelledby: "fork-lineage-title",
                div { class: "recipe-lineage__header",
                    div {
                        h2 {
                            id: "fork-lineage-title",
                            class: "recipe-detail__section-title",
                            "Fork lineage"
                        }
                        p {
                            "Trace this recipe back to its source and explore recipes forked from it."
                        }
                    }
                }

                div { class: "recipe-lineage__flow",
                    div { class: "recipe-lineage__group recipe-lineage__ancestors",
                        h3 { "Ancestors" }
                        if lineage.ancestors.is_empty() {
                            p { class: "recipe-lineage__empty", "This is the original recipe." }
                        }
                        ol { class: "recipe-lineage__ancestor-list",
                            if !lineage.ancestors.is_empty() {
                                for ancestor in lineage.ancestors.clone() {
                                    li {
                                        LineageRecipeNode {
                                            node: ancestor,
                                            is_current: false,
                                        }
                                    }
                                }
                            }
                            li { class: "recipe-lineage__ancestor-current",
                                LineageRecipeNode {
                                    node: lineage.current.clone(),
                                    is_current: true,
                                }
                            }
                        }
                    }

                    div { class: "recipe-lineage__group recipe-lineage__descendants",
                        h3 { "Descendants" }
                        if lineage.descendants.is_empty() {
                            p { class: "recipe-lineage__empty", "No published forks yet." }
                        } else {
                            ul {
                                id: "fork-lineage-descendants",
                                class: "recipe-lineage__descendant-list",
                                for (descendant, path) in visible_roots {
                                    LineageDescendant {
                                        branch: descendant,
                                        depth: 1,
                                        path,
                                        visible_paths: visible_paths.clone(),
                                        all_descendant_paths: all_descendant_paths.clone(),
                                        initial_visible_paths: initial_visible_paths.clone(),
                                        revealed_descendant_paths,
                                    }
                                }
                            }

                            if hidden_descendant_count > 0 || can_collapse {
                                div { class: "recipe-lineage__controls",
                                    span {
                                        class: "recipe-lineage__count",
                                        aria_live: "polite",
                                        "Showing {displayed_descendant_count} of {total_descendant_count} descendants"
                                    }
                                    div { class: "recipe-lineage__control-actions",
                                        if hidden_descendant_count > 0 {
                                            button {
                                                r#type: "button",
                                                class: "recipe-lineage__control-button",
                                                aria_controls: "fork-lineage-descendants",
                                                aria_label: root_expand_label,
                                                aria_expanded: can_collapse,
                                                onclick: move |_| {
                                                    let mut revealed_paths = revealed_descendant_paths();
                                                    for path in &root_next_paths {
                                                        if !initial_visible_paths.contains(path)
                                                            && !revealed_paths.contains(path)
                                                        {
                                                            revealed_paths.push(path.clone());
                                                        }
                                                    }
                                                    revealed_descendant_paths.set(revealed_paths);
                                                },
                                                "Show {root_next_batch_size} more"
                                            }
                                        }
                                        if can_collapse {
                                            button {
                                                r#type: "button",
                                                class: "recipe-lineage__control-button recipe-lineage__control-button--secondary",
                                                aria_controls: "fork-lineage-descendants",
                                                onclick: move |_| {
                                                    revealed_descendant_paths.set(Vec::new());
                                                },
                                                "Collapse to first {LINEAGE_INITIAL_VISIBLE_DESCENDANTS}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if lineage.node_limit_reached {
                    p {
                        class: "recipe-lineage__limit-notice",
                        role: "status",
                        "This lineage is too large to display in full. Showing the first branches only."
                    }
                }
            }
        }
    }
}

// ── Component ────────────────────────────────────────────────────────────────

/// Single recipe detail page.
///
/// Publicly accessible: tries public endpoint first, falls back to authenticated.
#[component]
pub fn RecipeDetail(id: String) -> Element {
    // ── Resources ────────────────────────────────────────────────────────
    let auth = use_auth();

    // Clone id before each use_resource to avoid move issues.
    // The closure is FnMut (called potentially multiple times), so we clone
    // the captured value inside the closure body before the async block.
    let id_for_recipe = id.clone();
    let auth_for_recipe = auth.clone();
    let mut recipe_resource = use_resource(move || {
        let rid = id_for_recipe.clone();
        let auth = auth_for_recipe.clone();
        async move {
            match get_public_recipe(rid.clone()).await {
                Ok(recipe) => Ok(recipe),
                Err(public_error) if auth.is_authenticated => {
                    tracing::debug!(
                        error = ?public_error,
                        "Public recipe lookup failed; retrying the owner endpoint"
                    );
                    get_recipe(rid).await
                }
                Err(public_error) => Err(public_error),
            }
        }
    });

    // Owner username resource
    let id_for_owner = id.clone();
    let owner_username_resource = use_resource(move || {
        let oid = id_for_owner.clone();
        async move { get_recipe_owner_username(oid).await }
    });
    let id_for_attribution = id.clone();
    let attribution_resource = use_resource(move || {
        let recipe_id = id_for_attribution.clone();
        async move { get_fork_attribution(recipe_id).await }
    });
    let id_for_versions = id.clone();
    let versions_resource = use_resource(move || {
        let recipe_id = id_for_versions.clone();
        async move { get_recipe_versions_api(recipe_id).await }
    });
    let id_for_lineage = id.clone();
    let lineage_resource = use_resource(move || {
        let recipe_id = id_for_lineage.clone();
        let descendant_depth = LINEAGE_DESCENDANT_DISPLAY_DEPTH as i32;
        async move { get_recipe_lineage(recipe_id, Some(descendant_depth)).await }
    });
    let mut version_action_error = use_signal(|| Option::<String>::None);
    let mut expanded_version_difference = use_signal(|| Option::<i32>::None);
    let mut version_difference = use_signal(|| Option::<RecipeVersionComparison>::None);
    let mut version_difference_error = use_signal(|| Option::<String>::None);
    let mut is_loading_version_difference = use_signal(|| false);
    let mut is_forking = use_signal(|| false);
    let mut selected_visibility = use_signal(|| "private".to_string());
    let mut is_saving_visibility = use_signal(|| false);
    let mut visibility_feedback = use_signal(|| Option::<Result<(), String>>::None);

    // Tags (signal-based, loaded conditionally based on ownership)
    let tags = use_signal(|| Option::<Vec<String>>::None);
    let tags_error = use_signal(|| Option::<String>::None);

    use_effect(move || {
        let recipe_result = recipe_resource.read().clone();
        if let Some(Ok(recipe)) = recipe_result {
            if !is_saving_visibility()
                && visibility_selection_differs_from_recipe(selected_visibility, &recipe.visibility)
            {
                selected_visibility.set(recipe.visibility);
            }
        }
    });

    // Load tags once recipe is available
    let id_for_tags_effect = id.clone();
    use_effect(move || {
        let recipe_result = recipe_resource.read().clone();
        let Some(Ok(ref recipe)) = recipe_result else {
            return;
        };

        let auth = use_auth();
        let is_owner = auth.current_user_id == Some(recipe.user_id);
        let rid = id_for_tags_effect.clone();
        let mut tag_signal = tags;
        let mut tag_err = tags_error;

        spawn(async move {
            if is_owner {
                match get_recipe_tags(rid).await {
                    Ok(t) => tag_signal.set(Some(t)),
                    Err(e) => tag_err.set(Some(e.to_string())),
                }
            } else {
                match get_public_recipe_tags_by_id(rid).await {
                    Ok(t) => tag_signal.set(Some(t)),
                    Err(e) => tag_err.set(Some(e.to_string())),
                }
            }
        });
    });

    // ── Delete state ─────────────────────────────────────────────────────
    #[allow(unused_mut)]
    let mut is_deleting = use_signal(|| false);
    #[allow(unused_mut)]
    let mut delete_error = use_signal(|| Option::<String>::None);

    // ── Extract resource states ──────────────────────────────────────────
    let recipe_pending = recipe_resource.pending();
    let any_pending = recipe_pending;

    // Read resource results to avoid borrow issues in rsx!
    let recipe_result: Option<Result<Recipe, ServerFnError>> = recipe_resource.read().clone();

    // ── Derived state ────────────────────────────────────────────────────
    let recipe = recipe_result.as_ref().and_then(|r| r.as_ref().ok());

    // Determine error message (if any)
    let error_message = if let Some(Err(e)) = &recipe_result {
        let msg = match e {
            ServerFnError::ServerError { message, .. } => message.clone(),
            _ => e.to_string(),
        };
        Some(msg)
    } else {
        None
    };

    // ── Auth context for ownership check ─────────────────────────────────
    let is_owner = recipe
        .map(|r| auth.current_user_id == Some(r.user_id))
        .unwrap_or(false);

    // Owner username: use owner's own username if they're viewing,
    // otherwise use the fetched owner username from the resource.
    let owner_username = if is_owner {
        auth.current_user.as_ref().map(|u| u.username.clone())
    } else {
        owner_username_resource
            .read()
            .clone()
            .and_then(|r| r.ok())
            .flatten()
    };
    let attribution: Option<ForkAttribution> = attribution_resource
        .read()
        .clone()
        .and_then(Result::ok)
        .flatten();
    let versions: Vec<RecipeVersionSummary> = versions_resource
        .read()
        .clone()
        .and_then(Result::ok)
        .unwrap_or_default();
    let lineage = lineage_resource.read().clone().and_then(Result::ok);
    let recipe_id_for_visibility = id.clone();
    let save_visibility = move |_| {
        let recipe_id = recipe_id_for_visibility.clone();
        let visibility = selected_visibility();
        is_saving_visibility.set(true);
        visibility_feedback.set(None);
        spawn(async move {
            match update_recipe_visibility(recipe_id, visibility).await {
                Ok(_) => {
                    visibility_feedback.set(Some(Ok(())));
                    recipe_resource.restart();
                }
                Err(error) => {
                    visibility_feedback.set(Some(Err(error.to_string())));
                }
            }
            is_saving_visibility.set(false);
        });
    };

    let id_for_fork = id.clone();
    let on_fork = move |_| {
        is_forking.set(true);
        version_action_error.set(None);
        let recipe_id = id_for_fork.clone();
        spawn(async move {
            match fork_recipe(recipe_id, None).await {
                #[allow(unused_variables)]
                Ok(draft) =>
                {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(window) = web_sys::window() {
                        let _ = window
                            .location()
                            .set_href(&format!("/recipes/drafts/{}/edit", draft.id));
                    }
                }
                Err(error) => {
                    version_action_error.set(Some(error.to_string()));
                    is_forking.set(false);
                }
            }
        });
    };

    // ── Delete handler ───────────────────────────────────────────────────
    // Clone id before the move closure (id was already moved into use_resource closures)
    #[allow(unused_variables)]
    let id_for_delete = id.clone();
    let on_delete = move |_| {
        // Clone inside the FnMut closure so it doesn't move out
        #[allow(unused_variables)]
        let id_clone = id_for_delete.clone();
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                // Note: web_sys `confirm(msg)` requires the "confirm" feature.
                // Using no-arg version which shows a generic confirmation dialog.
                let confirmed = window.confirm().unwrap_or(false);
                if confirmed {
                    is_deleting.set(true);
                    delete_error.set(None);
                    spawn(async move {
                        match delete_recipe(id_clone).await {
                            Ok(()) => {
                                if let Some(w) = web_sys::window() {
                                    let _ = w.location().set_href("/dashboard");
                                }
                            }
                            Err(e) => {
                                let msg = match &e {
                                    ServerFnError::ServerError { message, .. } => message.clone(),
                                    _ => e.to_string(),
                                };
                                delete_error.set(Some(msg));
                                is_deleting.set(false);
                            }
                        }
                    });
                }
            }
        }
    };

    // ── Render: loading ──────────────────────────────────────────────────
    if any_pending {
        return rsx! {
            div { class: "container",
                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "300px",
                    LoadingSpinner {}
                    p {
                        margin_top: "var(--space-md)",
                        color: "var(--text-secondary)",
                        "Loading recipe..."
                    }
                }
            }
        };
    }

    // ── Render: error ────────────────────────────────────────────────────
    if let Some(err_msg) = &error_message {
        return rsx! {
            div { class: "container",
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        text_align: "center",
                        gap: "var(--space-md)",
                        p {
                            color: "var(--error)",
                            font_weight: "600",
                            font_size: "18px",
                            "{err_msg}"
                        }
                        Link {
                            to: crate::Route::Dashboard {},
                            class: "btn btn-secondary touch-target",
                            "Back to Recipes"
                        }
                    }
                }
            }
        };
    }

    // ── Render: content ──────────────────────────────────────────────────
    let Some(recipe) = recipe else {
        // Should not reach here if logic is correct, but guard anyway
        return rsx! {
            div { class: "container",
                Card {
                    div {
                        text_align: "center",
                        color: "var(--error)",
                        "Unable to load recipe."
                    }
                }
            }
        };
    };

    let relative_time = format_relative_time(recipe.created_at);
    let fork_sign_in_url = fork_sign_in_url(&recipe.id.to_string());

    rsx! {
        div { class: "recipe-detail container",

            // ── CARD: Header + Overview (all in one card) ────────────────
            Card {
                PageHeader {
                    title: "{recipe.title}",
                    action: Some(rsx! {
                            div {
                                display: "flex",
                                gap: "var(--space-sm)",
                                Link {
                                    to: crate::Route::RecipeVersionGraph { id: recipe.id.to_string() },
                                    class: "btn btn-secondary touch-target",
                                    "View version graph"
                                }
                                if auth.is_authenticated {
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        disabled: is_forking(),
                                        onclick: on_fork,
                                        if is_forking() { "Forking..." } else { "Fork" }
                                    }
                                } else {
                                    a {
                                        href: fork_sign_in_url.clone(),
                                        class: "btn btn-secondary touch-target",
                                        "Fork"
                                    }
                                }
                                if is_owner {
                                    Link {
                                        to: crate::Route::RecipeEdit { id: recipe.id.to_string() },
                                        class: "btn btn-secondary touch-target",
                                        "Edit"
                                    }
                                    Button {
                                        variant: ButtonVariant::Danger,
                                        disabled: is_deleting(),
                                        onclick: on_delete,
                                        if is_deleting() { "Deleting..." } else { "Delete" }
                                    }
                                }
                            }
                        }),
                }

                if let Some(source) = &attribution {
                    div {
                        class: "recipe-detail__tags",
                        if let Some(source_id) = source.original_recipe_id {
                            Link {
                                to: crate::Route::RecipeDetail { id: source_id.to_string() },
                                "Forked from {source.original_title}"
                            }
                        } else {
                            span { "Forked from {source.original_title}" }
                        }
                        if let Some(message) = &source.message {
                            span { " · {message}" }
                        }
                    }
                }

                // Tags
                if let Some(ref tag_list) = tags() {
                    if !tag_list.is_empty() {
                        div { class: "recipe-detail__tags",
                            for tag in tag_list {
                                span { class: "recipe-detail__tag", "{tag}" }
                            }
                        }
                    }
                }

                // Meta row
                div { class: "recipe-detail__meta-row",
                    if let Some(prepare) = recipe.prep_time_minutes {
                        span { class: "recipe-detail__meta-item", "⏱ Prep: {prepare} min" }
                    }
                    if let Some(cook) = recipe.cook_time_minutes {
                        span { class: "recipe-detail__meta-item", "🔥 Cook: {cook} min" }
                    }
                    if let Some(serv) = recipe.servings {
                        span { class: "recipe-detail__meta-item", "🍽 Servings: {serv}" }
                    }
                }

                if is_owner {
                    div {
                        class: "recipe-detail__visibility-control neumo-inset",
                        label {
                            r#for: "recipe-visibility",
                            "Recipe visibility"
                        }
                        select {
                            id: "recipe-visibility",
                            class: "input",
                            value: selected_visibility(),
                            disabled: is_saving_visibility(),
                            onchange: move |event| {
                                selected_visibility.set(event.value());
                                visibility_feedback.set(None);
                            },
                            option { value: "private", "Private" }
                            option { value: "unlisted", "Unlisted" }
                            option { value: "public", "Public" }
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: is_saving_visibility()
                                || selected_visibility() == recipe.visibility,
                            onclick: save_visibility,
                            if is_saving_visibility() { "Saving..." } else { "Save visibility" }
                        }
                        if let Some(feedback) = visibility_feedback() {
                            match feedback {
                                Ok(()) => rsx! {
                                    span {
                                        class: "recipe-detail__visibility-success",
                                        role: "status",
                                        "Visibility saved."
                                    }
                                },
                                Err(error) => rsx! {
                                    span {
                                        class: "recipe-detail__visibility-error",
                                        role: "alert",
                                        "{error}"
                                    }
                                },
                            }
                        }
                    }
                }

                // Description
                if let Some(desc) = &recipe.description {
                    if !desc.is_empty() {
                        p { class: "recipe-detail__description", "{desc}" }
                    }
                }

                // Author line
                div { class: "recipe-detail__author-line",
                    if let Some(username) = &owner_username {
                        "by "
                        Link {
                            to: crate::Route::UserProfile { username: username.clone() },
                            class: "recipe-detail__author-link",
                            "@{username}"
                        }
                        " • created {relative_time}"
                    } else {
                        "created {relative_time}"
                    }
                }
            }

            // ── Image Slider (conditional) ────────────────────────────────
            {render_image_slider(recipe.images.clone())}

            // ── Delete error (NOT in a card — transient UI element) ──────
            if let Some(del_err) = delete_error() {
                div { class: "recipe-detail__delete-error", "{del_err}" }
            }
            if let Some(action_error) = version_action_error() {
                div { class: "recipe-detail__delete-error", "{action_error}" }
            }

            // ── CARD 4: Equipment (conditional) ──────────────────────────
            if !recipe.equipment.is_empty() {
                Card {
                    {render_equipment(&recipe.equipment)}
                }
            }

            // ── CARD 5: Ingredients + Scaler (conditional) ───────────────
            if !recipe.ingredients.is_empty() {
                Card {
                    div { class: "recipe-detail__ingredients-section",
                        h2 { class: "recipe-detail__section-title", "Ingredients" }
                        ul { class: "recipe-detail__list",
                            for ing in &recipe.ingredients {
                                li { class: "recipe-detail__list-item",
                                    if !ing.amount.is_empty() && !ing.unit.is_empty() {
                                        "- {ing.amount} {ing.unit} {ing.name}"
                                    } else if !ing.amount.is_empty() {
                                        "- {ing.amount} {ing.name}"
                                    } else {
                                        "- {ing.name}"
                                    }
                                }
                            }
                        }
                    }
                    {render_recipe_scaler(&recipe.ingredients, recipe.prep_time_minutes, recipe.cook_time_minutes, recipe.servings)}
                }
            }

            // ── CARD 6: Steps (conditional) ──────────────────────────────
            if !recipe.instructions.is_empty() {
                Card {
                    div { class: "recipe-detail__steps-section",
                        h2 { class: "recipe-detail__section-title", "Steps" }
                        ol { class: "recipe-detail__steps-list",
                            for (idx, step) in recipe.instructions.iter().enumerate() {
                                StepNode { step: step.clone(), path: vec![idx], level: 0 }
                            }
                        }
                    }
                }
            }

            // ── CARD 7: Commentary (conditional) ─────────────────────────
            if let Some(comm) = &recipe.commentary {
                if !comm.is_empty() {
                    Card {
                        p { class: "recipe-detail__commentary", "{comm}" }
                    }
                }
            }

            if let Some(lineage) = &lineage {
                if !lineage.ancestors.is_empty() || !lineage.descendants.is_empty() {
                    ForkLineage { key: "{id}", lineage: lineage.clone() }
                }
            }

            if is_owner && !versions.is_empty() {
                Card {
                    h2 { "Version history" }
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        for version in versions.clone() {
                            {
                                let version_number = version.version_number;
                                let previous_version_number = version_number - 1;
                                let latest_version_number = versions[0].version_number;
                                let difference_is_expanded =
                                    expanded_version_difference() == Some(version_number);
                                rsx! {
                                    div {
                                        key: "{version_number}",
                                        padding: "var(--space-sm) 0",
                                        border_bottom: "1px solid var(--border-color)",
                                        div {
                                            display: "flex",
                                            justify_content: "space-between",
                                            align_items: "center",
                                            flex_wrap: "wrap",
                                            gap: "var(--space-sm)",
                                            div { class: "recipe-version-history__summary",
                                                if let Some(image_url) = &version.image_url {
                                                    img {
                                                        class: "recipe-version-history__thumbnail",
                                                        src: image_url.clone(),
                                                        alt: "",
                                                        loading: "lazy",
                                                    }
                                                } else {
                                                    div {
                                                        class: "recipe-version-history__thumbnail recipe-version-history__thumbnail-placeholder",
                                                        aria_hidden: "true",
                                                        "🍽"
                                                    }
                                                }
                                                div { class: "recipe-version-history__details",
                                                    strong {
                                                        "v{version_number} · {version.title}"
                                                        if version.is_current { " · Current" }
                                                    }
                                                    p { "Published {version.created_at}" }
                                                    p { "{version.change_summary}" }
                                                    if let Some(notes) = &version.notes {
                                                        p { "{notes}" }
                                                    }
                                                }
                                            }
                                            div {
                                                display: "flex",
                                                width: "100%",
                                                flex: "0 0 100%",
                                                flex_wrap: "wrap",
                                                justify_content: "flex-start",
                                                gap: "var(--space-xs)",
                                                if version_number > 1 {
                                                    Button {
                                                        variant: ButtonVariant::Secondary,
                                                        disabled: is_loading_version_difference(),
                                                        onclick: {
                                                            let recipe_id = id.clone();
                                                            move |_| {
                                                                if expanded_version_difference()
                                                                    == Some(version_number)
                                                                {
                                                                    expanded_version_difference.set(None);
                                                                    version_difference.set(None);
                                                                    version_difference_error.set(None);
                                                                    return;
                                                                }

                                                                expanded_version_difference
                                                                    .set(Some(version_number));
                                                                version_difference.set(None);
                                                                version_difference_error.set(None);
                                                                is_loading_version_difference.set(true);
                                                                let recipe_id = recipe_id.clone();
                                                                spawn(async move {
                                                                    match compare_recipe_versions_api(
                                                                        recipe_id,
                                                                        previous_version_number,
                                                                        version_number,
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(detail)
                                                                            if expanded_version_difference()
                                                                                == Some(version_number) =>
                                                                        {
                                                                            version_difference.set(Some(detail));
                                                                        }
                                                                        Err(error)
                                                                            if expanded_version_difference()
                                                                                == Some(version_number) =>
                                                                        {
                                                                            version_difference_error
                                                                                .set(Some(error.to_string()));
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                    if expanded_version_difference()
                                                                        == Some(version_number)
                                                                    {
                                                                        is_loading_version_difference.set(false);
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        if difference_is_expanded {
                                                            if is_loading_version_difference() {
                                                                "Loading differences..."
                                                            } else {
                                                                "Hide differences"
                                                            }
                                                        } else {
                                                            "View differences"
                                                        }
                                                    }
                                                }
                                                Link {
                                                    to: crate::Route::RecipeVersionDetail {
                                                        id: id.clone(),
                                                        version_number,
                                                    },
                                                    class: "btn btn-ghost touch-target",
                                                    "View snapshot"
                                                }
                                                if !version.is_current {
                                                    Button {
                                                        variant: ButtonVariant::Secondary,
                                                        onclick: {
                                                            let recipe_id = id.clone();
                                                            move |_| {
                                                                let recipe_id = recipe_id.clone();
                                                                spawn(async move {
                                                                    match restore_version(
                                                                        recipe_id,
                                                                        version_number,
                                                                        latest_version_number,
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(_) => {
                                                                            #[cfg(target_arch = "wasm32")]
                                                                            if let Some(window) = web_sys::window() {
                                                                                let _ = window.location().reload();
                                                                            }
                                                                        }
                                                                        Err(error) => version_action_error
                                                                            .set(Some(error.to_string())),
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        "Restore"
                                                    }
                                                }
                                            }
                                        }
                                        if difference_is_expanded {
                                            div { margin_top: "var(--space-md)",
                                                if is_loading_version_difference() {
                                                    div {
                                                        display: "flex",
                                                        align_items: "center",
                                                        gap: "var(--space-sm)",
                                                        padding: "var(--space-md)",
                                                        LoadingSpinner {}
                                                        span { "Loading differences..." }
                                                    }
                                                } else if let Some(error) = version_difference_error() {
                                                    p {
                                                        padding: "var(--space-md)",
                                                        color: "var(--error)",
                                                        background: "var(--error-bg)",
                                                        border_radius: "var(--radius-md)",
                                                        "Unable to load differences: {error}"
                                                    }
                                                } else if let Some(detail) = version_difference() {
                                                    VersionComparisonPanel {
                                                        title: format!("Changes in v{version_number}"),
                                                        description: format!(
                                                            "Published differences from v{previous_version_number} to v{version_number}. Red shows removed content; green shows added content.",
                                                        ),
                                                        changes: detail.changes,
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Full-page, read-only rendering of one immutable published recipe snapshot.
#[component]
pub fn RecipeVersionDetail(id: String, version_number: i32) -> Element {
    let recipe_id = id.clone();
    let version_resource = use_resource(move || {
        let recipe_id = recipe_id.clone();
        async move { get_recipe_version_api(recipe_id, version_number).await }
    });
    let versions_recipe_id = id.clone();
    let versions_resource = use_resource(move || {
        let recipe_id = versions_recipe_id.clone();
        async move { get_recipe_versions_api(recipe_id).await }
    });
    let mut comparison_from = use_signal(|| 1i32);
    let mut comparison_to = use_signal(|| version_number);
    let mut custom_comparison = use_signal(|| Option::<RecipeVersionComparison>::None);
    let mut comparison_error = use_signal(|| Option::<String>::None);
    let mut is_comparing = use_signal(|| false);
    // Subscribe to the completed value before an early loading return. Reading
    // only `pending()` here can leave this route rendered in its initial state
    // even after the server function has completed.
    let version_result: Option<Result<RecipeVersionView, ServerFnError>> =
        version_resource.read().clone();
    let versions_result: Option<Result<Vec<RecipeVersionSummary>, ServerFnError>> =
        versions_resource.read().clone();

    if version_resource.pending() {
        return rsx! {
            div { class: "container",
                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "300px",
                    LoadingSpinner {}
                    p { margin_top: "var(--space-md)", "Loading historical version..." }
                }
            }
        };
    }

    let Some(Ok(detail)) = version_result else {
        let error_message = version_result
            .and_then(Result::err)
            .map(|error| error.to_string())
            .unwrap_or_else(|| "Unable to load this recipe version.".to_string());
        return rsx! {
            div { class: "container",
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        gap: "var(--space-md)",
                        p { color: "var(--error)", font_weight: "600", "{error_message}" }
                        Link {
                            to: crate::Route::RecipeDetail { id: id.clone() },
                            class: "btn btn-secondary touch-target",
                            "Back to current recipe"
                        }
                    }
                }
            }
        };
    };
    let editor = &detail.editor;
    let versions = versions_result.and_then(Result::ok).unwrap_or_default();
    let recipe_id_for_comparison = id.clone();
    let compare_selected_versions = move |_| {
        let recipe_id = recipe_id_for_comparison.clone();
        let from_version_number = comparison_from();
        let to_version_number = comparison_to();
        is_comparing.set(true);
        comparison_error.set(None);
        spawn(async move {
            match compare_recipe_versions_api(recipe_id, from_version_number, to_version_number)
                .await
            {
                Ok(comparison) => custom_comparison.set(Some(comparison)),
                Err(error) => comparison_error.set(Some(error.to_string())),
            }
            is_comparing.set(false);
        });
    };

    rsx! {
        div { class: "recipe-detail container",
            Card {
                PageHeader {
                    title: "{editor.title}",
                    action: Some(rsx! {
                        Link {
                            to: crate::Route::RecipeDetail { id: id.clone() },
                            class: "btn btn-secondary touch-target",
                            "Back to current recipe"
                        }
                    }),
                }

                div {
                    class: "neumo-inset",
                    padding: "var(--space-md)",
                    margin_bottom: "var(--space-md)",
                    strong { "Historical version v{detail.version_number}" }
                    p { "Published {detail.created_at}" }
                    p { "This is a read-only snapshot. The current recipe has not been changed." }
                    if let Some(notes) = &detail.notes {
                        p { "{notes}" }
                    }
                }

                if !editor.tags.is_empty() {
                    div { class: "recipe-detail__tags",
                        for tag in &editor.tags {
                            span { class: "recipe-detail__tag", "{tag}" }
                        }
                    }
                }

                div { class: "recipe-detail__meta-row",
                    if let Some(prep) = editor.prep_time_minutes {
                        span { class: "recipe-detail__meta-item", "⏱ Prep: {prep} min" }
                    }
                    if let Some(cook) = editor.cook_time_minutes {
                        span { class: "recipe-detail__meta-item", "🔥 Cook: {cook} min" }
                    }
                    if let Some(servings) = editor.servings {
                        span { class: "recipe-detail__meta-item", "🍽 Servings: {servings}" }
                    }
                    span { class: "recipe-detail__meta-item", "Visibility: {editor.visibility}" }
                }

                if let Some(description) = &editor.description {
                    if !description.is_empty() {
                        p { class: "recipe-detail__description", "{description}" }
                    }
                }
            }

            {render_image_slider(editor.images.clone())}

            if !editor.equipment.is_empty() {
                Card { {render_equipment(&editor.equipment)} }
            }

            if !editor.ingredients.is_empty() {
                Card {
                    div { class: "recipe-detail__ingredients-section",
                        h2 { class: "recipe-detail__section-title", "Ingredients" }
                        ul { class: "recipe-detail__list",
                            for ingredient in &editor.ingredients {
                                li { class: "recipe-detail__list-item",
                                    if !ingredient.amount.is_empty() && !ingredient.unit.is_empty() {
                                        "- {ingredient.amount} {ingredient.unit} {ingredient.name}"
                                    } else if !ingredient.amount.is_empty() {
                                        "- {ingredient.amount} {ingredient.name}"
                                    } else {
                                        "- {ingredient.name}"
                                    }
                                }
                            }
                        }
                    }
                    {render_recipe_scaler(
                        &editor.ingredients,
                        editor.prep_time_minutes,
                        editor.cook_time_minutes,
                        editor.servings,
                    )}
                }
            }

            if !editor.instructions.is_empty() {
                Card {
                    div { class: "recipe-detail__steps-section",
                        h2 { class: "recipe-detail__section-title", "Steps" }
                        ol { class: "recipe-detail__steps-list",
                            for (index, step) in editor.instructions.iter().enumerate() {
                                StepNode { step: step.clone(), path: vec![index], level: 0 }
                            }
                        }
                    }
                }
            }

            if let Some(commentary) = &editor.commentary {
                if !commentary.is_empty() {
                    Card { p { class: "recipe-detail__commentary", "{commentary}" } }
                }
            }

            if detail.comparisons_available {
                Card {
                    h2 { class: "recipe-detail__section-title", "Version comparisons" }
                    div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-md)",
                    if detail.version_number > 1 {
                        VersionComparisonPanel {
                            title: format!("Changes from v{}", detail.version_number - 1),
                            description: format!(
                                "Published changes from v{} to v{}.",
                                detail.version_number - 1,
                                detail.version_number,
                            ),
                            changes: detail.changes_from_previous.clone(),
                        }
                    }
                    VersionComparisonPanel {
                        title: "Compared with current version".to_string(),
                        description: format!(
                            "Historical v{} as Before and current v{} as After.",
                            detail.version_number,
                            detail.current_version_number,
                        ),
                        changes: detail.changes_to_current.clone(),
                    }
                    VersionComparisonPanel {
                        title: "Compared with original version".to_string(),
                        description: format!(
                            "Original v1 as Before and historical v{} as After.",
                            detail.version_number,
                        ),
                        changes: detail.changes_from_original.clone(),
                    }

                    div {
                        class: "neumo-inset",
                        padding: "var(--space-md)",
                        h3 { margin_bottom: "var(--space-xs)", "Compare any two versions" }
                        p {
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-md)",
                            "Choose the Before and After snapshots, then generate their field-by-field diff."
                        }
                        div {
                            display: "grid",
                            grid_template_columns: "repeat(auto-fit, minmax(180px, 1fr))",
                            gap: "var(--space-md)",
                            align_items: "end",
                            label {
                                "Before version"
                                select {
                                    class: "neumo-inset input",
                                    disabled: is_comparing(),
                                    onchange: move |event| {
                                        if let Ok(version) = event.value().parse::<i32>() {
                                            comparison_from.set(version);
                                            custom_comparison.set(None);
                                        }
                                    },
                                    for version in &versions {
                                        option {
                                            value: "{version.version_number}",
                                            selected: comparison_from() == version.version_number,
                                            "v{version.version_number} · {version.title}"
                                        }
                                    }
                                }
                            }
                            label {
                                "After version"
                                select {
                                    class: "neumo-inset input",
                                    disabled: is_comparing(),
                                    onchange: move |event| {
                                        if let Ok(version) = event.value().parse::<i32>() {
                                            comparison_to.set(version);
                                            custom_comparison.set(None);
                                        }
                                    },
                                    for version in &versions {
                                        option {
                                            value: "{version.version_number}",
                                            selected: comparison_to() == version.version_number,
                                            "v{version.version_number} · {version.title}"
                                        }
                                    }
                                }
                            }
                            Button {
                                variant: ButtonVariant::Primary,
                                disabled: versions.is_empty() || is_comparing(),
                                onclick: compare_selected_versions,
                                if is_comparing() { "Comparing..." } else { "Generate comparison" }
                            }
                        }
                        if let Some(error) = comparison_error() {
                            p { color: "var(--error)", margin_top: "var(--space-md)", "{error}" }
                        }
                    }

                    if let Some(comparison) = custom_comparison() {
                        VersionComparisonPanel {
                            title: format!(
                                "Custom comparison: v{} → v{}",
                                comparison.from_version_number,
                                comparison.to_version_number,
                            ),
                            description: format!(
                                "v{} as Before and v{} as After.",
                                comparison.from_version_number,
                                comparison.to_version_number,
                            ),
                            changes: comparison.changes,
                        }
                    }
                    }
                }
            } else {
                Card {
                    h2 { class: "recipe-detail__section-title", "Version comparisons" }
                    p { "Version comparisons are available to the recipe owner." }
                }
            }
        }
    }
}
