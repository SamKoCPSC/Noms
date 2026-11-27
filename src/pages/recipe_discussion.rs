use crate::api::comments::{change, thread, Change, Comment};
use crate::components::base::{Avatar, AvatarSize, PageIcon, PageIconKind};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Editor {
    New,
    Reply(i64),
    Edit(i64),
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Panel {
    Editor(Editor),
    Replies(i64),
    Report(i64),
    NextPage(Option<i64>, i64),
}

#[derive(Clone)]
struct Draft {
    body: String,
    request_id: Uuid,
}

#[derive(Default)]
struct DiscussionState {
    drafts: HashMap<Editor, Draft>,
    report_reasons: HashMap<i64, String>,
    open: HashSet<Panel>,
    collapsed_threads: HashSet<i64>,
}

impl DiscussionState {
    fn toggle(&mut self, panel: Panel) {
        if !self.open.remove(&panel) {
            self.open.insert(panel);
        }
    }

    fn saved(&mut self, editor: Editor, request_id: Uuid) {
        if self
            .drafts
            .get(&editor)
            .is_some_and(|d| d.request_id != request_id)
        {
            return;
        }
        self.drafts.remove(&editor);
        self.open.remove(&Panel::Editor(editor));
        if let Editor::Reply(parent) = editor {
            self.open.insert(Panel::Replies(parent));
        }
    }
}

#[component]
pub fn RecipeDiscussionScope(recipe: Uuid) -> Element {
    let state = use_signal(DiscussionState::default);
    use_context_provider(|| state);
    rsx! { RecipeDiscussion { recipe } }
}

#[component]
fn RecipeDiscussion(recipe: Uuid) -> Element {
    let auth = crate::auth::context::use_auth();
    let revision = use_signal(|| 0_u64);
    use_context_provider(|| revision);
    rsx! {
        section { class: "neumo-card recipe-discussion", aria_label: "Recipe discussion",
            header { class: "discussion-heading",
                div { class: "brand-title",
                    PageIcon { kind: PageIconKind::Editor, compact: true }
                    h2 { "Discussion" }
                }
                if !auth.is_authenticated {
                    a { class: "btn btn-secondary", href: "/login?redirect_uri=/recipes/{recipe}", "Sign in to comment" }
                }
            }
            if auth.is_authenticated {
                Composer { recipe, parent: None }
            }
            ThreadPage { recipe, parent: None, depth: 0, after: 0 }
        }
    }
}

#[component]
fn ThreadPage(recipe: Uuid, parent: Option<i64>, depth: usize, after: i64) -> Element {
    let revision = use_context::<Signal<u64>>();
    let mut state = use_context::<Signal<DiscussionState>>();
    let page = Panel::NextPage(parent, after);
    let mut rows = use_resource(move || {
        let requested_revision = revision();
        async move {
            thread(recipe, parent, after)
                .await
                .map(|items| (requested_revision, items))
        }
    });
    let data = rows.read();
    match data.as_ref() {
        None => rsx! { p { role: "status", "Loading comments…" } },
        Some(Err(_)) => {
            rsx! { button { class: "btn", onclick: move |_| rows.restart(), "Retry loading comments" } }
        }
        Some(Ok((loaded_revision, items))) => rsx! {
            if items.is_empty() && parent.is_none() && after == 0 { p { "No comments yet." } }
            for item in items.iter().take(20) {
                CommentRow { key: "{item.id}", recipe, item: item.clone(), depth, parent, loaded_revision: *loaded_revision }
            }
            if items.len()>20 {
                if state.read().open.contains(&page) {
                    ThreadPage { key: "{items[19].id}", recipe, parent, depth, after: items[19].id }
                } else {
                    button { class: "btn", onclick: move |_| { state.write().open.insert(page); }, "Load more comments" }
                }
            }
        },
    }
}

#[component]
fn CommentRow(
    recipe: Uuid,
    item: Comment,
    depth: usize,
    parent: Option<i64>,
    loaded_revision: u64,
) -> Element {
    let auth = crate::auth::context::use_auth();
    let account = crate::auth::account_binding::use_form_account();
    let mut revision = use_context::<Signal<u64>>();
    let mut state = use_context::<Signal<DiscussionState>>();
    let reply = Panel::Editor(Editor::Reply(item.id));
    let edit = Panel::Editor(Editor::Edit(item.id));
    let replies = Panel::Replies(item.id);
    let expanded = state.read().open.contains(&replies);
    let mut error = use_signal(String::new);
    let id = item.id;
    let collapsed = state.read().collapsed_threads.contains(&id);
    let author = item.author.as_deref().unwrap_or("Former member");
    let date = item.created_at.format("%b %-d, %Y · %H:%M UTC");
    let nested = matches!(depth, 1 | 2);
    let mut like_pending = use_signal(|| false);
    let liked = item.liked_by_viewer;
    let count = item.like_count;
    let noun = if count == 1 { "like" } else { "likes" };
    let own = auth.is_authenticated && item.author_id == auth.current_user_id;
    rsx! {
        article {
            id: "comment-{id}", class: "discussion-entry",
            class: if depth == 0 { "discussion-root" },
            class: if nested { "discussion-nested" },
            if depth > 2 {
                if let Some(parent) = parent {
                    a { class: "discussion-parent", href: "#comment-{parent}", "Replying to comment {parent}" }
                }
            }
            header { class: "discussion-byline",
                if item.body.is_some() {
                    span { aria_hidden: "true", Avatar { username: author.to_string(), size: AvatarSize::Small } }
                    div {
                        strong { "{author}" }
                        span { class: "discussion-version", "Version {item.version_number}" }
                        time { datetime: item.created_at.to_rfc3339(), "{date}" }
                    }
                } else {
                    span { class: "discussion-deleted", if item.moderation_hidden { "Comment hidden by moderation" } else { "Comment deleted" } }
                }
                button {
                    class: "discussion-action discussion-toggle discussion-thread-toggle",
                    aria_expanded: !collapsed, aria_controls: "comment-content-{id}",
                    onclick: move |_| {
                        let mut state = state.write();
                        if !state.collapsed_threads.remove(&id) {
                            state.collapsed_threads.insert(id);
                        }
                    },
                    if collapsed { "Expand thread" } else { "Collapse thread" }
                }
            }
            div { id: "comment-content-{id}", hidden: collapsed,
                if let Some(body) = &item.body {
                    p { class: "discussion-body", "{body}" }
                }
                div { class: "discussion-actions",
                    if item.body.is_some() {
                        if auth.is_authenticated {
                            button { class: "discussion-action discussion-like", aria_pressed: liked,
                                aria_label: "Like comment, {count} {noun}", disabled: like_pending() || loaded_revision != revision(),
                                onclick: move |_| async move {
                                    if like_pending() || loaded_revision != revision() { return; }
                                    like_pending.set(true);
                                    error.set(String::new());
                                    if change(account(), recipe, Change::SetLike { id, liked: !liked }).await.is_ok() {
                                        revision += 1;
                                    } else {
                                        error.set("Could not update your like. Try again.".into());
                                    }
                                    like_pending.set(false);
                                },
                                if liked { "♥ Liked" } else { "♡ Like" }
                                " · {count}"
                            }
                        } else {
                            span { class: "discussion-like-count", "{count} {noun}" }
                        }
                    }
                    if auth.is_authenticated {
                        button { class: "discussion-action", aria_expanded: state.read().open.contains(&reply),
                            onclick: move |_| state.write().toggle(reply), "Reply" }
                    }
                    if item.has_replies {
                        button { class: "discussion-action discussion-toggle", aria_expanded: expanded,
                            onclick: move |_| state.write().toggle(replies),
                            if expanded { "Hide replies" } else { "Show replies" }
                        }
                    }
                    if auth.is_authenticated {
                        if let Some(body) = &item.body {
                            ReportComment { recipe, id, body: body.clone() }
                        }
                    }
                    if own {
                        button { class: "discussion-action", aria_expanded: state.read().open.contains(&edit),
                            onclick: move |_| state.write().toggle(edit), "Edit" }
                        button { class: "discussion-action", onclick: move |_| async move {
                            if change(account(),recipe,Change::Delete{id}).await.is_ok() { revision += 1; }
                            else { error.set("Could not delete comment.".into()); }
                        }, "Delete" }
                    }
                }
                if !error().is_empty() { p { role: "alert", "{error}" } }
                if own && state.read().open.contains(&edit) {
                    if let Some(body) = &item.body {
                        Composer { recipe, edit: Some(id), initial: body.clone() }
                    }
                }
                if auth.is_authenticated && state.read().open.contains(&reply) {
                    Composer { recipe, parent: Some(id) }
                }
                if item.has_replies && expanded { ThreadPage { recipe, parent: Some(id), depth: depth+1, after: 0 } }
            }
        }
    }
}

#[component]
fn ReportComment(recipe: Uuid, id: i64, body: String) -> Element {
    let account = crate::auth::account_binding::use_form_account();
    let mut state = use_context::<Signal<DiscussionState>>();
    let panel = Panel::Report(id);
    let open = state.read().open.contains(&panel);
    let reason = state
        .read()
        .report_reasons
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let empty = reason.trim().is_empty();
    let mut busy = use_signal(|| false);
    let mut sent = use_signal(|| false);
    let mut failed = use_signal(|| false);
    rsx! {
        if sent() {
            span { role: "status", "Report received" }
        } else {
            button { class: "discussion-action", aria_expanded: open, disabled: busy(),
                onclick: move |_| state.write().toggle(panel), if open { "Cancel report" } else { "Report" }
            }
            if open {
                form { class: "discussion-report", onsubmit: move |event| {
                    event.prevent_default();
                    let action = Change::Report { id, observed_body: body.clone(), reason: state.peek().report_reasons.get(&id).cloned().unwrap_or_default() };
                    async move {
                        if busy() { return; }
                        busy.set(true);
                        failed.set(false);
                        if change(account(), recipe, action).await.is_ok() {
                            state.write().report_reasons.remove(&id);
                            state.write().open.remove(&panel);
                            sent.set(true);
                        } else {
                            failed.set(true);
                        }
                        busy.set(false);
                    }
                },
                    label { "Reason for reporting"
                        textarea { value: reason, required: true, maxlength: "500", disabled: busy(),
                            oninput: move |event| { state.write().report_reasons.insert(id, event.value()); } }
                    }
                    button { class: "btn btn-primary", disabled: busy() || empty,
                        r#type: "submit", "Submit report" }
                    if failed() { p { role: "alert", "Could not submit your report. Check that the comment is still current and try again." } }
                }
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
) -> Element {
    let editor = match (edit, parent) {
        (Some(id), _) => Editor::Edit(id),
        (_, Some(id)) => Editor::Reply(id),
        _ => Editor::New,
    };
    let mut state = use_context::<Signal<DiscussionState>>();
    let draft = state
        .read()
        .drafts
        .get(&editor)
        .cloned()
        .unwrap_or_else(|| Draft {
            body: initial,
            request_id: Uuid::new_v4(),
        });
    let request_id = draft.request_id;
    let value = draft.body;
    let empty = value.trim().is_empty();
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
    let submitted = value.clone();
    let submit = move || {
        let body = submitted.clone();
        async move {
            if busy() {
                return;
            }
            busy.set(true);
            error.set(String::new());
            let action = match editor {
                Editor::Edit(id) => Change::Edit { id, body },
                _ => Change::Publish {
                    parent,
                    body,
                    request_id,
                },
            };
            match change(account(), recipe, action).await {
                Ok(()) => {
                    state.write().saved(editor, request_id);
                    revision += 1;
                }
                Err(_) => error.set("Could not save. Your text is still here.".into()),
            }
            busy.set(false);
        }
    };
    rsx! {
        form { onsubmit: move |event| { event.prevent_default(); submit() },
            label { "{label}"
                textarea { value: "{value}", maxlength: "2000", required: true, disabled: busy(), oninput: move |e| { state.write().drafts.insert(editor, Draft { body: e.value(), request_id: Uuid::new_v4() }); } }
            }
            button { class: "btn btn-primary", r#type: "submit", disabled: busy() || empty, if edit.is_some() { "Save changes" } else { "Publish" } }
            if !error().is_empty() { p { role: "alert", "{error}" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::context::AuthContext;

    #[test]
    fn publishing_the_root_preserves_an_open_nested_reply_and_its_draft() {
        let mut state = DiscussionState::default();
        let pending = Editor::Reply(12);
        let request_id = Uuid::new_v4();
        for editor in [Editor::New, pending] {
            state.drafts.insert(
                editor,
                Draft {
                    body: format!("text for {editor:?}"),
                    request_id,
                },
            );
        }
        for panel in [
            Panel::NextPage(None, 0),
            Panel::Replies(3),
            Panel::Editor(pending),
        ] {
            state.toggle(panel);
        }

        state.saved(Editor::New, request_id);

        assert!(!state.drafts.contains_key(&Editor::New));
        assert_eq!(state.drafts[&pending].body, "text for Reply(12)");
        assert_eq!(state.drafts[&pending].request_id, request_id);
        assert!(state.open.contains(&Panel::NextPage(None, 0)));
        assert!(state.open.contains(&Panel::Replies(3)));
        assert!(state.open.contains(&Panel::Editor(pending)));
    }

    #[test]
    fn report_form_restores_its_reason_when_replies_remount() {
        use dioxus::html::{SerializedFormData, SerializedHtmlEventConverter, SerializedMouseData};
        use std::rc::Rc;

        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut dom = VirtualDom::new(|| {
            let auth = use_signal(AuthContext::default);
            use_context_provider(|| auth);
            let mut state = use_signal(DiscussionState::default);
            use_context_provider(|| state);
            let replies = Panel::Replies(1);
            rsx! {
                button { onclick: move |_| state.write().toggle(replies), "Toggle replies" }
                if state.read().open.contains(&replies) {
                    ReportComment { recipe: Uuid::nil(), id: 2, body: "Cooking note" }
                }
            }
        });
        let listener = |mutations: dioxus_core::Mutations, event: &str| {
            mutations
                .edits
                .into_iter()
                .find_map(|edit| match edit {
                    dioxus_core::Mutation::NewEventListener { name, id } if name == event => {
                        Some(id)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let click = |dom: &VirtualDom, id| {
            dom.runtime().handle_event(
                "click",
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(
                        SerializedMouseData::default(),
                    ))) as Rc<dyn std::any::Any>,
                    true,
                ),
                id,
            );
        };
        let branch = listener(dom.rebuild_to_vec(), "click");
        click(&dom, branch);
        let report = listener(dom.render_immediate_to_vec(), "click");
        click(&dom, report);
        let input = listener(dom.render_immediate_to_vec(), "input");
        let reason = "Unrelated advertising";
        dom.runtime().handle_event(
            "input",
            Event::new(
                Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                    reason.into(),
                    vec![],
                )))) as Rc<dyn std::any::Any>,
                true,
            ),
            input,
        );
        dom.render_immediate_to_vec();
        click(&dom, branch);
        dom.render_immediate_to_vec();
        assert!(!dioxus_ssr::render(&dom).contains("Reason for reporting"));
        click(&dom, branch);
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(
            html.contains("Cancel report") && html.contains(reason),
            "{html}"
        );
    }

    #[test]
    fn like_button_reenables_only_with_the_refreshed_selection() {
        fn row((loaded_revision, liked): (u64, bool)) -> Element {
            let auth = use_signal(|| AuthContext {
                is_authenticated: true,
                current_user_id: Some(Uuid::nil()),
                ..Default::default()
            });
            use_context_provider(|| auth);
            let revision = use_signal(|| 1_u64);
            use_context_provider(|| revision);
            let state = use_signal(DiscussionState::default);
            use_context_provider(|| state);
            let item = Comment {
                id: 12,
                author_id: None,
                author: None,
                body: Some("Cooking note".into()),
                version_number: 1,
                created_at: chrono::Utc::now(),
                moderation_hidden: false,
                has_replies: false,
                like_count: i64::from(liked),
                liked_by_viewer: liked,
            };
            rsx! { CommentRow { recipe: Uuid::nil(), item, depth: 0, parent: None, loaded_revision } }
        }

        for liked in [false, true] {
            for loaded_revision in [0, 1] {
                let mut dom = VirtualDom::new_with_props(row, (loaded_revision, liked));
                dom.rebuild_in_place();
                let html = dioxus_ssr::render(&dom);
                let button = html
                    .split("discussion-action discussion-like")
                    .nth(1)
                    .expect("like button")
                    .split("</button>")
                    .next()
                    .unwrap();
                assert_eq!(
                    button.contains("disabled"),
                    loaded_revision == 0,
                    "{button}"
                );
                assert!(
                    button.contains(&format!("aria-pressed={liked}")),
                    "{button}"
                );
            }
        }
    }

    #[test]
    fn signed_out_reader_sees_nested_context_without_mutation_controls() {
        let mut dom = VirtualDom::new(|| {
            let auth = use_signal(AuthContext::default);
            use_context_provider(|| auth);
            let revision = use_signal(|| 0_u64);
            use_context_provider(|| revision);
            let state = use_signal(DiscussionState::default);
            use_context_provider(|| state);
            let item = Comment {
                id: 12,
                author_id: Some(Uuid::nil()),
                author: Some("Robin".into()),
                body: Some("Used oat milk.".into()),
                version_number: 3,
                created_at: chrono::Utc::now(),
                moderation_hidden: false,
                has_replies: true,
                like_count: 3,
                liked_by_viewer: false,
            };
            rsx! { CommentRow { recipe: Uuid::nil(), item, depth: 4, parent: Some(11), loaded_revision: 0 } }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        for expected in [
            "Used oat milk.",
            "Version 3",
            "href=\"#comment-11\"",
            "Show replies",
            "3 likes",
        ] {
            assert!(html.contains(expected), "{html}");
        }
        for action in [">Edit<", ">Delete<", ">Reply<"] {
            assert!(!html.contains(action), "{html}");
        }
    }
}
