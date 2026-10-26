//! Independent grocery lists: compact creation and an optional recipe inspector.
use crate::api::grocery_list::*;
use crate::auth::account_binding::{
    account_completion_matches, use_form_account, ACCOUNT_CHANGED_FEEDBACK,
    ACCOUNT_COMPLETION_CHANGED_FEEDBACK,
};
use crate::auth::context::AuthContext;
use crate::components::base::{LoadingSpinner, SharedModal};
use crate::components::AuthRequired;
use crate::grocery::*;
use crate::Route;
use dioxus::prelude::*;
use uuid::Uuid;

fn feedback(error: &ServerFnError) -> String {
    crate::auth::account_binding::account_error_feedback(error)
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string())
}

#[component]
pub fn GroceryIcon(#[props(default="bag".into())] kind: String) -> Element {
    let path = match kind.as_str() {
        "book" => "M4 3h14a2 2 0 0 1 2 2v16H6a2 2 0 0 1-2-2V3ZM4 17h16M8 3v14",
        "pen" => "m15 4 5 5M4 20l5-1L21 7l-5-5L4 14v6Z",
        "plus" => "M12 5v14M5 12h14",
        "info" => "M12 11v6M12 7h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
        "check" => "m5 12 4 4 10-10",
        _ => "M5 7h14l2 14H3L5 7ZM8 8V6a4 4 0 0 1 8 0v2",
    };
    rsx! {
        svg {
            class: "grocery-icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.7",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path {
                d: path,
            }
        }
    }
}

#[component]
pub fn GroceryLists() -> Element {
    rsx! {
        GroceryWorkspace {

        }
    }
}
#[component]
pub fn GroceryListDetail(id: String) -> Element {
    rsx! {
        GroceryWorkspace {
            key: "{id}",
            id: Some(id),
        }
    }
}

#[component]
fn GroceryWorkspace(#[props(default)] id: Option<String>) -> Element {
    let form_account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let navigator = use_navigator();
    let parsed = id.as_deref().and_then(|v| Uuid::parse_str(v).ok());
    let invalid = id.is_some() && parsed.is_none();
    let mut refresh = use_signal(|| 0_u64);
    let summaries = use_resource(move || {
        let _ = refresh();
        let account = form_account();
        async move { list_grocery_lists(account).await }
    });
    let mut details = use_resource(use_reactive((&id,), move |(requested,)| {
        let _ = refresh();
        let account = form_account();
        let requested = requested.as_deref().and_then(|v| Uuid::parse_str(v).ok());
        async move {
            match requested {
                Some(id) => get_grocery_list(account, id).await.map(Some),
                None => Ok(None),
            }
        }
    }));
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut message = use_signal(String::new);
    let mut filter = use_signal(String::new);
    let mut hide_checked = use_signal(|| false);
    let mut inspector = use_signal(|| None::<Uuid>);
    let mut edit_item = use_signal(|| None::<GroceryItem>);
    let mut edit_fields = use_signal(ItemFields::default);
    let mut rename_open = use_signal(|| false);
    let mut rename_value = use_signal(String::new);
    let mut delete_open = use_signal(|| false);
    let mut add_fields = use_signal(ItemFields::default);
    let mut pending_add = use_signal(|| None::<(Uuid, ItemFields)>);
    let mut optimistic_checked = use_signal(|| None::<(Uuid, bool, i64)>);
    let mut route_id = use_signal(|| parsed);
    use_effect(use_reactive((&id,), move |(requested,)| {
        route_id.set(requested.as_deref().and_then(|v| Uuid::parse_str(v).ok()));
        inspector.set(None);
        filter.set(String::new());
        hide_checked.set(false);
        edit_item.set(None);
        rename_open.set(false);
        delete_open.set(false);
        pending_add.set(None);
        add_fields.set(ItemFields::default());
        optimistic_checked.set(None);
        error.set(None);
        message.set(String::new());
        busy.set(false);
    }));
    use_effect(move || {
        if let Some((item_id, desired, expected)) = optimistic_checked() {
            if let Some(Ok(Some(list))) = details.read().as_ref() {
                if list
                    .items
                    .iter()
                    .find(|i| i.id == item_id)
                    .is_none_or(|i| i.revision > expected || i.checked == desired)
                {
                    optimistic_checked.set(None);
                }
            }
        }
    });
    use_effect(use_reactive((&id,), move |(requested,)| {
        if *summaries.state().read() != UseResourceState::Pending && requested.is_none() {
            if let Some(Ok(lists)) = summaries.read().as_ref() {
                if let Some(first) = lists.first() {
                    navigator.replace(Route::GroceryListDetail {
                        id: first.id.to_string(),
                    });
                }
            }
        }
    }));
    let list = details
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(Clone::clone)
        .filter(|list| Some(list.id) == parsed);
    let sidebar = summaries
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned();
    let loading = *details.state().read() == UseResourceState::Pending;
    let disabled = busy() || loading;
    let account_changed = auth.read().current_user_id != form_account();
    let mut mutation_done = move |result: Result<(), ServerFnError>| {
        if *route_id.peek() != parsed {
            return;
        }
        busy.set(false);
        if auth.peek().current_user_id != form_account() {
            error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
            return;
        }
        match result {
            Ok(()) => {
                error.set(None);
                message.set("Changes saved.".into());
                refresh += 1;
            }
            Err(problem) => {
                optimistic_checked.set(None);
                error.set(Some(feedback(&problem)));
                message.set(String::new());
            }
        }
    };
    rsx! {
        AuthRequired {

            if account_changed {
                div {
                    class: "container grocery-page",
                    p {
                        class: "grocery-error",
                        role: "alert",
                        "{ACCOUNT_CHANGED_FEEDBACK}"
                    }
                }
            } else {
                div {
                    class: "container grocery-page",
                    nav {
                        class: "grocery-breadcrumb",
                        aria_label: "Breadcrumb",
                        Link {
                            to: Route::Dashboard {},
                            "Your library"
                        }
                        span {

                            " / "
                        }
                        span {

                            "Grocery lists"
                        }
                    }
                    if let Some(problem) = error() {
                        div {
                            class: "grocery-error",
                            role: "alert",
                            p {

                                "{problem}"
                            }
                            button {
                                class: "btn btn-secondary",
                                disabled: busy(),
                                onclick: move |_| {
                                    details.restart();
                                    refresh += 1;
                                },
                                "Refresh saved data"
                            }
                        }
                    }
                    p {
                        class: "sr-only",
                        role: "status",
                        "{message}"
                    }
                    div {
                        class: "grocery-workspace",
                        aside {
                            class: "grocery-rail neumo-raised",
                            aria_label: "Your grocery lists",
                            div {
                                class: "grocery-eyebrow",
                                "Your lists"
                            }
                            if let Some(lists) = sidebar {
                                div {
                                    class: "grocery-list-links",
                                    for summary in lists {
                                        Link {
                                            key: "{summary.id}",
                                            to: Route::GroceryListDetail {
                                                id: summary.id.to_string(),
                                            },
                                            class: if parsed == Some(summary.id) { "grocery-list-link is-current" } else { "grocery-list-link" },
                                            aria_current: if parsed == Some(summary.id) { "page" } else { "false" },
                                            div {

                                                GroceryIcon {
                                                    kind: (if summary.recipe_count > 0 { "book" } else { "pen" }).to_string(),
                                                }
                                                strong {

                                                    "{summary.name}"
                                                }
                                            }
                                            small {

                                                "{summary.item_count-summary.checked_count} to get · "
                                                if summary.recipe_count > 0 {
                                                    "{summary.recipe_count} recipes"
                                                } else {
                                                    "Manual list"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if summaries.read().as_ref().is_some_and(Result::is_err) {
                                p {
                                    class: "grocery-muted",
                                    "Your lists couldn’t be loaded."
                                }
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| {
                                        refresh += 1;
                                    },
                                    "Try again"
                                }
                            } else {
                                LoadingSpinner {

                                }
                            }
                            Link {
                                to: Route::GroceryListNew {},
                                class: "btn btn-primary grocery-new-link",
                                GroceryIcon {
                                    kind: "plus",
                                }
                                "Create a list"
                            }
                        }
                        div {
                            class: "grocery-workspace-main",
                            if invalid {
                                h1 {

                                    "List not found"
                                }
                                p {

                                    "Check the list address or choose a saved list."
                                }
                            } else if let Some(list) = list.clone() {
                                header {
                                    class: "grocery-list-heading",
                                    div {

                                        div {
                                            class: "grocery-eyebrow",
                                            "Your grocery workspace"
                                        }
                                        h1 {

                                            "{list.name}"
                                        }
                                        if !list.note.is_empty() {
                                            p {

                                                "{list.note}"
                                            }
                                        }
                                    }
                                    div {
                                        class: "grocery-heading-actions",
                                        button {
                                            class: "btn btn-secondary",
                                            disabled,
                                            onclick: {
                                                let name = list.name.clone();
                                                move |_| {
                                                    rename_value.set(name.clone());
                                                    rename_open.set(true);
                                                }
                                            },
                                            "Rename"
                                        }
                                        button {
                                            class: "btn btn-ghost",
                                            disabled,
                                            onclick: move |_| delete_open.set(true),
                                            "Delete list"
                                        }
                                    }
                                }
                                if list.sources.is_empty() {
                                    div {
                                        class: "grocery-source-banner",
                                        GroceryIcon {
                                            kind: "pen",
                                        }
                                        div {

                                            strong {

                                                "A list of your own."
                                            }
                                            p {

                                                "Add anything you need. No recipes required."
                                            }
                                        }
                                    }
                                } else {
                                    div {
                                        class: "grocery-source-banner",
                                        GroceryIcon {
                                            kind: "book",
                                        }
                                        strong {

                                            "From your recipe book and beyond"
                                        }
                                        span {

                                            "{list.sources.len()} recipes · Independent list"
                                        }
                                    }
                                }
                                div {
                                    class: if list.items.iter().any(|item|Some(item.id)==inspector()) { "grocery-detail-layout has-inspector" } else { "grocery-detail-layout" },
                                    section {
                                        class: "grocery-shopping neumo-raised",
                                        aria_label: "Shopping checklist",
                                        div {
                                            class: "grocery-section-heading",
                                            h2 {

                                                "Shopping checklist"
                                            }
                                            span {
                                                class: "grocery-pill",
                                                "{list.items.iter().filter(|i|i.checked).count()} of {list.items.len()} checked"
                                            }
                                        }
                                        div {
                                            class: "grocery-progress",
                                            "aria-hidden": "true",
                                            div {
                                                style: format!(
                                                    "width: {}%",
                                                    if list.items.is_empty() {
                                                        0
                                                    } else {
                                                        list.items.iter().filter(|i| i.checked).count() * 100 / list.items.len()
                                                    },
                                                ),
                                            }
                                        }
                                        div {
                                            class: "grocery-tools",
                                            label {

                                                span {
                                                    class: "sr-only",
                                                    "Find an item"
                                                }
                                                input {
                                                    class: "neumo-inset input input--shared",
                                                    r#type: "search",
                                                    placeholder: "Find an item…",
                                                    value: filter(),
                                                    oninput: move |e| filter.set(e.value()),
                                                }
                                            }
                                            button {
                                                class: "btn btn-ghost",
                                                onclick: move |_| hide_checked.set(!hide_checked()),
                                                if hide_checked() {
                                                    "Show checked"
                                                } else {
                                                    "Hide checked"
                                                }
                                            }
                                        }
                                        if loading {
                                            p {
                                                class: "grocery-muted",
                                                role: "status",
                                                "Refreshing saved list…"
                                            }
                                        }
                                        div {
                                            class: "grocery-rows",
                                            for item in list.items
                                                .iter()
                                                .filter(|item| {
                                                    (!hide_checked() || !item.checked)
                                                        && item.fields.name.to_lowercase().contains(&filter().to_lowercase())
                                                })
                                            {
                                                div {
                                                    key: "{item.id}",
                                                    class: if item.checked { "grocery-row is-checked" } else { "grocery-row" },
                                                    input {
                                                        id: "grocery-check-{item.id}",
                                                        r#type: "checkbox",
                                                        checked: optimistic_checked().filter(|(id,_,_)|*id==item.id).map(|(_,value,_)|value).unwrap_or(item.checked),
                                                        disabled,
                                                        aria_label: "Mark {item.fields.name} as purchased",
                                                        onchange: {
                                                            let item = item.clone();
                                                            let list_id = list.id;
                                                            move |e: FormEvent| {
                                                                if disabled {
                                                                    return;
                                                                }
                                                                busy.set(true);
                                                                error.set(None);
                                                                message.set("Saving shopping progress…".into());
                                                                let checked = e.checked();
                                                                optimistic_checked.set(Some((item.id,checked,item.revision)));
                                                                spawn(async move {
                                                                    mutation_done(
                                                                        update_grocery_item(
                                                                                form_account(),
                                                                                list_id,
                                                                                item.id,
                                                                                item.revision,
                                                                                GroceryItemChange::Checked(checked),
                                                                            )
                                                                            .await,
                                                                    );
                                                                });
                                                            }
                                                        },
                                                    }
                                                    label {
                                                        r#for: "grocery-check-{item.id}",
                                                        strong {

                                                            "{item.fields.name}"
                                                        }
                                                        small {

                                                            if item.contributions.is_empty() {
                                                                "Added by you"
                                                            } else if item.edited() {
                                                                "Edited on this list"
                                                            } else {
                                                                if item.contributions.len()==1 { "From 1 recipe contribution" } else { "From {item.contributions.len()} recipe contributions" }
                                                            if item.contributions.iter().any(|c|c.quantity_uncertain) { " · Check quantity" }
                                                            }
                                                        }
                                                    }
                                                    span {
                                                        class: "grocery-amount",
                                                        "{item.fields.amount} {item.fields.unit}"
                                                    }
                                                    div {
                                                        class: "grocery-row-actions",
                                                        if !item.contributions.is_empty() {
                                                            button {
                                                                id: "grocery-inspect-{item.id}",
                                                                class: "btn btn-ghost grocery-icon-button",
                                                                aria_label: "Recipe details for {item.fields.name}",
                                                                onclick: {
                                                                    let item_id = item.id;
                                                                    move |_| inspector.set(Some(item_id))
                                                                },
                                                                GroceryIcon {
                                                                    kind: "info",
                                                                }
                                                            }
                                                        }
                                                        button {
                                                            class: "btn btn-ghost grocery-icon-button",
                                                            disabled,
                                                            aria_label: "Edit {item.fields.name}",
                                                            onclick: {
                                                                let item = item.clone();
                                                                move |_| {
                                                                    edit_fields.set(item.fields.clone());
                                                                    edit_item.set(Some(item.clone()));
                                                                }
                                                            },
                                                            GroceryIcon {
                                                                kind: "pen",
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if list.items.is_empty() {
                                            p {
                                                class: "grocery-empty",
                                                "Your list is ready. Add your first item below."
                                            }
                                        }
                                        form {
                                            class: "grocery-add-row",
                                            onsubmit: {
                                                let list_id = list.id;
                                                move |e| {
                                                    e.prevent_default();
                                                    if disabled {
                                                        return;
                                                    }
                                                    let fields = add_fields();
                                                    if let Err(problem) = fields.validate() {
                                                        error.set(Some(problem));
                                                        return;
                                                    }
                                                    let (key, payload) = pending_add()
                                                        .unwrap_or_else(|| (Uuid::new_v4(), fields));
                                                    pending_add.set(Some((key, payload.clone())));
                                                    busy.set(true);
                                                    spawn(async move {
                                                        let result = add_grocery_item(form_account(), list_id, key, payload)
                                                            .await;
                                                        if result.is_ok() {
                                                            add_fields.set(ItemFields::default());
                                                            pending_add.set(None);
                                                        }
                                                        mutation_done(result);
                                                    });
                                                }
                                            },
                                            label {

                                                span {
                                                    class: "sr-only",
                                                    "Item name"
                                                }
                                                input {
                                                    class: "neumo-inset input input--shared",
                                                    placeholder: "Add an item…",
                                                    required: true,
                                                    maxlength: 500,
                                                    value: add_fields().name,
                                                    disabled: disabled || pending_add().is_some(),
                                                    oninput: move |e| add_fields.write().name = e.value(),
                                                }
                                            }
                                            label {

                                                span {
                                                    class: "sr-only",
                                                    "Amount"
                                                }
                                                input {
                                                    class: "neumo-inset input input--shared",
                                                    placeholder: "Amount",
                                                    maxlength: 64,
                                                    value: add_fields().amount,
                                                    disabled: disabled || pending_add().is_some(),
                                                    oninput: move |e| add_fields.write().amount = e.value(),
                                                }
                                            }
                                            label {

                                                span {
                                                    class: "sr-only",
                                                    "Unit"
                                                }
                                                input {
                                                    class: "neumo-inset input input--shared",
                                                    placeholder: "Unit",
                                                    maxlength: 64,
                                                    value: add_fields().unit,
                                                    disabled: disabled || pending_add().is_some(),
                                                    oninput: move |e| add_fields.write().unit = e.value(),
                                                }
                                            }
                                            button {
                                                class: "btn btn-secondary",
                                                r#type: "submit",
                                                disabled,
                                                if pending_add().is_some() {
                                                    "Retry add"
                                                } else {
                                                    "Add"
                                                }
                                            }
                                        }
                                        p {
                                            class: "grocery-save-status",
                                            role: "status",
                                            if busy() {
                                                "Saving…"
                                            } else if loading {
                                                "Loading saved data…"
                                            } else {
                                                "Showing saved shopping progress."
                                            }
                                        }
                                    }
                                    if let Some(item) = list.items.iter().find(|i| Some(i.id) == inspector()) {
                                        aside {
                                            id: "grocery-recipe-inspector",
                                            onmounted: move |_| {
                                                #[cfg(target_arch="wasm32")]
                                                spawn(async move { let _=document::eval("document.getElementById('grocery-recipe-inspector')?.focus()").await; });
                                            },
                                            class: "grocery-inspector",
                                            aria_label: "Recipe contribution details",
                                            tabindex: "-1",
                                            div {
                                                class: "grocery-section-heading",
                                                span {
                                                    class: "grocery-eyebrow",
                                                    "Ingredient details"
                                                }
                                                button {
                                                    class: "btn btn-ghost grocery-icon-button",
                                                    aria_label: "Close recipe details",
                                                    onclick: {
                                                        let item_id = item.id;
                                                        move |_| {
                                                            inspector.set(None);
                                                            #[cfg(target_arch = "wasm32")]
                                                            {
                                                                let id = format!("grocery-inspect-{item_id}");
                                                                spawn(async move {
                                                                    let _ = document::eval(
                                                                            &format!(
                                                                                "document.getElementById({})?.focus()",
                                                                                serde_json::to_string(&id).unwrap_or_default(),
                                                                            ),
                                                                        )
                                                                        .await;
                                                                });
                                                            }
                                                            #[cfg(not(target_arch = "wasm32"))]
                                                            let _ = item_id;
                                                        }
                                                    },
                                                    "✕"
                                                }
                                            }
                                            h2 {

                                                "{item.fields.name}"
                                            }
                                            p {
                                                class: "grocery-muted",
                                                "Current shopping entry"
                                            }
                                            p {
                                                class: "grocery-inspector-amount",
                                                "{item.fields.amount} {item.fields.unit}"
                                            }
                                            if item.edited() {
                                                p {
                                                    class: "grocery-edited-note",
                                                    "Edited on this list. The original recipe requirements below have not changed."
                                                }
                                            }
                                            h3 {

                                                "Recipe contributions"
                                            }
                                            for (index, source) in item.contributions.iter().enumerate() {
                                                div {
                                                    key: "{index}",
                                                    class: "grocery-contribution",
                                                    strong {

                                                        "{source.source.title}"
                                                    }
                                                    p {

                                                        "{source.original.name}: {source.scaled_amount} {source.original.unit}"
                                                    }
                                                    p {
                                                        class: "grocery-muted",
                                                        if let Some(base) = source.source.base_servings.filter(|s| *s > 0) {
                                                            "{base} → {source.source.target_servings.unwrap_or(base)} servings"
                                                        } else {
                                                            "Base servings unknown · Original amounts copied"
                                                        }
                                                    }
                                                    if source.quantity_uncertain {
                                                        p {
                                                            class: "grocery-uncertainty",
                                                            "Quantity not fully evaluated. Original: {source.original.amount} {source.original.unit}"
                                                        }
                                                    }
                                                    if source.approximate {
                                                        p {
                                                            class: "grocery-muted",
                                                            "Displayed amount is rounded."
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if loading {
                                div {
                                    class: "grocery-state",
                                    LoadingSpinner {

                                    }
                                    p {

                                        "Loading your grocery list…"
                                    }
                                }
                            } else if details.read().as_ref().is_some_and(Result::is_err) {
                                div {
                                    class: "grocery-state",
                                    h1 {

                                        "List unavailable"
                                    }
                                    p {

                                        "This list couldn’t be loaded. It may have been deleted, or your connection may have changed."
                                    }
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| details.restart(),
                                        "Try again"
                                    }
                                }
                            } else {
                                div {
                                    class: "grocery-state neumo-raised",
                                    GroceryIcon {

                                    }
                                    h1 {

                                        "A home for every grocery list."
                                    }
                                    p {

                                        "Start with your own essentials, add recipes if you like, and keep every shop in one place."
                                    }
                                    Link {
                                        to: Route::GroceryListNew {},
                                        class: "btn btn-primary",
                                        "Create your first list"
                                    }
                                }
                            }
                        }
                    }
                    if let (Some(item), Some(list)) = (edit_item(), list.clone()) {
                        SharedModal {
                            aria_labelledby: "grocery-edit-title",
                            focus_selector: "#grocery-edit-name",
                            pending: busy(),
                            on_dismiss: move |_| edit_item.set(None),
                            div {
                                class: "modal-card grocery-edit-dialog",
                                onclick: move |e| e.stop_propagation(),
                                h2 {
                                    id: "grocery-edit-title",
                                    "Edit shopping item"
                                }
                                if !item.contributions.is_empty() {
                                    p {
                                        class: "grocery-muted",
                                        "Recipe context stays as originally copied. These edits only change your list."
                                    }
                                }
                                form {
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        if busy() {
                                            return;
                                        }
                                        let fields = edit_fields();
                                        if let Err(problem) = fields.validate() {
                                            error.set(Some(problem));
                                            return;
                                        }
                                        busy.set(true);
                                        spawn(async move {
                                            let result = update_grocery_item(
                                                    form_account(),
                                                    list.id,
                                                    item.id,
                                                    item.revision,
                                                    GroceryItemChange::Fields(fields),
                                                )
                                                .await;
                                            if result.is_ok() {
                                                edit_item.set(None);
                                            }
                                            mutation_done(result);
                                        });
                                    },
                                    label {
                                        r#for: "grocery-edit-name",
                                        "Ingredient / item name"
                                    }
                                    input {
                                        id: "grocery-edit-name",
                                        class: "neumo-inset input input--shared",
                                        required: true,
                                        maxlength: 500,
                                        disabled: busy(),
                                        value: edit_fields().name,
                                        oninput: move |e| edit_fields.write().name = e.value(),
                                    }
                                    div {
                                        class: "grocery-edit-amounts",
                                        label {

                                            "Amount"
                                            input {
                                                class: "neumo-inset input input--shared",
                                                maxlength: 64,
                                                disabled: busy(),
                                                value: edit_fields().amount,
                                                oninput: move |e| edit_fields.write().amount = e.value(),
                                            }
                                        }
                                        label {

                                            "Unit"
                                            input {
                                                class: "neumo-inset input input--shared",
                                                maxlength: 64,
                                                disabled: busy(),
                                                value: edit_fields().unit,
                                                oninput: move |e| edit_fields.write().unit = e.value(),
                                            }
                                        }
                                    }
                                    if let Some(problem) = error() {
                                        p {
                                            class: "grocery-error",
                                            role: "alert",
                                            "{problem}"
                                        }
                                    }
                                    div {
                                        class: "grocery-dialog-actions",
                                        button {
                                            class: "btn btn-ghost",
                                            r#type: "button",
                                            disabled: busy(),
                                            onclick: move |_| edit_item.set(None),
                                            "Cancel"
                                        }
                                        button {
                                            class: "btn btn-primary",
                                            r#type: "submit",
                                            disabled: busy(),
                                            "Save item"
                                        }
                                    }
                                }
                                button {
                                    class: "btn btn-ghost grocery-remove",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        if busy() {
                                            return;
                                        }
                                        busy.set(true);
                                        spawn(async move {
                                            let result = remove_grocery_item(
                                                    form_account(),
                                                    list.id,
                                                    item.id,
                                                    item.revision,
                                                )
                                                .await;
                                            if result.is_ok() {
                                                edit_item.set(None);
                                                inspector.set(None);
                                            }
                                            mutation_done(result);
                                        });
                                    },
                                    "Remove this item"
                                }
                            }
                        }
                    }
                    if rename_open() {
                        if let Some(list) = list.clone() {
                            SharedModal {
                                aria_labelledby: "grocery-rename-title",
                                focus_selector: "#grocery-rename-name",
                                pending: busy(),
                                on_dismiss: move |_| rename_open.set(false),
                                div {
                                    class: "modal-card grocery-edit-dialog",
                                    onclick: move |e| e.stop_propagation(),
                                    h2 {
                                        id: "grocery-rename-title",
                                        "Rename your list"
                                    }
                                    form {
                                        onsubmit: move |e| {
                                            e.prevent_default();
                                            if busy() {
                                                return;
                                            }
                                            let name = rename_value();
                                            if let Err(problem) = validate_list_name(&name) {
                                                error.set(Some(problem));
                                                return;
                                            }
                                            busy.set(true);
                                            spawn(async move {
                                                let result = rename_grocery_list(
                                                        form_account(),
                                                        list.id,
                                                        list.revision,
                                                        name,
                                                    )
                                                    .await;
                                                if result.is_ok() {
                                                    rename_open.set(false);
                                                }
                                                mutation_done(result);
                                            });
                                        },
                                        label {
                                            r#for: "grocery-rename-name",
                                            "List name"
                                        }
                                        input {
                                            id: "grocery-rename-name",
                                            class: "neumo-inset input input--shared",
                                            value: rename_value(),
                                            required: true,
                                            maxlength: 100,
                                            disabled: busy(),
                                            oninput: move |e| rename_value.set(e.value()),
                                        }
                                        if let Some(problem) = error() {
                                            p {
                                                class: "grocery-error",
                                                role: "alert",
                                                "{problem}"
                                            }
                                        }
                                        div {
                                            class: "grocery-dialog-actions",
                                            button {
                                                class: "btn btn-ghost",
                                                r#type: "button",
                                                disabled: busy(),
                                                onclick: move |_| rename_open.set(false),
                                                "Cancel"
                                            }
                                            button {
                                                class: "btn btn-primary",
                                                disabled: busy(),
                                                "Save name"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if delete_open() {
                        if let Some(list) = list {
                            SharedModal {
                                aria_labelledby: "grocery-delete-title",
                                focus_selector: "#grocery-delete-cancel",
                                pending: busy(),
                                on_dismiss: move |_| delete_open.set(false),
                                div {
                                    class: "modal-card grocery-edit-dialog",
                                    onclick: move |e| e.stop_propagation(),
                                    h2 {
                                        id: "grocery-delete-title",
                                        "Delete this grocery list?"
                                    }
                                    p {

                                        "“{list.name}” and its {list.items.len()} items will be removed. Your recipes will stay unchanged."
                                    }
                                    if let Some(problem) = error() {
                                        p {
                                            class: "grocery-error",
                                            role: "alert",
                                            "{problem}"
                                        }
                                    }
                                    div {
                                        class: "grocery-dialog-actions",
                                        button {
                                            id: "grocery-delete-cancel",
                                            class: "btn btn-secondary",
                                            disabled: busy(),
                                            onclick: move |_| delete_open.set(false),
                                            "Keep list"
                                        }
                                        button {
                                            class: "btn btn-danger",
                                            disabled: busy(),
                                            onclick: move |_| {
                                                if busy() {
                                                    return;
                                                }
                                                busy.set(true);
                                                spawn(async move {
                                                    let result = delete_grocery_list(form_account(), list.id, list.revision)
                                                        .await;
                                                    if result.is_ok() {
                                                        delete_open.set(false);
                                                        navigator.push(Route::GroceryLists {});
                                                    }
                                                    mutation_done(result);
                                                });
                                            },
                                            "Delete list"
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

/// Accept recipe IDs or existing relative/absolute recipe links, without fetching
/// arbitrary external URLs or enumerating unlisted recipes.
fn recipe_reference(text: &str) -> Option<Uuid> {
    if let Ok(id) = Uuid::parse_str(text.trim()) {
        return Some(id);
    }
    let url = url::Url::parse(text.trim())
        .or_else(|_| {
            url::Url::parse("https://noms.invalid").and_then(|base| base.join(text.trim()))
        })
        .ok()?;
    let mut parts = url.path_segments()?;
    if parts.next()? != "recipes" {
        return None;
    }
    let id = Uuid::parse_str(parts.next()?).ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(id)
}

#[component]
pub fn GroceryListNew() -> Element {
    let form_account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let navigator = use_navigator();
    let mut name = use_signal(String::new);
    let mut note = use_signal(String::new);
    let mut recipe_mode = use_signal(|| false);
    let mut manual = use_signal(Vec::<ItemFields>::new);
    let mut entry = use_signal(ItemFields::default);
    let mut selected = use_signal(Vec::<GroceryRecipePreview>::new);
    let mut serving_inputs = use_signal(std::collections::HashMap::<Uuid, String>::new);
    let mut query = use_signal(String::new);
    let mut search_query = use_signal(String::new);
    let mut offset = use_signal(|| 0_i64);
    let options = use_resource(move || {
        let account = form_account();
        let q = search_query();
        let page = offset();
        async move { find_grocery_recipes(account, q, page).await }
    });
    let mut reference = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut selecting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut pending = use_signal(|| None::<GroceryCreate>);
    let mut created = use_signal(|| None::<Uuid>);
    let mut add_recipe = move |id: Uuid| {
        if selecting() || busy() || selected().iter().any(|p| p.source.recipe_id == id) {
            return;
        }
        if selected().len() >= MAX_RECIPES {
            error.set(Some("Select up to 20 recipes.".into()));
            return;
        }
        selecting.set(true);
        error.set(None);
        spawn(async move {
            match preview_grocery_recipe(form_account(), id).await {
                Ok(preview) => {
                    if auth.peek().current_user_id == form_account() {
                        serving_inputs.write().remove(&id);
                        selected.write().push(preview);
                        reference.set(String::new());
                    } else {
                        error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                    }
                }
                Err(problem) => error.set(Some(feedback(&problem))),
            }
            selecting.set(false);
        });
    };
    let frozen = busy() || pending().is_some() || created().is_some();
    let account_changed = auth.read().current_user_id != form_account();
    let preview = build_requirements(&selected());
    rsx! {
        AuthRequired {

            if account_changed {
                div {
                    class: "container grocery-page",
                    p {
                        class: "grocery-error",
                        role: "alert",
                        "{ACCOUNT_CHANGED_FEEDBACK}"
                    }
                }
            } else {
                div {
                    class: "container grocery-create-page",
                    nav {
                        class: "grocery-breadcrumb",
                        aria_label: "Breadcrumb",
                        Link {
                            to: Route::GroceryLists {},
                            "Grocery lists"
                        }
                        span {

                            " / "
                        }
                        span {

                            "New list"
                        }
                    }
                    header {
                        class: "grocery-create-heading",
                        div {
                            class: "grocery-eyebrow",
                            "Your grocery workspace"
                        }
                        h1 {

                            "Create a grocery list."
                        }
                        p {

                            "Just a name and whatever you need. Recipes are optional."
                        }
                    }
                    section {
                        class: "grocery-compact-form neumo-raised",
                        if let Some(problem) = error() {
                            p {
                                class: "grocery-error",
                                role: "alert",
                                "{problem}"
                            }
                        }
                        label {
                            r#for: "grocery-list-name",
                            class: "grocery-field-label",
                            "List name"
                        }
                        input {
                            id: "grocery-list-name",
                            class: "neumo-inset input input--shared",
                            placeholder: "e.g. This week’s groceries",
                            maxlength: 100,
                            required: true,
                            disabled: frozen,
                            value: name(),
                            oninput: move |e| name.set(e.value()),
                        }
                        p {
                            class: "grocery-field-label grocery-start-label",
                            "How would you like to start?"
                        }
                        p {
                            class: "grocery-muted",
                            "You can save a list without choosing any recipes."
                        }
                        div {
                            class: "grocery-mode-choice",
                            button {
                                class: if !recipe_mode() { "grocery-mode is-selected" } else { "grocery-mode" },
                                aria_pressed: !recipe_mode(),
                                disabled: frozen || selecting(),
                                onclick: move |_| {
                                    recipe_mode.set(false);
                                    selected.set(Vec::new());
                                    serving_inputs.write().clear();
                                },
                                GroceryIcon {
                                    kind: "pen",
                                }
                                span {

                                    strong {

                                        "Your own items"
                                    }
                                    small {

                                        "No recipes needed"
                                    }
                                }
                            }
                            button {
                                class: if recipe_mode() { "grocery-mode is-selected" } else { "grocery-mode" },
                                aria_pressed: recipe_mode(),
                                disabled: frozen || selecting(),
                                onclick: move |_| recipe_mode.set(true),
                                GroceryIcon {
                                    kind: "book",
                                }
                                span {

                                    strong {

                                        "From recipes"
                                    }
                                    small {

                                        "Recipes + your extras"
                                    }
                                }
                            }
                        }
                        if recipe_mode() {
                            section {
                                class: "grocery-recipe-picker",
                                aria_label: "Select recipes",
                                h2 {

                                    "Choose your recipes"
                                }
                                p {
                                    class: "grocery-muted",
                                    "Your recipes and public recipes. Have an unlisted link? Add it below."
                                }
                                form {
                                    class: "grocery-recipe-search",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        search_query.set(query());
                                        offset.set(0);
                                    },
                                    label {

                                        span {
                                            class: "sr-only",
                                            "Search accessible recipes"
                                        }
                                        input {
                                            class: "neumo-inset input input--shared",
                                            r#type: "search",
                                            placeholder: "Search recipes…",
                                            maxlength: 256,
                                            value: query(),
                                            disabled: frozen,
                                            oninput: move |e| query.set(e.value()),
                                        }
                                    }
                                    button {
                                        class: "btn btn-secondary",
                                        disabled: frozen,
                                        "Search"
                                    }
                                }
                                match options.read().as_ref() {
                                    Some(Ok(recipes)) => rsx! {
                                        div {
                                            class: "grocery-recipe-options",
                                            for recipe in recipes {
                                                div {
                                                    key: "{recipe.id}",
                                                    class: "grocery-recipe-option",
                                                    div {

                                                        strong {

                                                            "{recipe.title}"
                                                        }
                                                        small {

                                                            if recipe.owned {
                                                                "Your recipe"
                                                            } else {
                                                                "Public recipe"
                                                            }
                                                            if let Some(servings) = recipe.servings.filter(|s| *s > 0) {
                                                                " · {servings} servings"
                                                            }
                                                        }
                                                    }
                                                    button {
                                                        class: "btn btn-secondary",
                                                        disabled: frozen || selecting() || selected().iter().any(|s| s.source.recipe_id == recipe.id),
                                                        onclick: {
                                                            let id = recipe.id;
                                                            move |_| add_recipe(id)
                                                        },
                                                        if selected().iter().any(|s| s.source.recipe_id == recipe.id) {
                                                            "Selected"
                                                        } else {
                                                            "Add"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if recipes.is_empty() {
                                            p {
                                                class: "grocery-muted",
                                                "No matching recipes. Try another title or an accessible link."
                                            }
                                        }
                                        div {
                                            class: "grocery-recipe-pagination",
                                            button {
                                                class: "btn btn-ghost",
                                                disabled: frozen || offset() == 0,
                                                onclick: move |_| offset.set((offset() - 25).max(0)),
                                                "Previous"
                                            }
                                            span {

                                                "Page {offset()/25+1}"
                                            }
                                            button {
                                                class: "btn btn-ghost",
                                                disabled: frozen || recipes.len() < 25 || offset() >= 10_000,
                                                onclick: move |_| offset += 25,
                                                "Next"
                                            }
                                        }
                                    },
                                    Some(Err(_)) => rsx! {
                                        p {
                                            class: "grocery-error",
                                            "Recipes couldn’t be loaded. Search again or add an accessible recipe link."
                                        }
                                    },
                                    None => rsx! {
                                        p {
                                            role: "status",
                                            "Loading recipes…"
                                        }
                                    },
                                }
                                form {
                                    class: "grocery-recipe-search",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        if let Some(id) = recipe_reference(&reference()) {
                                            add_recipe(id)
                                        } else {
                                            error
                                                .set(
                                                    Some(
                                                        "Enter a recipe ID or a recipe link ending in /recipes/ID."
                                                            .into(),
                                                    ),
                                                );
                                        }
                                    },
                                    label {

                                        span {
                                            class: "sr-only",
                                            "Accessible recipe link or ID"
                                        }
                                        input {
                                            class: "neumo-inset input input--shared",
                                            placeholder: "Paste a recipe link or ID…",
                                            maxlength: 2048,
                                            value: reference(),
                                            disabled: frozen || selecting(),
                                            oninput: move |e| reference.set(e.value()),
                                        }
                                    }
                                    button {
                                        class: "btn btn-secondary",
                                        disabled: frozen || selecting() || reference().trim().is_empty(),
                                        "Add link"
                                    }
                                }
                                if selecting() {
                                    p {
                                        role: "status",
                                        class: "grocery-muted",
                                        "Checking recipe access…"
                                    }
                                }
                                for (index, recipe) in selected().iter().enumerate() {
                                    div {
                                        key: "{recipe.source.recipe_id}",
                                        class: "grocery-selected-recipe",
                                        div {

                                            strong {

                                                "{recipe.source.title}"
                                            }
                                            if recipe.source.base_servings.is_none_or(|s| s <= 0) {
                                                small {

                                                    "Base servings unknown. Original amounts will be copied."
                                                }
                                            } else {
                                                small {

                                                    "Base: {recipe.source.base_servings.unwrap_or(1)} servings"
                                                }
                                            }
                                        }
                                        label {

                                            "Servings"
                                            input {
                                                class: "neumo-inset input input--shared",
                                                r#type: "number",
                                                min: 1,
                                                max: MAX_SERVINGS,
                                                disabled: frozen || recipe.source.base_servings.is_none_or(|s| s <= 0),
                                                value: serving_inputs().get(&recipe.source.recipe_id).cloned().unwrap_or_else(||recipe.source.target_servings.or(recipe.source.base_servings.filter(|s|*s>0)).map(|s|s.to_string()).unwrap_or_default()),
                                                oninput: move |e| {
                                                    let text = e.value();
                                                    if let Some(recipe) = selected().get(index) {
                                                        serving_inputs.write().insert(recipe.source.recipe_id,text.clone());
                                                    }
                                                    let value = text
                                                        .parse::<i32>()
                                                        .ok()
                                                        .filter(|v| (1..=MAX_SERVINGS).contains(v));
                                                    if let Some(recipe) = selected.write().get_mut(index) {
                                                        recipe.source.target_servings = value;
                                                    }
                                                },
                                            }
                                        }
                                        button {
                                            class: "btn btn-ghost",
                                            aria_label: "Remove {recipe.source.title}",
                                            disabled: frozen,
                                            onclick: move |_| {
                                                let removed=selected.write().remove(index);
                                                serving_inputs.write().remove(&removed.source.recipe_id);
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }
                        }
                        section {
                            class: "grocery-manual-editor",
                            aria_label: "Your own items",
                            h2 {

                                if recipe_mode() {
                                    "Your extras"
                                } else {
                                    "Your items"
                                }
                            }
                            for (index, item) in manual().iter().enumerate() {
                                div {
                                    key: "manual-{index}",
                                    class: "grocery-draft-row",
                                    span {

                                        strong {

                                            "{item.name}"
                                        }
                                        small {

                                            "{item.amount} {item.unit}"
                                        }
                                    }
                                    button {
                                        class: "btn btn-ghost",
                                        aria_label: "Remove {item.name}",
                                        disabled: frozen,
                                        onclick: move |_| {
                                            manual.write().remove(index);
                                        },
                                        "✕"
                                    }
                                }
                            }
                            form {
                                class: "grocery-add-row",
                                onsubmit: move |e| {
                                    e.prevent_default();
                                    if frozen {
                                        return;
                                    }
                                    let fields = entry();
                                    if let Err(problem) = fields.validate() {
                                        error.set(Some(problem));
                                        return;
                                    }
                                    if manual().len() >= MAX_ITEMS {
                                        error.set(Some("A list supports up to 1,000 items.".into()));
                                        return;
                                    }
                                    manual.write().push(fields);
                                    entry.set(ItemFields::default());
                                },
                                label {

                                    span {
                                        class: "sr-only",
                                        "Item name"
                                    }
                                    input {
                                        class: "neumo-inset input input--shared",
                                        placeholder: "Add an item…",
                                        value: entry().name,
                                        maxlength: 500,
                                        disabled: frozen,
                                        oninput: move |e| entry.write().name = e.value(),
                                    }
                                }
                                label {

                                    span {
                                        class: "sr-only",
                                        "Amount"
                                    }
                                    input {
                                        class: "neumo-inset input input--shared",
                                        placeholder: "Amount",
                                        value: entry().amount,
                                        maxlength: 64,
                                        disabled: frozen,
                                        oninput: move |e| entry.write().amount = e.value(),
                                    }
                                }
                                label {

                                    span {
                                        class: "sr-only",
                                        "Unit"
                                    }
                                    input {
                                        class: "neumo-inset input input--shared",
                                        placeholder: "Unit",
                                        value: entry().unit,
                                        maxlength: 64,
                                        disabled: frozen,
                                        oninput: move |e| entry.write().unit = e.value(),
                                    }
                                }
                                button {
                                    class: "btn btn-secondary",
                                    disabled: frozen,
                                    "Add"
                                }
                            }
                            p {
                                class: "grocery-muted",
                                "Amounts and units are optional. Add food, household supplies, or anything else."
                            }
                        }
                        label {
                            class: "grocery-field-label",
                            r#for: "grocery-list-note",
                            "Note (optional)"
                        }
                        input {
                            id: "grocery-list-note",
                            class: "neumo-inset input input--shared",
                            maxlength: 500,
                            placeholder: "Anything to remember for this shop…",
                            disabled: frozen,
                            value: note(),
                            oninput: move |e| note.set(e.value()),
                        }
                        if !selected().is_empty() {
                            details {
                                class: "grocery-preview",
                                summary {

                                    "Review copied ingredients ({preview.as_ref().map(Vec::len).unwrap_or(0)})"
                                }
                                match preview {
                                    Ok(items) => rsx! {
                                        for (index, item) in items.iter().enumerate() {
                                            div {
                                                key: "{index}",
                                                class: "grocery-draft-row",
                                                strong {

                                                    "{item.fields.name}"
                                                }
                                                span {

                                                    "{item.fields.amount} {item.fields.unit}"
                                                }
                                            }
                                        }
                                        p {
                                            class: "grocery-muted",
                                            "Matching names and units are combined without conversion. Textual quantities keep their wording. Every ingredient is copied; edit or remove any item after saving."
                                        }
                                    },
                                    Err(problem) => rsx! {
                                        p {
                                            class: "grocery-error",
                                            "{problem}"
                                        }
                                    },
                                }
                            }
                        }
                        if pending().is_some() && !busy() {
                            p {
                                class: "grocery-retry-note",
                                "We couldn’t confirm the save. Retry the same request to avoid a duplicate list. To change the form, first check your saved lists in another tab."
                            }
                        }
                        if let Some(id) = created() {
                            Link {
                                to: Route::GroceryListDetail {
                                    id: id.to_string(),
                                },
                                class: "btn btn-secondary",
                                "Open created list"
                            }
                        }
                        div {
                            class: "grocery-create-actions",
                            Link {
                                to: Route::GroceryLists {},
                                class: "btn btn-ghost",
                                "Cancel"
                            }
                            button {
                                class: "btn btn-primary",
                                disabled: busy() || selecting() || created().is_some(),
                                onclick: move |_| {
                                    if busy() || selecting() || created().is_some() {
                                        return;
                                    }
                                    let request = if let Some(request) = pending() {
                                        request
                                    } else {
                                        let mut extras = manual();
                                        let unadded = entry();
                                        for recipe in selected().iter() {
                                            if let Some(raw)=serving_inputs().get(&recipe.source.recipe_id) {
                                                if raw.parse::<i32>().ok().is_none_or(|v|!(1..=MAX_SERVINGS).contains(&v)) {
                                                    error.set(Some("Target servings must be between 1 and 10,000.".into()));return;
                                                }
                                            }
                                        }
                                        if !unadded.name.is_empty() || !unadded.amount.is_empty() || !unadded.unit.is_empty() {
                                            if let Err(problem) = unadded.validate() {
                                                error.set(Some(problem));
                                                return;
                                            }
                                            extras.push(unadded);
                                        }
                                        let request = GroceryCreate {
                                            request_id: Uuid::new_v4(),
                                            name: name(),
                                            note: note(),
                                            recipes: selected()
                                                .iter()
                                                .map(|p| RecipeSelection {
                                                    recipe_id: p.source.recipe_id,
                                                    target_servings: p.source.target_servings,
                                                    expected_revision: p.source.revision.clone(),
                                                })
                                                .collect(),
                                            manual_items: extras,
                                        };
                                        if let Err(problem) = request.validate() {
                                            error.set(Some(problem));
                                            return;
                                        }
                                        if let Err(problem) = build_requirements(&selected()) {
                                            error.set(Some(problem));
                                            return;
                                        }
                                        request
                                    };
                                    pending.set(Some(request.clone()));
                                    busy.set(true);
                                    error.set(None);
                                    spawn(async move {
                                        match create_grocery_list(form_account(), request).await {
                                            Ok(list) => {
                                                created.set(Some(list.id));
                                                if account_completion_matches(
                                                    form_account(),
                                                    auth.peek().current_user_id,
                                                    list.owner_id,
                                                ) {
                                                    navigator
                                                        .push(Route::GroceryListDetail {
                                                            id: list.id.to_string(),
                                                        });
                                                } else {
                                                    error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                                                }
                                            }
                                            Err(problem) => {
                                                let text = feedback(&problem);
                                                if text.contains("selected recipe changed")
                                                    || text.contains("Recipe not accessible")
                                                {
                                                    pending.set(None);
                                                    let selected_before = selected();
                                                    let mut reviewed = Vec::new();
                                                    for prior in selected_before {
                                                        if let Ok(mut fresh) = preview_grocery_recipe(
                                                                form_account(),
                                                                prior.source.recipe_id,
                                                            )
                                                            .await
                                                        {
                                                            if fresh.source.base_servings.is_some_and(|s|s>0) {
                                                                fresh.source.target_servings = prior.source.target_servings;
                                                            } else {
                                                                serving_inputs.write().remove(&fresh.source.recipe_id);
                                                            }
                                                            reviewed.push(fresh);
                                                        }
                                                    }
                                                    selected.set(reviewed);
                                                }
                                                error.set(Some(text));
                                            }
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() {
                                    "Saving…"
                                } else if pending().is_some() {
                                    "Retry creation"
                                } else {
                                    "Create list"
                                }
                            }
                        }
                        p {
                            class: "grocery-create-footnote",
                            "Private · Saved independently · Edit anything after creation"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipe_links_resolve_only_recipe_paths() {
        let id = Uuid::new_v4();
        assert_eq!(recipe_reference(&id.to_string()), Some(id));
        assert_eq!(
            recipe_reference(&format!("https://example.com/recipes/{id}?from=share")),
            Some(id)
        );
        assert_eq!(recipe_reference(&format!("/recipes/{id}")), Some(id));
        assert_eq!(
            recipe_reference("https://example.com/collections/abc"),
            None
        );
        assert_eq!(recipe_reference(&format!("/recipes/{id}/edit")), None);
    }
}
