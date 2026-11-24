use crate::api::comments::{change, thread, Change, Comment};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn RecipeDiscussion(recipe: Uuid) -> Element {
    let auth = crate::auth::context::use_auth();
    let revision = use_signal(|| 0_u64);
    use_context_provider(|| revision);
    rsx! {
        section { class: "neumo-card recipe-discussion", aria_label: "Recipe discussion",
            h2 { "Discussion" }
            if auth.is_authenticated {
                Composer { recipe, parent: None }
            } else {
                a { href: "/login?redirect_uri=/recipes/{recipe}", "Sign in to comment" }
            }
            ThreadPage { recipe, parent: None, depth: 0, after: 0 }
        }
    }
}

#[component]
fn ThreadPage(recipe: Uuid, parent: Option<i64>, depth: usize, after: i64) -> Element {
    let revision = use_context::<Signal<u64>>();
    let mut more = use_signal(|| false);
    let mut rows = use_resource(move || {
        revision();
        thread(recipe, parent, after)
    });
    let data = rows.read();
    match data.as_ref() {
        None => rsx! { p { role: "status", "Loading comments…" } },
        Some(Err(_)) => {
            rsx! { button { class: "btn", onclick: move |_| rows.restart(), "Retry loading comments" } }
        }
        Some(Ok(items)) => rsx! {
            if items.is_empty() && parent.is_none() && after == 0 { p { "No comments yet." } }
            for item in items.iter().take(20) {
                CommentRow { key: "{item.id}", recipe, item: item.clone(), depth, parent }
            }
            if items.len()>20 {
                if more() {
                    ThreadPage { key: "{items[19].id}", recipe, parent, depth, after: items[19].id }
                } else {
                    button { class: "btn", onclick: move |_| more.set(true), "Load more comments" }
                }
            }
        },
    }
}

#[component]
fn CommentRow(recipe: Uuid, item: Comment, depth: usize, parent: Option<i64>) -> Element {
    let auth = crate::auth::context::use_auth();
    let account = crate::auth::account_binding::use_form_account();
    let mut revision = use_context::<Signal<u64>>();
    let mut expanded = use_signal(|| false);
    let mut replying = use_signal(|| false);
    let mut editing = use_signal(|| false);
    let mut error = use_signal(String::new);
    let id = item.id;
    let author = item.author.as_deref().unwrap_or("Former member");
    let date = item.created_at.format("%Y-%m-%d %H:%M UTC");
    let indent = if depth == 1 || depth == 2 {
        "1rem"
    } else {
        "0"
    };
    rsx! {
        article { id: "comment-{id}", style: "margin-inline-start: {indent}; padding-block: 0.5rem",
            if depth > 2 {
                if let Some(parent) = parent { a { href: "#comment-{parent}", "Replying to comment {parent}" } }
            }
            if let Some(body) = item.body {
                strong { "{author}" }
                small { " · Version {item.version_number} · {date}" }
                p { white_space: "pre-wrap", overflow_wrap: "anywhere", "{body}" }
                if auth.is_authenticated && item.author_id == auth.current_user_id {
                    button { class: "btn", onclick: move |_| editing.toggle(), "Edit" }
                    button { class: "btn", onclick: move |_| async move {
                        if change(account(),recipe,Change::Delete{id}).await.is_ok() { revision += 1; }
                        else { error.set("Could not delete comment.".into()); }
                    }, "Delete" }
                    if editing() {
                        Composer { recipe, edit: Some(id), initial: body.clone(), ondone: move |_| editing.set(false) }
                    }
                }
            } else { p { "Comment deleted" } }
            if !error().is_empty() { p { role: "alert", "{error}" } }
            if auth.is_authenticated {
                button { class: "btn", onclick: move |_| replying.toggle(), "Reply" }
                if replying() {
                    Composer { recipe, parent: Some(id), ondone: move |_| { replying.set(false); expanded.set(true); } }
                }
            }
            if item.has_replies {
                button { class: "btn", aria_expanded: expanded(), onclick: move |_| expanded.toggle(),
                    if expanded() { "Hide replies" } else { "Show replies" }
                }
                if expanded() { ThreadPage { recipe, parent: Some(id), depth: depth+1, after: 0 } }
            }
        }
    }
}

#[component]
fn Composer(
    recipe: Uuid,
    parent: Option<i64>,
    edit: Option<i64>,
    #[props(default)] initial: String,
    ondone: Option<EventHandler<()>>,
) -> Element {
    let mut draft = use_signal(|| initial);
    let mut request_id = use_signal(Uuid::new_v4);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let account = crate::auth::account_binding::use_form_account();
    let mut revision = use_context::<Signal<u64>>();
    let label = if edit.is_some() {
        "Edit comment"
    } else if parent.is_some() {
        "Your reply"
    } else {
        "Your comment"
    };
    rsx! {
        form { onsubmit: move |_| async move {
            if busy() { return; }
            busy.set(true);
            error.set(String::new());
            let body = draft();
            let action = match edit { Some(id)=> Change::Edit{id,body}, None=>Change::Publish{parent,body,request_id:request_id()} };
            match change(account(),recipe,action).await {
                Ok(())=> { request_id.set(Uuid::new_v4()); draft.set(String::new()); revision += 1; if let Some(done)=ondone {done.call(());} }
                Err(_)=>error.set("Could not save. Your text is still here.".into()),
            }
            busy.set(false);
        },
            label { "{label}"
                textarea { value: "{draft}", maxlength: "2000", required: true, disabled: busy(), oninput: move |e| { draft.set(e.value()); request_id.set(Uuid::new_v4()); } }
            }
            button { class: "btn btn-primary", r#type: "submit", disabled: busy() || draft().trim().is_empty(), if edit.is_some() { "Save changes" } else { "Publish" } }
            if !error().is_empty() { p { role: "alert", "{error}" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::context::AuthContext;

    #[test]
    fn signed_out_reader_sees_nested_context_without_mutation_controls() {
        let mut dom = VirtualDom::new(|| {
            let auth = use_signal(AuthContext::default);
            use_context_provider(|| auth);
            let revision = use_signal(|| 0_u64);
            use_context_provider(|| revision);
            let item = Comment {
                id: 12,
                author_id: Some(Uuid::nil()),
                author: Some("Robin".into()),
                body: Some("Used oat milk.".into()),
                version_number: 3,
                created_at: chrono::Utc::now(),
                has_replies: true,
            };
            rsx! { CommentRow { recipe: Uuid::nil(), item, depth: 4, parent: Some(11) } }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        for expected in [
            "Used oat milk.",
            "Version 3",
            "href=\"#comment-11\"",
            "Show replies",
        ] {
            assert!(html.contains(expected), "{html}");
        }
        for action in [">Edit<", ">Delete<", ">Reply<"] {
            assert!(!html.contains(action), "{html}");
        }
    }
}
