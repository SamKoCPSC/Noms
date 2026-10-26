//! Pantry W01 workspace and C01 compact creation, backed by authenticated functions.
use crate::api::pantry::*;
use crate::auth::account_binding::{
    account_completion_matches, use_form_account, ACCOUNT_CHANGED_FEEDBACK,
    ACCOUNT_COMPLETION_CHANGED_FEEDBACK,
};
use crate::auth::context::AuthContext;
use crate::components::base::{LoadingSpinner, SharedModal};
use crate::components::AuthRequired;
use crate::pantry::*;
use crate::Route;
use dioxus::prelude::*;
use uuid::Uuid;
fn feedback(error: &ServerFnError) -> String {
    crate::auth::account_binding::account_error_feedback(error)
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string())
}
#[cfg(target_arch = "wasm32")]
fn saved_selection(owner: Uuid) -> Option<Uuid> {
    let storage = web_sys::window()?.local_storage().ok()??;
    Uuid::parse_str(
        &storage
            .get_item(&format!("noms-pantry-selection-{owner}"))
            .ok()??,
    )
    .ok()
}
#[cfg(not(target_arch = "wasm32"))]
fn saved_selection(_owner: Uuid) -> Option<Uuid> {
    None
}
fn remember_selection(owner: Uuid, id: Option<Uuid>) {
    #[cfg(target_arch = "wasm32")]
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let key = format!("noms-pantry-selection-{owner}");
        if let Some(id) = id {
            let _ = storage.set_item(&key, &id.to_string());
        } else {
            let _ = storage.remove_item(&key);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (owner, id);
}
#[component]
fn PantryIcon(#[props(default="jar".into())] kind: String) -> Element {
    let path = match kind.as_str() {
        "home" => "M3 11 12 3l9 8M5 10v11h14V10M9 21v-7h6v7",
        "check" => "m5 12 4 4 10-10",
        "plus" => "M12 5v14M5 12h14",
        "pen" => "m15 4 5 5M4 20l5-1L21 7l-5-5L4 14v6Z",
        _ => "M8 3h8v3H8zM7 6h10l2 4v10a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V10l2-4ZM5 12h14",
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
            path { d: path }
        }
    }
}
#[component]
pub fn Pantry() -> Element {
    rsx! {
        PantryWorkspace {
        }
    }
}
#[component]
pub fn PantryDetail(id: String) -> Element {
    rsx! {
        PantryWorkspace { key: "{id}", id: Some(id) }
    }
}
#[component]
fn PantryFieldsInput(
    mut fields: Signal<PantryFields>,
    prefix: String,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        label { r#for: "{prefix}-name", "Ingredient name" }
        input {
            id: "{prefix}-name",
            class: "input input--shared neumo-inset",
            value: "{fields().name}",
            maxlength: 500,
            required: true,
            disabled,
            oninput: move |e| fields.write().name = e.value(),
        }

        div { class: "pantry-amount-fields",
            label { r#for: "{prefix}-amount",
                "Amount"
                span { class: "grocery-muted", "Optional" }
            }
            label { r#for: "{prefix}-unit",
                "Unit"
                span { class: "grocery-muted", "As written" }
            }
            input {
                id: "{prefix}-amount",
                class: "input input--shared neumo-inset",
                value: "{fields().amount}",
                maxlength: 64,
                placeholder: "Unknown",
                disabled,
                oninput: move |e| fields.write().amount = e.value(),
            }
            input {
                id: "{prefix}-unit",
                class: "input input--shared neumo-inset",
                value: "{fields().unit}",
                maxlength: 64,
                placeholder: "e.g. cup",
                disabled,
                oninput: move |e| fields.write().unit = e.value(),
            }
        }

    }
}
#[component]
fn PantryEditor(
    profile_id: Uuid,
    entry: PantryEntry,
    on_saved: EventHandler<()>,
    #[props(default)] disabled: bool,
) -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let initial = entry.stock.fields.clone();
    let fields = use_signal(move || initial);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut confirm_remove = use_signal(|| false);
    let kind = PantryKind::Ingredient;
    let expected = entry.revision;
    let item = entry.id;
    let unavailable = disabled || busy() || auth.read().current_user_id != account();
    rsx! {
        aside {
            class: "grocery-inspector pantry-editor",
            id: "pantry-editor",
            tabindex: "-1",
            aria_label: "Inventory details",
            div { class: "pantry-editor-head",
                span { class: "grocery-eyebrow", "Ingredient details" }
                PantryIcon { kind: "jar" }
            }
            h2 { "{entry.stock.fields.name}" }
            p { class: "grocery-muted", "{entry.status()}" }

            div { class: "grocery-inspector-amount", "{entry.amount_label()}" }

            if let Some(problem) = error() {
                p { class: "grocery-error", role: "alert", "{problem}" }
            }
            form {
                class: "pantry-editor-form",
                onsubmit: move |e| {
                    e.prevent_default();
                    if unavailable {
                        return;
                    }
                    let submitted = fields();
                    if let Err(problem) = submitted.validate(kind) {
                        error.set(Some(problem));
                        return;
                    }
                    busy.set(true);
                    error.set(None);
                    spawn(async move {
                        let result = update_pantry_entry(
                                account(),
                                profile_id,
                                kind,
                                item,
                                expected,
                                PantryChange::Fields(submitted),
                            )
                            .await;
                        busy.set(false);
                        if auth.peek().current_user_id != account() {
                            error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                            return;
                        }
                        match result {
                            Ok(()) => on_saved.call(()),
                            Err(e) => error.set(Some(feedback(&e))),
                        }
                    });
                },
                PantryFieldsInput {
                    fields,

                    prefix: "pantry-edit",
                    disabled: unavailable,
                }
                button {
                    class: "btn btn-primary",
                    r#type: "submit",
                    disabled: unavailable,
                    if busy() {
                        "Saving…"
                    } else {
                        "Save changes"
                    }
                }
            }
            button {
                class: "btn btn-ghost pantry-remove",
                disabled: unavailable,
                onclick: move |_| confirm_remove.set(true),

                "Remove ingredient"

            }
            if confirm_remove() {
                SharedModal {
                    aria_labelledby: "pantry-remove-title",
                    focus_selector: "#pantry-keep-item",
                    pending: busy(),
                    on_dismiss: move |_| confirm_remove.set(false),
                    div {
                        class: "modal-card pantry-modal",
                        onclick: move |e| e.stop_propagation(),
                        h2 { id: "pantry-remove-title", "Remove {entry.stock.fields.name}?" }
                        div { class: "grocery-dialog-actions",
                            button {
                                id: "pantry-keep-item",
                                class: "btn btn-secondary",
                                disabled: busy(),
                                onclick: move |_| confirm_remove.set(false),
                                "Keep item"
                            }
                            button {
                                class: "btn btn-danger",
                                disabled: unavailable,
                                onclick: move |_| {
                                    if unavailable {
                                        return;
                                    }
                                    busy.set(true);
                                    spawn(async move {
                                        let result = remove_pantry_entry(account(), profile_id, kind, item, expected)
                                            .await;
                                        busy.set(false);
                                        if auth.peek().current_user_id != account() {
                                            error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                                            return;
                                        }
                                        match result {
                                            Ok(()) => {
                                                confirm_remove.set(false);
                                                on_saved.call(())
                                            }
                                            Err(e) => error.set(Some(feedback(&e))),
                                        }
                                    });
                                },
                                "Remove item"
                            }
                        }

                    }
                }
            }
        }
    }
}
#[component]
fn PantryWorkspace(#[props(default)] id: Option<String>) -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let nav = use_navigator();
    let parsed = id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
    let invalid = id.is_some() && parsed.is_none();
    let mut refresh = use_signal(|| 0_u64);
    let summaries = use_resource(move || {
        let _ = refresh();
        let owner = account();
        async move { list_kitchens(owner).await }
    });
    let mut details = use_resource(use_reactive((&id,), move |(value,)| {
        let _ = refresh();
        let owner = account();
        async move {
            match value.as_deref().and_then(|s| Uuid::parse_str(s).ok()) {
                Some(id) => get_kitchen(owner, id).await.map(Some),
                None => Ok(None),
            }
        }
    }));
    let kind = PantryKind::Ingredient;
    let mut filter = use_signal(String::new);
    let mut availability = use_signal(|| "all".to_string());
    let mut selected = use_signal(|| None::<Uuid>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut message = use_signal(String::new);
    let mut addition = use_signal(PantryFields::default);
    let mut pending_add = use_signal(|| None::<(Uuid, PantryStock)>);
    let mut edit_open = use_signal(|| false);
    let mut edit_name = use_signal(String::new);
    let mut edit_note = use_signal(String::new);
    let mut delete_open = use_signal(|| false);
    let mut duplicate = use_signal(|| None::<PantryCreate>);
    let mut current_route = use_signal(|| parsed);
    use_effect(use_reactive((&id,), move |(value,)| {
        current_route.set(value.as_deref().and_then(|s| Uuid::parse_str(s).ok()));
        filter.set(String::new());
        selected.set(None);
        addition.set(PantryFields::default());
        pending_add.set(None);
        duplicate.set(None);
        edit_open.set(false);
        delete_open.set(false);
        busy.set(false);
        error.set(None);
    }));
    use_effect(use_reactive((&id,), move |(value,)| {
        if value.is_none() && *summaries.state().read() != UseResourceState::Pending {
            if let Some(Ok(rows)) = summaries.read().as_ref() {
                if let Some(owner) = account() {
                    if auth.peek().current_user_id != Some(owner) {
                        return;
                    }
                    let remembered = saved_selection(owner);
                    if let Some(chosen) = remembered
                        .and_then(|id| rows.iter().find(|p| p.id == id))
                        .or(rows.first())
                    {
                        nav.replace(Route::PantryDetail {
                            id: chosen.id.to_string(),
                        });
                    } else {
                        remember_selection(owner, None)
                    }
                }
            }
        }
    }));
    let kitchen = details
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(Clone::clone)
        .filter(|p| Some(p.id) == parsed);
    let sidebar = summaries
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned();
    let loading = *details.state().read() == UseResourceState::Pending;
    let disabled = busy() || loading;
    use_effect(move || {
        if let Some(Ok(Some(p))) = details.read().as_ref() {
            if auth.peek().current_user_id == account() {
                remember_selection(p.owner_id, Some(p.id));
            }
        }
    });
    let mut done = move |result: Result<(), ServerFnError>| {
        if *current_route.peek() != parsed {
            return;
        }
        busy.set(false);
        if auth.peek().current_user_id != account() {
            error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
            return;
        }
        match result {
            Ok(()) => {
                error.set(None);
                message.set("Changes saved.".into());
                refresh += 1
            }
            Err(e) => {
                error.set(Some(feedback(&e)));
                message.set(String::new())
            }
        }
    };
    let account_changed = auth.read().current_user_id != account();
    rsx! {
        AuthRequired {

            div { class: "container grocery-page pantry-page",
                if account_changed {
                    p { class: "grocery-error", role: "alert", "{ACCOUNT_CHANGED_FEEDBACK}" }
                } else {
                    if let Some(problem) = error() {
                        div { class: "grocery-error", role: "alert",
                            p { "{problem}" }
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
                    p { class: "sr-only", role: "status", "{message}" }
                    div { class: "grocery-workspace",
                        aside {
                            class: "grocery-rail neumo-raised",
                            aria_label: "Your pantries",
                            div { class: "grocery-eyebrow", "Your pantries" }
                            if let Some(rows) = sidebar.clone() {
                                div { class: "grocery-list-links",
                                    for row in rows {
                                        Link {
                                            key: "{row.id}",
                                            to: Route::PantryDetail {
                                                id: row.id.to_string(),
                                            },
                                            class: if Some(row.id) == parsed { "grocery-list-link is-current pantry-profile-link" } else { "grocery-list-link pantry-profile-link" },
                                            aria_current: if Some(row.id) == parsed { "page" } else { "false" },
                                            div {

                                                PantryIcon { kind: "home" }
                                                strong { "{row.name}" }
                                            }
                                            small { "{row.ingredient_count} ingredients" }
                                        }
                                    }
                                }
                            } else if summaries.read().as_ref().is_some_and(Result::is_err) {
                                p { class: "grocery-muted", "Your pantries couldn’t be loaded." }
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| refresh += 1,
                                    "Try again"
                                }
                            } else {
                                LoadingSpinner {
                                }
                            }
                            Link {
                                to: Route::PantryNew {},
                                class: "btn btn-primary grocery-new-link",
                                PantryIcon { kind: "plus" }
                                "New pantry"
                            }
                        }
                        div { class: "grocery-workspace-main",
                            if invalid {
                                div { class: "grocery-state",
                                    h1 { "Pantry not found" }
                                    p { "Check the pantry address or choose a saved pantry." }
                                }
                            } else if let Some(p) = kitchen.clone() {
                                header { class: "grocery-list-heading",
                                    div {

                                        div { class: "grocery-eyebrow", "Pantry" }
                                        h1 { "{p.name}" }
                                    }
                                    div { class: "grocery-heading-actions",
                                        button {
                                            class: "btn btn-secondary",
                                            disabled,
                                            onclick: {
                                                let name = p.name.clone();
                                                let note = p.note.clone();
                                                move |_| {
                                                    edit_name.set(name.clone());
                                                    edit_note.set(note.clone());
                                                    edit_open.set(true);
                                                }
                                            },
                                            "Edit pantry"
                                        }
                                        button {
                                            class: "btn btn-ghost",
                                            disabled: disabled || pending_add().is_some(),
                                            onclick: {
                                                let p = p.clone();
                                                move |_| {
                                                    if disabled {
                                                        return;
                                                    }
                                                    let request = duplicate()
                                                        .unwrap_or_else(|| PantryCreate {
                                                            request_id: Uuid::new_v4(),
                                                            name: format!(
                                                                "{} copy",
                                                                p.name.chars().take(95).collect::<String>(),
                                                            ),
                                                            note: p.note.clone(),
                                                            copy: Some(PantryCopy {
                                                                id: p.id,
                                                                expected_revision: p.revision,
                                                            }),
                                                            ..Default::default()
                                                        });
                                                    duplicate.set(Some(request.clone()));
                                                    busy.set(true);
                                                    spawn(async move {
                                                        let result = create_kitchen(account(), request).await;
                                                        if *current_route.peek() != parsed {
                                                            return;
                                                        }
                                                        busy.set(false);
                                                        match result {
                                                            Ok(saved) => {
                                                                if account_completion_matches(
                                                                    account(),
                                                                    auth.peek().current_user_id,
                                                                    saved.owner_id,
                                                                ) {
                                                                    duplicate.set(None);
                                                                    refresh += 1;
                                                                    nav.push(Route::PantryDetail {
                                                                        id: saved.id.to_string(),
                                                                    });
                                                                } else {
                                                                    error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                                                                }
                                                            }
                                                            Err(e) => {
                                                                // A revision conflict is a definite pre-insert rejection.
                                                                // Allow a fresh duplicate after loading the new source.
                                                                if e.to_string().contains("changed in another session") {
                                                                    duplicate.set(None);
                                                                    refresh += 1;
                                                                }
                                                                error.set(Some(feedback(&e)));
                                                            }
                                                        }
                                                    });
                                                }
                                            },
                                            if duplicate().is_some() {
                                                "Retry duplicate"
                                            } else {
                                                "Duplicate pantry"
                                            }
                                        }
                                        button {
                                            class: "btn btn-ghost",
                                            disabled,
                                            onclick: move |_| delete_open.set(true),
                                            "Delete pantry"
                                        }
                                    }
                                }
                                div { class: "grocery-detail-layout has-inspector pantry-detail-layout",
                                    section { class: "grocery-shopping neumo-raised",
                                        div { class: "grocery-section-heading",
                                            h2 { "Available ingredients" }
                                            span { class: "grocery-pill",
                                                "{p.entries.iter().filter(|e|e.kind==PantryKind::Ingredient&&e.usable()).count()} in stock"
                                            }
                                        }

                                        div { class: "grocery-tools",
                                            label {
                                                class: "sr-only",
                                                r#for: "pantry-search",
                                                "Search inventory"
                                            }
                                            input {
                                                id: "pantry-search",
                                                class: "input input--shared neumo-inset",
                                                placeholder: "Find an ingredient…",
                                                value: "{filter}",
                                                oninput: move |e| filter.set(e.value()),
                                            }
                                            label {
                                                class: "sr-only",
                                                r#for: "pantry-filter",
                                                "Availability filter"
                                            }
                                            select {
                                                id: "pantry-filter",
                                                class: "input input--shared neumo-inset",
                                                value: "{availability}",
                                                onchange: move |e| availability.set(e.value()),
                                                option { value: "all", "Everything" }
                                                option { value: "in", "Available" }
                                                option { value: "out", "Out of stock" }
                                            }
                                        }
                                        div { class: "pantry-rows",
                                            for item in p.entries
                                                .iter()
                                                .filter(|e| e.kind == kind)
                                                .filter(|e| {
                                                    e.stock.fields.name.to_lowercase().contains(&filter().to_lowercase())
                                                })
                                                .filter(|e| {
                                                    availability() == "all" || (availability() == "in" && e.usable())
                                                        || (availability() == "out" && !e.usable())
                                                })
                                                .cloned()
                                            {
                                                div {
                                                    key: "{item.id}",
                                                    class: if item.usable() { "pantry-row" } else { "pantry-row pantry-row-out" },
                                                    div { class: "pantry-row-name",
                                                        strong { "{item.stock.fields.name}" }
                                                        small { "{item.status()}" }
                                                    }

                                                    span { class: "grocery-amount",
                                                        if item.stock.fields.amount.trim().is_empty() {
                                                            "—"
                                                        } else {
                                                            "{item.amount_label()}"
                                                        }
                                                    }

                                                    button {
                                                        class: "pantry-stock-toggle",
                                                        disabled: disabled || pending_add().is_some() || duplicate().is_some(),
                                                        aria_pressed: item.stock.available,
                                                        aria_label: if item.stock.available { format!("Mark {} {}", item.stock.fields.name, "out") } else { format!("Restore {}", item.stock.fields.name) },
                                                        onclick: move |_| {
                                                            if disabled {
                                                                return;
                                                            }
                                                            busy.set(true);
                                                            spawn(async move {
                                                                done(
                                                                    update_pantry_entry(
                                                                            account(),
                                                                            p.id,
                                                                            item.kind,
                                                                            item.id,
                                                                            item.revision,
                                                                            PantryChange::Availability(!item.stock.available),
                                                                        )
                                                                        .await,
                                                                )
                                                            });
                                                        },
                                                        PantryIcon { kind: if item.stock.available { "check" } else { "plus" } }
                                                    }
                                                    button {
                                                        class: "btn btn-ghost pantry-edit-trigger",
                                                        id: "pantry-edit-{item.id}",
                                                        aria_label: "Edit {item.stock.fields.name}",
                                                        disabled,
                                                        onclick: move |_| {
                                                            selected.set(Some(item.id));
                                                            spawn(async move {
                                                                let _ = document::eval(
                                                                        "document.getElementById('pantry-editor')?.focus();document.getElementById('pantry-editor')?.scrollIntoView({block:'nearest',behavior:'smooth'})",
                                                                    )
                                                                    .await;
                                                            });
                                                        },
                                                        PantryIcon { kind: "pen" }
                                                    }
                                                }
                                            }
                                            if !p.entries.iter().any(|e| e.kind == kind) {
                                                p { class: "grocery-muted pantry-empty",
                                                    "No items here yet. Add your first one below."
                                                }
                                            }
                                        }
                                        PantryQuickAdd {
                                            key: "{p.id}:{kind:?}",

                                            fields: addition,
                                            disabled: disabled || duplicate().is_some(),
                                            frozen: pending_add().is_some(),
                                            on_add: move |fields: PantryFields| {
                                                if disabled {
                                                    return;
                                                }
                                                let (key, stock) = pending_add()
                                                    .unwrap_or_else(|| (
                                                        Uuid::new_v4(),

                                                        PantryStock {
                                                            fields,
                                                            available: true,
                                                        },
                                                    ));
                                                if let Err(problem) = stock.fields.validate(PantryKind::Ingredient) {
                                                    error.set(Some(problem));
                                                    return;
                                                }
                                                pending_add.set(Some((key, stock.clone())));
                                                busy.set(true);
                                                spawn(async move {
                                                    let result = add_pantry_entry(
                                                            account(),
                                                            p.id,
                                                            PantryKind::Ingredient,
                                                            key,
                                                            stock,
                                                        )
                                                        .await;
                                                    if *current_route.peek() == parsed && result.is_ok()
                                                        && auth.peek().current_user_id == account()
                                                    {
                                                        pending_add.set(None);
                                                        addition.set(PantryFields::default());
                                                    }
                                                    done(result);
                                                });
                                            },
                                        }
                                    }
                                    if let Some(entry) = p
                                        .entries
                                        .iter()
                                        .find(|e| e.kind == kind && Some(e.id) == selected())
                                        .or_else(|| p.entries.iter().find(|e| e.kind == kind))
                                        .cloned()
                                    {
                                        PantryEditor {
                                            key: "{entry.id}:{entry.revision}",
                                            profile_id: p.id,
                                            entry,
                                            disabled: disabled || pending_add().is_some() || duplicate().is_some(),
                                            on_saved: move |_| {
                                                refresh += 1;
                                            },
                                        }
                                    } else {
                                        aside { class: "grocery-inspector",
                                            div { class: "grocery-eyebrow", "Inventory details" }
                                            h2 { "A little room to grow." }
                                            p { class: "grocery-muted",
                                                "Add an ingredient to start keeping stock."
                                            }
                                        }
                                    }
                                }
                                if edit_open() {
                                    SharedModal {
                                        aria_labelledby: "pantry-profile-title",
                                        focus_selector: "#pantry-profile-name",
                                        pending: busy(),
                                        on_dismiss: move |_| edit_open.set(false),
                                        div {
                                            class: "modal-card pantry-modal",
                                            onclick: move |e| e.stop_propagation(),
                                            h2 { id: "pantry-profile-title", "Edit pantry" }
                                            form {
                                                class: "pantry-profile-form",
                                                onsubmit: move |e| {
                                                    e.prevent_default();
                                                    if busy() {
                                                        return;
                                                    }
                                                    if let Err(problem) = validate_profile(&edit_name(), &edit_note()) {
                                                        error.set(Some(problem));
                                                        return;
                                                    }
                                                    busy.set(true);
                                                    spawn(async move {
                                                        let result = edit_kitchen(
                                                                account(),
                                                                p.id,
                                                                p.revision,
                                                                edit_name(),
                                                                edit_note(),
                                                            )
                                                            .await;
                                                        if *current_route.peek() == parsed && result.is_ok() {
                                                            edit_open.set(false)
                                                        }
                                                        done(result);
                                                    });
                                                },
                                                label { r#for: "pantry-profile-name",
                                                    "Pantry name"
                                                }
                                                input {
                                                    id: "pantry-profile-name",
                                                    class: "input input--shared neumo-inset",
                                                    value: "{edit_name}",
                                                    maxlength: 100,
                                                    required: true,
                                                    disabled: busy(),
                                                    oninput: move |e| edit_name.set(e.value()),
                                                }
                                                label { r#for: "pantry-profile-note",
                                                    "Note"
                                                }
                                                input {
                                                    id: "pantry-profile-note",
                                                    class: "input input--shared neumo-inset",
                                                    value: "{edit_note}",
                                                    maxlength: 500,
                                                    disabled: busy(),
                                                    oninput: move |e| edit_note.set(e.value()),
                                                }
                                                if let Some(problem) = error() {
                                                    p {
                                                        class: "grocery-error",
                                                        role: "alert",
                                                        "{problem}"
                                                    }
                                                }
                                                div { class: "grocery-dialog-actions",
                                                    button {
                                                        class: "btn btn-secondary",
                                                        r#type: "button",
                                                        disabled: busy(),
                                                        onclick: move |_| edit_open.set(false),
                                                        "Cancel"
                                                    }
                                                    button {
                                                        class: "btn btn-primary",
                                                        disabled: busy(),
                                                        "Save pantry"
                                                    }
                                                }
                                            }

                                        }
                                    }
                                }
                                if delete_open() {
                                    SharedModal {
                                        aria_labelledby: "pantry-delete-title",
                                        focus_selector: "#pantry-keep-profile",
                                        pending: busy(),
                                        on_dismiss: move |_| delete_open.set(false),
                                        div {
                                            class: "modal-card pantry-modal",
                                            onclick: move |e| e.stop_propagation(),
                                            h2 { id: "pantry-delete-title", "Delete {p.name}?" }
                                            p { class: "grocery-muted",
                                                "This removes this pantry and its inventory."
                                            }
                                            if let Some(problem) = error() {
                                                p {
                                                    class: "grocery-error",
                                                    role: "alert",
                                                    "{problem}"
                                                }
                                            }
                                            div { class: "grocery-dialog-actions",
                                                button {
                                                    id: "pantry-keep-profile",
                                                    class: "btn btn-secondary",
                                                    disabled: busy(),
                                                    onclick: move |_| delete_open.set(false),
                                                    "Keep pantry"
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
                                                            let result = delete_kitchen(account(), p.id, p.revision).await;
                                                            if *current_route.peek() != parsed {
                                                                return;
                                                            }
                                                            busy.set(false);
                                                            match result {
                                                                Ok(()) => {
                                                                    if auth.peek().current_user_id != account() {
                                                                        error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                                                                        return;
                                                                    }
                                                                    if let Some(owner) = account() {
                                                                        remember_selection(owner, None)
                                                                    }
                                                                    delete_open.set(false);
                                                                    refresh += 1;
                                                                    nav.replace(Route::Pantry {});
                                                                }
                                                                Err(e) => error.set(Some(feedback(&e))),
                                                            }
                                                        });
                                                    },
                                                    "Delete pantry"
                                                }
                                            }

                                        }
                                    }
                                }
                            } else if loading {
                                div { class: "grocery-state",
                                    LoadingSpinner {
                                    }
                                    p { "Loading your pantry…" }
                                }
                            } else if details.read().as_ref().is_some_and(Result::is_err) {
                                div { class: "grocery-state",
                                    h1 { "Pantry unavailable" }
                                    p { class: "grocery-muted",
                                        "This pantry could not be opened. Choose another pantry or retry."
                                    }
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| details.restart(),
                                        "Try again"
                                    }
                                }
                            } else {
                                div { class: "grocery-state neumo-raised",
                                    PantryIcon { kind: "home" }
                                    h1 { "A pantry of your own." }
                                    p { "Keep track of your ingredients, wherever you cook." }
                                    Link {
                                        to: Route::PantryNew {},
                                        class: "btn btn-primary",
                                        "Create your first pantry"
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
#[component]
fn PantryQuickAdd(
    mut fields: Signal<PantryFields>,
    disabled: bool,
    #[props(default)] frozen: bool,
    on_add: EventHandler<PantryFields>,
) -> Element {
    rsx! {
        form {
            class: "grocery-add-row pantry-add",
            onsubmit: move |e| {
                e.prevent_default();
                if !disabled {
                    on_add.call(fields());
                }
            },
            label {

                span { class: "sr-only", "Ingredient name" }
                input {
                    class: "input input--shared neumo-inset",
                    placeholder: "Add an ingredient…",
                    value: "{fields().name}",
                    disabled: disabled || frozen,
                    maxlength: 500,
                    required: true,
                    oninput: move |e| fields.write().name = e.value(),
                }
            }

            label {

                span { class: "sr-only", "Amount" }
                input {
                    class: "input input--shared neumo-inset",
                    value: "{fields().amount}",
                    placeholder: "Qty",
                    disabled: disabled || frozen,
                    maxlength: 64,
                    oninput: move |e| fields.write().amount = e.value(),
                }
            }
            label {

                span { class: "sr-only", "Unit" }
                input {
                    class: "input input--shared neumo-inset",
                    value: "{fields().unit}",
                    placeholder: "Unit",
                    disabled: disabled || frozen,
                    maxlength: 64,
                    oninput: move |e| fields.write().unit = e.value(),
                }
            }

            button {
                class: "btn btn-secondary",
                r#type: "submit",
                disabled,
                aria_label: "Add ingredient",
                if frozen {
                    "Retry"
                } else {
                    PantryIcon { kind: "plus" }
                }
            }
        }
    }
}
#[component]
pub fn PantryNew() -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let nav = use_navigator();
    let mut name = use_signal(String::new);
    let mut note = use_signal(String::new);
    let mut ingredients = use_signal(Vec::<PantryStock>::new);
    let mut ingredient = use_signal(PantryFields::default);
    let mut busy = use_signal(|| false);
    let mut pending = use_signal(|| None::<PantryCreate>);
    let mut created = use_signal(|| None::<Uuid>);
    let mut error = use_signal(|| None::<String>);
    let mut draft_edit = use_signal(|| None::<(usize, PantryStock)>);
    let mut edited = use_signal(PantryFields::default);
    let frozen = busy() || pending().is_some() || created().is_some();
    let account_changed = auth.read().current_user_id != account();
    let add_initial = move |fields: PantryFields| {
        if frozen {
            return;
        }
        if let Err(problem) = fields.validate(PantryKind::Ingredient) {
            error.set(Some(problem));
            return;
        }
        if ingredients.read().len() >= MAX_INGREDIENTS {
            error.set(Some("This pantry has reached its ingredient limit.".into()));
            return;
        }
        ingredients.write().push(PantryStock {
            fields,
            available: true,
        });
        ingredient.set(PantryFields::default());
        error.set(None);
    };
    rsx! {
        AuthRequired {

            div { class: "container grocery-create-page pantry-create-page",
                nav { class: "grocery-breadcrumb", aria_label: "Breadcrumb",
                    Link { to: Route::Pantry {}, "Pantry" }
                    span { " / " }
                    span { "New pantry" }
                }
                if account_changed {
                    p { class: "grocery-error", role: "alert", "{ACCOUNT_CHANGED_FEEDBACK}" }
                } else {
                    header { class: "grocery-create-heading",
                        div { class: "grocery-eyebrow", "A new pantry" }
                        h1 { "A place for what’s on hand." }
                        p { "Start with a name. Add your ingredients whenever you’re ready." }
                    }
                    div { class: "neumo-raised pantry-create-card",
                        div { class: "pantry-create-intro",
                            div {

                                h2 { "Make yourself at home." }
                                p { class: "grocery-muted", "One pantry, just the things you have." }
                            }
                            PantryIcon {
                            }
                        }
                        label { r#for: "pantry-create-name", "Pantry name" }
                        input {
                            id: "pantry-create-name",
                            class: "input input--shared neumo-inset",
                            value: "{name}",
                            placeholder: "e.g. Home pantry",
                            maxlength: 100,
                            required: true,
                            disabled: frozen,
                            oninput: move |e| name.set(e.value()),
                        }
                        label { r#for: "pantry-create-note",
                            "Note"
                            span { class: "grocery-muted", "Optional" }
                        }
                        input {
                            id: "pantry-create-note",
                            class: "input input--shared neumo-inset",
                            value: "{note}",
                            placeholder: "A little context for this pantry…",
                            maxlength: 500,
                            disabled: frozen,
                            oninput: move |e| note.set(e.value()),
                        }
                        section { class: "pantry-initial-section",
                            div { class: "pantry-initial-heading",
                                h3 { "Starting ingredients" }
                                span { class: "grocery-muted", "Optional · {ingredients.read().len()}" }
                            }
                            PantryDraftRows {

                                rows: ingredients(),
                                disabled: frozen,
                                on_remove: move |i| {
                                    ingredients.write().remove(i);
                                },
                                on_edit: move |i: usize| {
                                    let stock = ingredients()[i].clone();
                                    edited.set(stock.fields.clone());
                                    draft_edit.set(Some((i, stock)));
                                },
                            }
                            PantryQuickAdd {

                                fields: ingredient,
                                disabled: frozen,
                                on_add: add_initial,
                            }
                        }

                        if let Some(problem) = error() {
                            p { class: "grocery-error", role: "alert", "{problem}" }
                        }
                        if let Some(id) = created() {
                            Link {
                                to: Route::PantryDetail {
                                    id: id.to_string(),
                                },
                                class: "btn btn-primary",
                                "Open created pantry"
                            }
                        }
                        div { class: "grocery-create-actions",
                            Link { to: Route::Pantry {}, class: "btn btn-ghost", "Cancel" }
                            button {
                                class: "btn btn-primary",
                                disabled: busy() || created().is_some(),
                                onclick: move |_| {
                                    if busy() || created().is_some() {
                                        return;
                                    }
                                    let request = if let Some(request) = pending() {
                                        request
                                    } else {
                                        let mut initial = ingredients();
                                        let fields = ingredient();
                                        if !fields.name.is_empty() || !fields.amount.is_empty()
                                            || !fields.unit.is_empty()
                                        {
                                            if let Err(problem) = fields.validate(PantryKind::Ingredient) {
                                                error.set(Some(problem));
                                                return;
                                            }
                                            initial
                                                .push(PantryStock {
                                                    fields,
                                                    available: true,
                                                });
                                        }
                                        let request = PantryCreate {
                                            request_id: Uuid::new_v4(),
                                            name: name(),
                                            note: note(),
                                            ingredients: initial,
                                            equipment: Vec::new(),
                                            copy: None,
                                        };
                                        if let Err(problem) = request.validate() {
                                            error.set(Some(problem));
                                            return;
                                        }
                                        request
                                    };
                                    pending.set(Some(request.clone()));
                                    busy.set(true);
                                    error.set(None);
                                    spawn(async move {
                                        let result = create_kitchen(account(), request).await;
                                        busy.set(false);
                                        match result {
                                            Ok(saved) => {
                                                created.set(Some(saved.id));
                                                if account_completion_matches(
                                                    account(),
                                                    auth.peek().current_user_id,
                                                    saved.owner_id,
                                                ) {
                                                    remember_selection(saved.owner_id, Some(saved.id));
                                                    nav.push(Route::PantryDetail {
                                                        id: saved.id.to_string(),
                                                    });
                                                } else {
                                                    error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                                                }
                                            }
                                            Err(e) => error.set(Some(feedback(&e))),
                                        }
                                    });
                                },
                                if busy() {
                                    "Creating…"
                                } else if pending().is_some() {
                                    "Retry create"
                                } else {
                                    "Create pantry"
                                }
                            }
                        }
                    }
                    if let Some((index, stock)) = draft_edit() {
                        SharedModal {
                            aria_labelledby: "pantry-draft-title",
                            focus_selector: "#pantry-draft-name",
                            on_dismiss: move |_| draft_edit.set(None),
                            div {
                                class: "modal-card pantry-modal",
                                onclick: move |e| e.stop_propagation(),
                                h2 { id: "pantry-draft-title", "Edit starting stock" }
                                form {
                                    class: "pantry-profile-form",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        let fields = edited();
                                        if let Err(problem) = fields.validate(PantryKind::Ingredient) {
                                            error.set(Some(problem));
                                            return;
                                        }
                                        if let Some(row) = ingredients.write().get_mut(index) {
                                            *row = PantryStock {
                                                fields,
                                                available: stock.available,
                                            };
                                        }
                                        draft_edit.set(None);
                                    },
                                    PantryFieldsInput {
                                        fields: edited,

                                        prefix: "pantry-draft",
                                    }
                                    if let Some(problem) = error() {
                                        p {
                                            class: "grocery-error",
                                            role: "alert",
                                            "{problem}"
                                        }
                                    }
                                    div { class: "grocery-dialog-actions",
                                        button {
                                            r#type: "button",
                                            class: "btn btn-secondary",
                                            onclick: move |_| draft_edit.set(None),
                                            "Cancel"
                                        }
                                        button { class: "btn btn-primary", "Save item" }
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
#[component]
fn PantryDraftRows(
    rows: Vec<PantryStock>,
    disabled: bool,
    on_remove: EventHandler<usize>,
    on_edit: EventHandler<usize>,
) -> Element {
    rsx! {
        div { class: "pantry-draft-rows",
            if rows.is_empty() {
                p { class: "grocery-muted", "Add anything you have on hand, or start empty." }
            }
            for (index, row) in rows.into_iter().enumerate() {
                div { class: "pantry-draft-row",
                    strong { "{row.fields.name}" }

                    span {

                        if row.fields.amount.trim().is_empty() {
                            "Quantity unknown"
                        } else {
                            "{row.fields.amount} {row.fields.unit}"
                        }
                    }

                    button {
                        class: "btn btn-ghost",
                        disabled,
                        aria_label: "Edit initial {row.fields.name}",
                        onclick: move |_| on_edit.call(index),
                        PantryIcon { kind: "pen" }
                    }
                    button {
                        class: "btn btn-ghost",
                        disabled,
                        aria_label: "Remove initial {row.fields.name}",
                        onclick: move |_| on_remove.call(index),
                        "×"
                    }
                }
            }
        }
    }
}
