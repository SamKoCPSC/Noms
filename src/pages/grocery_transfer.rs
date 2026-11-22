use crate::auth::account_binding::use_form_account;
use crate::auth::context::AuthContext;
use crate::components::base::SharedModal;
use crate::grocery::GroceryList;
use crate::Route;
use dioxus::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

#[component]
pub(super) fn PutAwayGroceries(
    list: GroceryList,
    disabled: bool,
    onrefresh: EventHandler<()>,
) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        button { class: "btn btn-secondary", disabled: disabled || !list.items.iter().any(|i| i.checked), onclick: move |_| open.set(true), "Add purchased to pantry" }
        if open() { PutAwayDialog { list: list.clone(), onclose: move |_| { open.set(false); onrefresh.call(()); } } }
    }
}

#[component]
fn PutAwayDialog(list: GroceryList, onclose: EventHandler<()>) -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let mut selected = use_signal(|| {
        list.items
            .iter()
            .filter(|i| i.checked)
            .map(|i| i.id)
            .collect::<HashSet<Uuid>>()
    });
    let mut pantry = use_signal(|| None::<Uuid>);
    let mut retry = use_signal(|| 0_u64);
    let mut busy = use_signal(|| false);
    let mut done = use_signal(|| false);
    let mut remove = use_signal(|| false);
    let mut uncertain = use_signal(|| false);
    let frozen = busy() || uncertain();
    let mut error = use_signal(|| None::<String>);
    let pantries = use_resource(move || {
        let owner = account();
        let _ = retry();
        async move { crate::api::pantry::list_kitchens(owner).await }
    });
    use_effect(move || {
        if pantry().is_none() {
            if let Some(Ok(rows)) = pantries.read().as_ref() {
                let remembered = account().and_then(super::pantry::saved_selection);
                let choice = remembered
                    .filter(|id| rows.iter().any(|p| p.id == *id))
                    .or_else(|| rows.first().map(|p| p.id));
                if choice != pantry() {
                    pantry.set(choice);
                }
            }
        }
    });
    let account_changed = auth.read().current_user_id != account();
    rsx! {
        SharedModal { aria_labelledby: "put-away-title", focus_selector: "#put-away-close", pending: busy(), on_dismiss: move |_| onclose.call(()),
            div { class: "modal-card grocery-edit-dialog", onclick: move |e| e.stop_propagation(),
                h2 { id: "put-away-title", "Add purchased ingredients to pantry" }
                if account_changed { p { role: "alert", "Your account changed. Reload this page before continuing." } }
                else if done() {
                    p { role: "status", "Your purchased ingredients are available in this pantry." }
                    if remove() { p { "The selected ingredients were removed from your grocery list." } }
                    if let Some(id)=pantry() { Link { to: Route::PantryDetail { id: id.to_string() }, class: "btn btn-primary", "Open pantry" } }
                } else {
                    p { "Select ingredients to add to a pantry." }
                    match pantries.read().as_ref() {
                        Some(Ok(rows)) if rows.is_empty() => rsx! { p { "Create a pantry first to put away your groceries." } Link { to: Route::PantryNew {}, "Create pantry" } },
                        Some(Ok(rows)) => rsx! {
                            label { r#for: "put-away-pantry", "Pantry" }
                            select { id: "put-away-pantry", class: "neumo-inset input input--shared", disabled: frozen, value: pantry().map(|id|id.to_string()).unwrap_or_default(), onchange: move |e| pantry.set(Uuid::parse_str(&e.value()).ok()),
                                option { value: "", disabled: true, "Choose a pantry" }
                                for row in rows { option { value: "{row.id}", selected: pantry()==Some(row.id), "{row.name}" } }
                            }
                        },
                        Some(Err(_)) => rsx! { p { role: "alert", "Your pantries couldn’t be loaded." } button { class: "btn btn-secondary", onclick: move |_| retry+=1, "Retry pantries" } },
                        None => rsx! { p { role: "status", "Loading pantries…" } },
                    }
                    div { class: "grocery-put-away-items",
                        for item in list.items.iter().filter(|i| i.checked) {
                            label { class: "grocery-option-toggle", key: "{item.id}",
                                input { r#type: "checkbox", disabled: frozen, checked: selected().contains(&item.id), onchange: { let id=item.id; move |e: FormEvent| { if e.checked() { selected.write().insert(id); } else { selected.write().remove(&id); } } } }
                                span { "{item.fields.name} · {item.fields.amount} {item.fields.unit}" }
                            }
                        }
                    }
                    fieldset {
                        legend { "After adding" }
                        label { class: "grocery-option-toggle",
                            input { r#type: "radio", name: "put-away-after", checked: !remove(), disabled: frozen, onchange: move |_| remove.set(false) }
                            "Keep on grocery list"
                        }
                        label { class: "grocery-option-toggle",
                            input { r#type: "radio", name: "put-away-after", checked: remove(), disabled: frozen, onchange: move |_| remove.set(true) }
                            "Remove from grocery list"
                        }
                    }
                    if let Some(problem)=error() { p { class: "grocery-error", role: "alert", "{problem}" } }
                    button { class: "btn btn-primary", disabled: frozen || pantry().is_none() || selected().is_empty(), onclick: move |_| {
                        let Some(id)=pantry() else { return; };
                        if busy() { return; }
                        busy.set(true);error.set(None);
                        let items=selected().into_iter().collect();
                        let options=crate::grocery::GroceryTransferOptions { remove_from_list:remove() };
                        let list_id=list.id;let revision=list.revision;
                        spawn(async move {
                            match crate::api::grocery_list::transfer_purchased_groceries(account(),list_id,revision,id,items,Some(options)).await {
                                Ok(()) if auth.peek().current_user_id==account() => done.set(true),
                                Ok(()) => error.set(Some("Your account changed. Reload the page to view the result.".into())),
                                Err(ServerFnError::ServerError { message, .. }) if !message.contains("timed out") && !message.contains("could not be saved") => error.set(Some(message)),
                                Err(_) => {
                                    uncertain.set(true);
                                    error.set(Some("We couldn’t confirm the save. Refresh your grocery list and pantry before retrying.".into()));
                                },
                            }
                            busy.set(false);
                        });
                    }, if busy() { "Adding…" } else if error().is_some() && !uncertain() { "Retry add" } else { "Add to pantry" } }
                }
                button { id: "put-away-close", class: "btn btn-ghost", disabled: busy(), onclick: move |_| onclose.call(()), if done() { "Done" } else if uncertain() { "Close" } else { "Cancel" } }
            }
        }
    }
}
