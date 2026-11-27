use crate::api::moderation::{self, Action, Detail, Queue, PAGE_SIZE};
use crate::auth::context::use_auth;
use crate::Route;
use dioxus::prelude::*;
use uuid::Uuid;

const CSS: Asset = asset!("/assets/admin.css");
const FILTERS: [(&str, &str, Queue); 4] = [
    ("waiting", "Needs review", Queue::Waiting),
    ("mine", "Assigned to me", Queue::Mine),
    ("reviewing", "All in review", Queue::Reviewing),
    ("resolved", "Resolved", Queue::Resolved),
];

fn stamp(time: chrono::DateTime<chrono::Utc>) -> String {
    time.format("%Y-%m-%d %H:%M UTC").to_string()
}

struct AdminSection {
    label: &'static str,
    route: fn() -> Route,
}

const ADMIN_SECTIONS: &[AdminSection] = &[AdminSection {
    label: "Comment moderation",
    route: || queue_route(None, 0),
}];

fn queue_route(view: Option<String>, page: u32) -> Route {
    Route::ModerationQueue {
        view,
        page: (page > 0).then_some(page),
    }
}

#[component]
pub(crate) fn AdminMenuLink(identity: Option<Uuid>, menu: bool) -> Element {
    rsx! { for actor in identity {
        PermissionLink { key: "{actor}", actor, menu }
    } }
}

#[component]
fn PermissionLink(actor: Uuid, menu: bool) -> Element {
    let access = use_resource(move || moderation::can_review(Some(actor)));
    if !matches!(&*access.read(), Some(Ok(true))) {
        return rsx! {};
    }
    rsx! { a { href:"/admin", class:if menu {"account-menu-item"} else {"btn btn-secondary"},
    role:if menu {Some("menuitem")} else {None}, tabindex:if menu {Some("-1")} else {None}, "Admin" } }
}

#[component]
fn AdminShell(children: Element) -> Element {
    rsx! {
        document::Stylesheet { href:CSS }
        div { class:"container admin-workspace",
            aside { nav { aria_label:"Admin sections",
                h2 { "Admin" }
                for section in ADMIN_SECTIONS { Link { to:(section.route)(), "{section.label}" } }
            } }
            section { class:"admin-content", {children} }
        }
    }
}

#[component]
pub fn ModerationQueue(view: Option<String>, page: Option<u32>) -> Element {
    let auth = use_auth();
    let page = page.unwrap_or_default();
    rsx! { AdminShell {
        if auth.current_user_id.is_some() { for actor in auth.current_user_id {
            QueueList { key:"{actor}-{view:?}-{page}",actor,view:view.clone(),page }
        } } else { p {"Sign in with a moderator account to continue."} }
    } }
}

#[component]
fn QueueList(actor: Uuid, view: Option<String>, page: u32) -> Element {
    let (slug, _, filter) = FILTERS
        .iter()
        .copied()
        .find(|f| Some(f.0) == view.as_deref())
        .unwrap_or(FILTERS[0]);
    let mut rows = use_resource(move || moderation::list_cases(Some(actor), filter, page));
    rsx! {
        h1 { "Comment moderation" }
        nav { class:"admin-filters", aria_label:"Case queues",
            for (key,label,_) in FILTERS {
                Link { to:queue_route(Some(key.into()),0), aria_current:if key==slug {"page"} else {"false"}, "{label}" }
            }
        }
        match &*rows.read() {
            None => rsx!{p { role:"status", "Loading cases…" }},
            Some(Err(_)) => rsx!{p { role:"alert","Unable to open this queue. Moderator access is required." } button { class:"btn",onclick:move |_| rows.restart(),"Retry" }},
            Some(Ok(cases)) => rsx!{
                if cases.is_empty() { p { if filter==Queue::Waiting {"No comments awaiting review."} else {"No cases on this page."} } }
                for case in cases.iter().take(PAGE_SIZE) {
                    article { key:"{case.id}", class:"neumo-card admin-card",
                        Link { to:Route::ModerationCase { id:case.id, view:view.clone(), page:Some(page) }, "Case #{case.id} · {case.title}" }
                        p { "{case.excerpt}" }
                        small { "{case.status} · Reports: {case.reports} · Queued {stamp(case.queued_at)}" }
                    }
                }
                nav { class:"admin-filters", aria_label:"Queue pages",
                    if page>0 { Link { to:queue_route(view.clone(),page-1),"Previous" } }
                    if cases.len()>PAGE_SIZE { Link { to:queue_route(view.clone(),page+1),"Next" } }
                }
            }
        }
    }
}

#[component]
pub fn ModerationCase(id: i64, view: Option<String>, page: Option<u32>) -> Element {
    let auth = use_auth();
    rsx! { AdminShell {
        Link { to:queue_route(view,page.unwrap_or_default()), "← Back to queue" }
        if auth.current_user_id.is_some() { for actor in auth.current_user_id {
            CasePanel { key:"{actor}-{id}",actor,id }
        } } else { p {"Sign in with a moderator account to continue."} }
    } }
}

#[component]
fn CasePanel(actor: Uuid, id: i64) -> Element {
    let mut page = use_signal(|| 0_u32);
    let mut result = use_resource(move || moderation::case_detail(Some(actor), id, page()));
    let mut reason = use_signal(String::new);
    let mut hide = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut message = use_signal(String::new);
    let read = result.read();
    let detail = match &*read {
        None => return rsx! {p {role:"status","Loading case…"}},
        Some(Err(_)) => {
            return rsx! {p {role:"alert","Case unavailable or access denied."} button {class:"btn",onclick:move |_|result.restart(),"Retry"}}
        }
        Some(Ok(detail)) => detail,
    };
    let info = &detail.info;
    let available = matches!(
        info.status.as_str(),
        "queued" | "flagged" | "unflagged" | "uncertain"
    );
    let reviewing = info.status == "reviewing";
    let mine = reviewing && info.claimant == Some(actor);
    let can_hide = info.body.is_some() && !info.hidden;
    let revision = info.revision;
    let body = info.body.clone();
    let mut act = move |action| {
        if busy() {
            return;
        }
        busy.set(true);
        message.set(String::new());
        let text = reason();
        let seen = body.clone();
        spawn(async move {
            match moderation::review(Some(actor), id, revision, action, text, seen).await {
                Ok(()) => {
                    reason.set(String::new());
                    hide.set(false);
                    message.set("Case updated.".into());
                }
                Err(_) => message.set(
                    "Action could not be saved. Check the latest case details before retrying."
                        .into(),
                ),
            }
            result.restart();
            busy.set(false);
        });
    };
    let mut claim = act.clone();
    let mut release = act.clone();
    let mut dismiss = act.clone();
    rsx! {
        h1 {"Case #{id}"}
        h2 {"{info.title}"}
        p { "{info.status}" if let Some(name)=&info.reviewer {" · Assigned to {name}"} }
        if !message().is_empty() {p {role:"status","{message}"}}
        article { class:"neumo-card admin-card",
            if info.parent_id.is_some() { h3 {"Parent comment"} p { {info.parent.as_deref().unwrap_or("Comment deleted")} } }
            h3 {"Current comment"}
            if info.hidden {p {"Hidden from the discussion"}}
            p { {info.body.as_deref().unwrap_or("Comment deleted")} }
        }
        EvidenceList { detail:detail.clone(),page:page() }
        nav {class:"admin-filters", aria_label:"Report pages",
            if page()>0 { button {class:"btn",onclick:move |_|page-=1,"Previous reports"} }
            if detail.evidence.len()>PAGE_SIZE { button {class:"btn",onclick:move |_|page+=1,"More reports"} }
        }
        if available {
            button {class:"btn btn-primary",disabled:busy(),onclick:move |_|claim(Action::Claim),"Claim case"}
        }
        if reviewing {
            div {class:"neumo-card admin-card",
                label { "Reason for action" textarea { maxlength:"2000",value:reason(),oninput:move |e|reason.set(e.value()) } }
                if mine && can_hide {label { input {r#type:"checkbox",checked:hide(),onchange:move |e| hide.set(e.checked())} " Hide this comment from the discussion if upheld" }}
                div {class:"admin-filters",
                    if mine {
                        button {class:"btn btn-primary",disabled:busy()||reason.read().trim().is_empty(),
                            onclick:move |_|act(if hide() && can_hide {Action::UpholdAndHide} else {Action::Uphold}),
                            if hide() && can_hide {"Uphold and hide comment"} else {"Uphold report"}
                        }
                        button {class:"btn",disabled:busy()||reason.read().trim().is_empty(),onclick:move |_|dismiss(Action::Dismiss),"Dismiss report"}
                    }
                    button {class:"btn",disabled:busy()||reason.read().trim().is_empty(),onclick:move |_|release(Action::Release),"Release claim"}
                }
            }
        }
        h2 {"Case history"}
        if detail.history.is_empty() {p {"No review actions yet."}}
        for entry in &detail.history {
            article { class:"admin-history",strong {"{entry.actor} · {entry.action}"} small {"{stamp(entry.created_at)}"} p {"{entry.rationale}"} }
        }
    }
}

#[component]
fn EvidenceList(detail: Detail, page: u32) -> Element {
    let evidence = &detail.evidence;
    rsx! {
        h2 {"Reported evidence"}
        for (i,report) in evidence.iter().take(PAGE_SIZE).enumerate() {
            if i==0 || (report.reported_body.as_str(),report.recipe_version)!=(evidence[i-1].reported_body.as_str(),evidence[i-1].recipe_version) {
                h3 {"Reported text · Recipe version {report.recipe_version}"}
                small { {report.author.as_deref().unwrap_or("Former member")} }
                blockquote { "{report.reported_body}" }
            }
            p { strong {"Report {page as usize * PAGE_SIZE+i+1} · "} "{report.reason}" }
            small {"{stamp(report.reported_at)}"}
        }
    }
}
