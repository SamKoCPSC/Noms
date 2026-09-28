use crate::components::base::{PageIcon, PageIconKind};
use chrono::{NaiveDate, TimeZone, Utc};
use dioxus::prelude::*;

use crate::api::search::{
    autocomplete_entities, search_all, search_collections, search_recipes, search_users,
};
use crate::components::base::{EmptyState, LoadingSpinner, RecipeCard};
use crate::components::AuthRequired;
use crate::search_url::{
    parse_search_query, parsed_from_request, ActiveAdvancedCriteria, ActiveSearchRequest,
    ParsedSearchQuery, SearchRouteQuery,
};
use crate::types::{
    AdvancedCollectionSearchCriteria, AdvancedRecipeSearchCriteria, AdvancedTextField,
    AdvancedUserSearchCriteria, CollectionSearchResponse, CollectionSearchSort, IntegerRange,
    RecipeSearchResponse, RecipeSearchSort, RecipeSearchVisibility, SearchResponse, SearchScope,
    SearchSuggestion, SearchSuggestionKind, TextCriterion, TextMatchMode, TimestampRange,
    UserSearchResponse, UserSearchSort,
};
use crate::Route;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchInteraction {
    Typing,
    Submit,
    Suggestion,
    Filter,
    Sort,
    Pagination,
    Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HistoryAction {
    Replace,
    Push,
}

fn history_action(interaction: SearchInteraction) -> HistoryAction {
    match interaction {
        SearchInteraction::Typing => HistoryAction::Replace,
        SearchInteraction::Submit
        | SearchInteraction::Suggestion
        | SearchInteraction::Filter
        | SearchInteraction::Sort
        | SearchInteraction::Pagination
        | SearchInteraction::Scope => HistoryAction::Push,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuggestionDirection {
    Up,
    Down,
}

fn next_suggestion(
    current: Option<usize>,
    count: usize,
    direction: SuggestionDirection,
) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match (current, direction) {
        (None, SuggestionDirection::Down) => 0,
        (None, SuggestionDirection::Up) => count - 1,
        (Some(index), SuggestionDirection::Down) => (index + 1) % count,
        (Some(0), SuggestionDirection::Up) => count - 1,
        (Some(index), SuggestionDirection::Up) => index.saturating_sub(1),
    })
}

#[derive(Clone)]
enum LoadedSearch {
    All(SearchResponse),
    Recipes(RecipeSearchResponse),
    Collections(CollectionSearchResponse),
    Users(UserSearchResponse),
}

pub fn landing_search_route(text: &str) -> Option<Route> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    parsed_from_request(ActiveSearchRequest::All(crate::types::AllSearchRequest {
        text: text.to_string(),
        ..crate::types::AllSearchRequest::default()
    }))
    .ok()
    .map(|parsed| Route::Search {
        query: SearchRouteQuery::new(parsed.canonical_query),
    })
}

fn navigate_route(target: Route, interaction: SearchInteraction) {
    navigate_with_action(target, history_action(interaction));
}

fn navigate_with_action(target: Route, action: HistoryAction) {
    match action {
        HistoryAction::Replace => {
            navigator().replace(target);
        }
        HistoryAction::Push => {
            navigator().push(target);
        }
    }
}

fn search_transition(
    request: ActiveSearchRequest,
    interaction: SearchInteraction,
) -> Result<(Route, HistoryAction), crate::search_url::SearchUrlError> {
    let parsed = parsed_from_request(request)?;
    Ok((
        Route::Search {
            query: SearchRouteQuery::new(parsed.canonical_query),
        },
        history_action(interaction),
    ))
}

fn restored_search_state(query: &SearchRouteQuery) -> Result<(String, SearchScope), String> {
    parse_search_query(query.as_str())
        .map(|parsed| (parsed.request.text().to_string(), parsed.request.scope()))
        .map_err(|error| error.to_string())
}

fn canonical_route_if_needed(raw: &str, parsed: &ParsedSearchQuery) -> Option<Route> {
    (raw.trim_start_matches('?') != parsed.canonical_query).then(|| Route::Search {
        query: SearchRouteQuery::new(parsed.canonical_query.clone()),
    })
}

fn update_text(request: &mut ActiveSearchRequest, text: String) {
    let text = text.trim().to_string();
    match request {
        ActiveSearchRequest::All(request) => request.text = text,
        ActiveSearchRequest::Recipes(request) => {
            request.text = (!text.is_empty()).then_some(text);
            request.page = 1;
        }
        ActiveSearchRequest::Collections(request) => {
            request.text = (!text.is_empty()).then_some(text);
            request.page = 1;
        }
        ActiveSearchRequest::Users(request) => {
            request.text = (!text.is_empty()).then_some(text);
            request.page = 1;
        }
    }
}

fn update_page(request: &mut ActiveSearchRequest, page: u32) {
    match request {
        ActiveSearchRequest::All(_) => {}
        ActiveSearchRequest::Recipes(request) => request.page = page,
        ActiveSearchRequest::Collections(request) => request.page = page,
        ActiveSearchRequest::Users(request) => request.page = page,
    }
}

async fn load_search(request: ActiveSearchRequest) -> Result<LoadedSearch, String> {
    match request {
        ActiveSearchRequest::All(request) => search_all(request)
            .await
            .map(LoadedSearch::All)
            .map_err(|error| error.to_string()),
        ActiveSearchRequest::Recipes(request) => search_recipes(*request)
            .await
            .map(LoadedSearch::Recipes)
            .map_err(|error| error.to_string()),
        ActiveSearchRequest::Collections(request) => search_collections(request)
            .await
            .map(LoadedSearch::Collections)
            .map_err(|error| error.to_string()),
        ActiveSearchRequest::Users(request) => search_users(request)
            .await
            .map(LoadedSearch::Users)
            .map_err(|error| error.to_string()),
    }
}

fn suggestion_route(suggestion: &SearchSuggestion) -> Option<Route> {
    match suggestion.kind {
        SearchSuggestionKind::Recipe => Some(Route::RecipeDetail {
            id: suggestion.id.to_string(),
        }),
        SearchSuggestionKind::Collection => Some(Route::CollectionDetail {
            id: suggestion.id.to_string(),
        }),
        SearchSuggestionKind::User => suggestion
            .secondary_text
            .as_deref()
            .and_then(|username| username.strip_prefix('@'))
            .map(|username| Route::UserProfile {
                username: username.to_string(),
            }),
    }
}

#[component]
pub fn Search(query: SearchRouteQuery) -> Element {
    let parsed = parse_search_query(query.as_str());
    let initial_text = parsed
        .as_ref()
        .map(|parsed| parsed.request.text().to_string())
        .unwrap_or_default();
    let initial_scope = parsed
        .as_ref()
        .map(|parsed| parsed.request.scope())
        .unwrap_or(SearchScope::All);
    let mut input_text = use_signal(|| initial_text.clone());
    let mut active_scope = use_signal(|| initial_scope);
    let mut active_suggestion = use_signal(|| Option::<usize>::None);
    let mut suggestions_dismissed = use_signal(|| false);
    let mut search_focused = use_signal(|| false);
    let mut navigation_error = use_signal(|| None::<crate::search_url::SearchUrlError>);
    let mut navigate_search =
        move |request, interaction| match search_transition(request, interaction) {
            Ok((target, action)) => {
                navigation_error.set(None);
                navigate_with_action(target, action);
            }
            Err(error) => navigation_error.set(Some(error)),
        };

    use_effect(use_reactive((&query,), move |(query,)| {
        if let Ok(parsed) = parse_search_query(query.as_str()) {
            if let Some(route) = canonical_route_if_needed(query.as_str(), &parsed) {
                navigator().replace(route);
                return;
            }
            if let Ok((text, scope)) = restored_search_state(&query) {
                input_text.set(text);
                active_scope.set(scope);
                navigation_error.set(None);
            }
            active_suggestion.set(None);
            suggestions_dismissed.set(false);
        }
    }));

    let results = use_resource(use_reactive((&query,), move |(query,)| async move {
        match parse_search_query(query.as_str()) {
            Ok(parsed) => Some(load_search(parsed.request).await),
            Err(_) => None,
        }
    }));

    let suggestions = use_resource(move || {
        let text = input_text();
        let scope = active_scope();
        let focused = search_focused();
        async move {
            if !focused || text.trim().chars().count() < crate::search::MIN_AUTOCOMPLETE_CHARS {
                return Ok(Vec::new());
            }
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(180).await;
            autocomplete_entities(scope, text)
                .await
                .map_err(|error| error.to_string())
        }
    });

    let parsed_for_view = parsed.clone();
    let suggestion_values = suggestions
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let suggestions_open = search_focused()
        && !suggestions_dismissed()
        && !suggestion_values.is_empty()
        && input_text().chars().count() >= 2;
    let input_query = query.clone();
    let submit_query = query.clone();
    let scope_query = query.clone();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/search.css") }
        AuthRequired {
            div { class: "container search-page",
                header { class: "search-hero",
                    div {
                        h1 { class: "brand-title", PageIcon { kind: PageIconKind::Search } span { "Search Noms" } }
                        p { class: "search-subtitle",
                            "Find recipes, collections and cooks in one place."
                        }
                    }
                }

                if let Err(error) = &parsed_for_view {
                    SearchValidationState { message: error.to_string() }
                } else if let Ok(parsed) = parsed_for_view {
                    div { class: "search-workspace",
                        SearchInput {
                            value: input_text(),
                            scope: parsed.request.scope(),
                            suggestions: suggestion_values.clone(),
                            active_suggestion: active_suggestion(),
                            suggestions_open,
                            oninput: move |value: String| {
                                input_text.set(value.clone());
                                active_suggestion.set(None);
                                suggestions_dismissed.set(false);
                                if let Ok(parsed) = parse_search_query(input_query.as_str()) {
                                    let mut request = parsed.request;
                                    update_text(&mut request, value);
                                    navigate_search(request, SearchInteraction::Typing);
                                }
                            },
                            onsubmit: move |_| {
                                if let Ok(parsed) = parse_search_query(submit_query.as_str()) {
                                    let mut request = parsed.request;
                                    update_text(&mut request, input_text());
                                    navigate_search(request, SearchInteraction::Submit);
                                }
                            },
                            onmove: move |direction| {
                                suggestions_dismissed.set(false);
                                active_suggestion.set(next_suggestion(
                                    active_suggestion(),
                                    suggestion_values.len(),
                                    direction,
                                ));
                            },
                            onselect: move |suggestion: SearchSuggestion| {
                                active_suggestion.set(None);
                                suggestions_dismissed.set(true);
                                if let Some(route) = suggestion_route(&suggestion) {
                                    navigate_route(route, SearchInteraction::Suggestion);
                                }
                            },
                            ondismiss: move |_| {
                                active_suggestion.set(None);
                                suggestions_dismissed.set(true);
                            },
                            onfocus: move |_| search_focused.set(true),
                            onblur: move |_| search_focused.set(false),
                        }

                        if let Some(message) = navigation_error() {
                            div { class: "search-state-card search-state-error", role: "alert",
                                p { "{message}" }
                            }
                        }

                        ScopeTabs {
                            active: parsed.request.scope(),
                            onchange: move |scope| {
                                if let Ok(parsed) = parse_search_query(scope_query.as_str()) {
                                    if let Ok(switched) = parsed.switch_scope(scope) {
                                        navigate_search(switched.request, SearchInteraction::Scope);
                                    }
                                }
                            },
                        }

                        if !matches!(parsed.request, ActiveSearchRequest::All(_)) {
                            SearchControls {
                                parsed: parsed.clone(),
                                onchange: move |request| navigate_search(request, SearchInteraction::Filter),
                                ontagsinput: move |request| navigate_search(request, SearchInteraction::Typing),
                                onsort: move |request| navigate_search(request, SearchInteraction::Sort),
                            }
                            AdvancedSearchPanel {
                                request: parsed.request.clone(),
                                onapply: {
                                    let base_request = parsed.request.clone();
                                    move |advanced| {
                                        let mut request = base_request.clone();
                                        match (&mut request, advanced) {
                                            (ActiveSearchRequest::Recipes(request), ActiveAdvancedCriteria::Recipes(value)) => request.advanced = Some(value),
                                            (ActiveSearchRequest::Collections(request), ActiveAdvancedCriteria::Collections(value)) => request.advanced = Some(value),
                                            (ActiveSearchRequest::Users(request), ActiveAdvancedCriteria::Users(value)) => request.advanced = Some(value),
                                            _ => return,
                                        }
                                        update_page(&mut request, 1);
                                        navigate_search(request, SearchInteraction::Filter);
                                    }
                                },
                                onclear: {
                                    let base_request = parsed.request.clone();
                                    move |_| {
                                        let mut request = base_request.clone();
                                        match &mut request {
                                            ActiveSearchRequest::Recipes(request) => request.advanced = None,
                                            ActiveSearchRequest::Collections(request) => request.advanced = None,
                                            ActiveSearchRequest::Users(request) => request.advanced = None,
                                            ActiveSearchRequest::All(_) => {}
                                        }
                                        update_page(&mut request, 1);
                                        navigate_search(request, SearchInteraction::Filter);
                                    }
                                },
                            }
                        }

                        SearchResults {
                            resource: results,
                            request: parsed.request.clone(),
                            onpage: {
                                let base_request = parsed.request.clone();
                                move |page| {
                                    let mut request = base_request.clone();
                                    update_page(&mut request, page);
                                    navigate_search(request, SearchInteraction::Pagination);
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct SearchInputProps {
    value: String,
    scope: SearchScope,
    suggestions: Vec<SearchSuggestion>,
    active_suggestion: Option<usize>,
    suggestions_open: bool,
    oninput: EventHandler<String>,
    onsubmit: EventHandler<()>,
    onmove: EventHandler<SuggestionDirection>,
    onselect: EventHandler<SearchSuggestion>,
    ondismiss: EventHandler<()>,
    onfocus: EventHandler<()>,
    onblur: EventHandler<()>,
}

#[component]
fn SearchInput(props: SearchInputProps) -> Element {
    let scope_label = match props.scope {
        SearchScope::All => "everything",
        SearchScope::Recipes => "recipes",
        SearchScope::Collections => "collections",
        SearchScope::Users => "cooks",
    };
    rsx! {
        form {
            class: "search-box",
            role: "search",
            onsubmit: move |event| {
                event.prevent_default();
                props.onsubmit.call(());
            },
            div { class: "search-input-row",
                svg { class: "search-icon", view_box: "0 0 24 24", width: "22", height: "22", fill: "none", stroke: "currentColor", stroke_width: "1.8", "aria-hidden": "true",
                    circle { cx: "10.5", cy: "10.5", r: "6.5" }
                    path { d: "m16 16 4 4", stroke_linecap: "round" }
                }
                input {
                    id: "search-query",
                    class: "search-query-input",
                    r#type: "search",
                    value: props.value,
                    placeholder: "Search {scope_label}…",
                    autocomplete: "off",
                    role: "combobox",
                    aria_label: "Search {scope_label}",
                    aria_expanded: props.suggestions_open,
                    aria_controls: "search-suggestions",
                    aria_activedescendant: props.active_suggestion.map(|index| format!("search-suggestion-{index}")),
                    onfocus: move |_| props.onfocus.call(()),
                    onblur: move |_| props.onblur.call(()),
                    oninput: move |event| props.oninput.call(event.value()),
                    onkeydown: move |event| match event.key() {
                        Key::ArrowDown => {
                            event.prevent_default();
                            props.onmove.call(SuggestionDirection::Down);
                        }
                        Key::ArrowUp => {
                            event.prevent_default();
                            props.onmove.call(SuggestionDirection::Up);
                        }
                        Key::Enter if props.active_suggestion.is_some() => {
                            event.prevent_default();
                            if let Some(suggestion) = props
                                .active_suggestion
                                .and_then(|index| props.suggestions.get(index))
                                .cloned()
                            {
                                props.onselect.call(suggestion);
                            }
                        }
                        Key::Escape => props.ondismiss.call(()),
                        _ => {}
                    },
                }
                button { class: "btn btn-primary search-submit", r#type: "submit", "Search" }
            }
            if props.suggestions_open {
                ul { id: "search-suggestions", class: "search-suggestions", role: "listbox",
                    for (index, suggestion) in props.suggestions.iter().enumerate() {
                        li {
                            id: "search-suggestion-{index}",
                            role: "option",
                            aria_selected: props.active_suggestion == Some(index),
                            class: if props.active_suggestion == Some(index) { "search-suggestion is-active" } else { "search-suggestion" },
                            onmousedown: move |event| event.prevent_default(),
                            onclick: {
                                let suggestion = suggestion.clone();
                                move |_| props.onselect.call(suggestion.clone())
                            },
                            span { class: "suggestion-kind", {suggestion_icon(suggestion.kind)} }
                            span { class: "suggestion-copy",
                                strong { "{suggestion.label}" }
                                if let Some(secondary) = &suggestion.secondary_text {
                                    small { "{secondary}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn suggestion_icon(kind: SearchSuggestionKind) -> &'static str {
    match kind {
        SearchSuggestionKind::Recipe => "🍲",
        SearchSuggestionKind::Collection => "📚",
        SearchSuggestionKind::User => "👤",
    }
}

#[derive(Props, Clone, PartialEq)]
struct ScopeTabsProps {
    active: SearchScope,
    onchange: EventHandler<SearchScope>,
}

#[component]
fn ScopeTabs(props: ScopeTabsProps) -> Element {
    rsx! {
        nav { class: "search-scope-tabs", aria_label: "Search type",
            for (scope, label) in [
                (SearchScope::All, "All"),
                (SearchScope::Recipes, "Recipes"),
                (SearchScope::Collections, "Collections"),
                (SearchScope::Users, "Cooks"),
            ] {
                button {
                    class: if props.active == scope { "search-scope-tab is-active" } else { "search-scope-tab" },
                    aria_current: (props.active == scope).then_some("page"),
                    onclick: move |_| props.onchange.call(scope),
                    "{label}"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct SearchControlsProps {
    parsed: ParsedSearchQuery,
    onchange: EventHandler<ActiveSearchRequest>,
    ontagsinput: EventHandler<ActiveSearchRequest>,
    onsort: EventHandler<ActiveSearchRequest>,
}

#[component]
fn SearchControls(props: SearchControlsProps) -> Element {
    let request = props.parsed.request.clone();
    let mut filters_open = use_signal(|| false);
    let initial_required_tags = match &request {
        ActiveSearchRequest::Recipes(recipe) => recipe.tags_all.join(", "),
        _ => String::new(),
    };
    let initial_required_tags_for_signal = initial_required_tags.clone();
    let mut required_tags_draft = use_signal(|| initial_required_tags_for_signal);
    use_effect(use_reactive(
        (&initial_required_tags,),
        move |(required_tags_from_query,)| {
            if comma_values(&required_tags_draft()) != comma_values(&required_tags_from_query) {
                required_tags_draft.set(required_tags_from_query);
            }
        },
    ));
    rsx! {
        section { class: "search-controls", aria_label: "Search filters",
            if matches!(&request, ActiveSearchRequest::Recipes(_)) {
                button { class: "btn btn-secondary search-filter-toggle", r#type: "button",
                    aria_expanded: filters_open(), aria_controls: "search-filter-fields",
                    onclick: move |_| filters_open.toggle(),
                    "Filters"
                }
            }
            label { class: "search-control search-sort-control",
                span { "Sort by" }
                select {
                    value: sort_value(&request),
                    onchange: {
                        let request = request.clone();
                        move |event| {
                            let mut request = request.clone();
                            apply_sort(&mut request, &event.value());
                            update_page(&mut request, 1);
                            props.onsort.call(request);
                        }
                    },
                    {sort_options(&request)}
                }
            }
            if has_filters(&request) {
                div { class: "active-filter-row",
                    for chip in filter_chips(&request) {
                        button { class: "filter-chip", r#type: "button", aria_label: "Remove {chip}",
                            onclick: {
                                let request = request.clone();
                                let chip = chip.clone();
                                move |_| props.onchange.call(remove_filter(request.clone(), &chip))
                            },
                            "{chip}" span { aria_hidden: "true", " ×" }
                        }
                    }
                    button {
                        class: "clear-filters",
                        onclick: {
                            let request = request.clone();
                            move |_| props.onchange.call(clear_filters(request.clone()))
                        },
                        "Clear filters"
                    }
                }
            }
            if let ActiveSearchRequest::Recipes(recipe) = &request {
              div { id: "search-filter-fields", class: "search-filter-fields", hidden: !filters_open(),
                label { class: "search-control",
                    span { "Visibility" }
                    select {
                        value: visibility_value(recipe.visibility),
                        onchange: {
                            let request = request.clone();
                            move |event| {
                                let mut request = request.clone();
                                if let ActiveSearchRequest::Recipes(recipe) = &mut request {
                                    recipe.visibility = match event.value().as_str() {
                                        "mine" => RecipeSearchVisibility::MineOnly,
                                        "public" => RecipeSearchVisibility::PublicOnly,
                                        _ => RecipeSearchVisibility::AllVisible,
                                    };
                                    recipe.page = 1;
                                }
                                props.onchange.call(request);
                            }
                        },
                        option { value: "all", "All visible" }
                        option { value: "mine", "Mine only" }
                        option { value: "public", "Public only" }
                    }
                }
                SelectRange {
                    label: "Total time",
                    value: recipe.total_time.and_then(time_value).unwrap_or_default(),
                    options: vec![("", "Any time"), ("0-30", "Under 30 min"), ("30-60", "30–60 min"), ("60-120", "1–2 hours"), ("120+", "2+ hours")],
                    onchange: {
                        let request = request.clone();
                        move |value: String| {
                            let mut request = request.clone();
                            if let ActiveSearchRequest::Recipes(recipe) = &mut request {
                                recipe.total_time = range_from_time(&value);
                                recipe.page = 1;
                            }
                            props.onchange.call(request);
                        }
                    },
                }
                SelectRange {
                    label: "Servings",
                    value: recipe.servings.and_then(servings_value).unwrap_or_default(),
                    options: vec![("", "Any servings"), ("1-2", "1–2"), ("3-4", "3–4"), ("5-8", "5–8"), ("9+", "9+")],
                    onchange: {
                        let request = request.clone();
                        move |value: String| {
                            let mut request = request.clone();
                            if let ActiveSearchRequest::Recipes(recipe) = &mut request {
                                recipe.servings = range_from_servings(&value);
                                recipe.page = 1;
                            }
                            props.onchange.call(request);
                        }
                    },
                }
                label { class: "search-control search-control-wide",
                    span { "Required tags" }
                    input {
                        value: required_tags_draft(),
                        placeholder: "vegan, weeknight",
                        oninput: {
                            let request = request.clone();
                            move |event| {
                                let value = event.value();
                                required_tags_draft.set(value.clone());
                                let mut request = request.clone();
                                update_required_tags(&mut request, comma_values(&value));
                                props.ontagsinput.call(request);
                            }
                        },
                    }
                }
              }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct SelectRangeProps {
    label: &'static str,
    value: String,
    options: Vec<(&'static str, &'static str)>,
    onchange: EventHandler<String>,
}

#[component]
fn SelectRange(props: SelectRangeProps) -> Element {
    rsx! {
        label { class: "search-control",
            span { "{props.label}" }
            select { value: props.value, onchange: move |event| props.onchange.call(event.value()),
                for (value, label) in props.options { option { value, "{label}" } }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AdvancedSearchPanelProps {
    request: ActiveSearchRequest,
    onapply: EventHandler<ActiveAdvancedCriteria>,
    onclear: EventHandler<()>,
}

#[component]
fn AdvancedSearchPanel(props: AdvancedSearchPanelProps) -> Element {
    rsx! {
        details { class: "advanced-search", open: props.request.advanced().is_some(),
            summary { "Advanced search" }
            form {
                class: "advanced-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    if let Some(criteria) = advanced_from_form(&props.request, &event) {
                        props.onapply.call(criteria);
                    }
                },
                p { class: "advanced-help",
                    "Choose a fixed match mode for each field. Separate exclusions with commas."
                }
                match &props.request {
                    ActiveSearchRequest::Recipes(request) => rsx! {
                        AdvancedTextInput { prefix: "title", label: "Title", field: request.advanced.as_ref().map(|value| value.title.clone()) }
                        AdvancedTextInput { prefix: "description", label: "Description", field: request.advanced.as_ref().map(|value| value.description.clone()) }
                        AdvancedTextInput { prefix: "ingredients", label: "Ingredients", field: request.advanced.as_ref().map(|value| value.ingredients.clone()) }
                        AdvancedTextInput { prefix: "instructions", label: "Instructions", field: request.advanced.as_ref().map(|value| value.instructions.clone()) }
                        label { class: "advanced-simple-field", span { "Any tags" } input { name: "tags_any", value: request.advanced.as_ref().map(|value| value.tags_any.join(", ")).unwrap_or_default() } }
                        label { class: "advanced-simple-field", span { "Excluded tags" } input { name: "tags_excluded", value: request.advanced.as_ref().map(|value| value.tags_excluded.join(", ")).unwrap_or_default() } }
                        DateRangeInputs { prefix: "created", label: "Created", range: request.advanced.as_ref().and_then(|value| value.created_at) }
                        DateRangeInputs { prefix: "updated", label: "Updated", range: request.advanced.as_ref().and_then(|value| value.updated_at) }
                    },
                    ActiveSearchRequest::Collections(request) => rsx! {
                        AdvancedTextInput { prefix: "name", label: "Name", field: request.advanced.as_ref().map(|value| value.name.clone()) }
                        AdvancedTextInput { prefix: "description", label: "Description", field: request.advanced.as_ref().map(|value| value.description.clone()) }
                        DateRangeInputs { prefix: "updated", label: "Updated", range: request.advanced.as_ref().and_then(|value| value.updated_at) }
                    },
                    ActiveSearchRequest::Users(request) => rsx! {
                        AdvancedTextInput { prefix: "username", label: "Username", field: request.advanced.as_ref().map(|value| value.username.clone()) }
                        AdvancedTextInput { prefix: "display_name", label: "Display name", field: request.advanced.as_ref().map(|value| value.display_name.clone()) }
                        AdvancedTextInput { prefix: "bio", label: "Bio", field: request.advanced.as_ref().map(|value| value.bio.clone()) }
                    },
                    ActiveSearchRequest::All(_) => rsx! {},
                }
                div { class: "advanced-actions",
                    button { class: "btn btn-primary", r#type: "submit", "Apply advanced search" }
                    button { class: "btn btn-ghost", r#type: "button", onclick: move |_| props.onclear.call(()), "Clear advanced" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AdvancedTextInputProps {
    prefix: &'static str,
    label: &'static str,
    field: Option<AdvancedTextField>,
}

#[component]
fn AdvancedTextInput(props: AdvancedTextInputProps) -> Element {
    let include = props
        .field
        .as_ref()
        .and_then(|field| field.include.as_ref())
        .map(|criterion| criterion.text.clone())
        .unwrap_or_default();
    let mode = props
        .field
        .as_ref()
        .and_then(|field| field.include.as_ref())
        .map(|criterion| mode_value(criterion.mode))
        .unwrap_or("all");
    let excluded = props
        .field
        .as_ref()
        .map(|field| {
            field
                .exclude
                .iter()
                .map(|value| value.text.clone())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    rsx! {
        fieldset { class: "advanced-field",
            legend { "{props.label}" }
            input { name: "{props.prefix}_include", aria_label: "{props.label} contains", value: include, placeholder: "Words or phrase" }
            select { name: "{props.prefix}_mode", aria_label: "{props.label} match mode", value: mode,
                option { value: "all", "All terms" }
                option { value: "any", "Any term" }
                option { value: "exact", "Exact phrase" }
            }
            input { name: "{props.prefix}_exclude", aria_label: "{props.label} excludes", value: excluded, placeholder: "Exclude (comma-separated)" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DateRangeInputsProps {
    prefix: &'static str,
    label: &'static str,
    range: Option<TimestampRange>,
}

#[component]
fn DateRangeInputs(props: DateRangeInputsProps) -> Element {
    rsx! {
        fieldset { class: "advanced-field advanced-date-field",
            legend { "{props.label}" }
            label { span { "From" } input { r#type: "date", name: "{props.prefix}_from", value: props.range.and_then(|range| range.lower_inclusive).map(|date| date.format("%Y-%m-%d").to_string()).unwrap_or_default() } }
            label { span { "Before" } input { r#type: "date", name: "{props.prefix}_before", value: props.range.and_then(|range| range.upper_exclusive).map(|date| date.format("%Y-%m-%d").to_string()).unwrap_or_default() } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct SearchResultsProps {
    resource: Resource<Option<Result<LoadedSearch, String>>>,
    request: ActiveSearchRequest,
    onpage: EventHandler<u32>,
}

#[component]
fn SearchResults(props: SearchResultsProps) -> Element {
    let pending = *props.resource.state().read() == UseResourceState::Pending;
    // A previous scope's cards must not appear under the new scope while loading.
    let result = if pending {
        None
    } else {
        props.resource.read().clone()
    };
    let mut resource = props.resource;
    rsx! {
        section { class: if matches!(props.request, ActiveSearchRequest::All(_)) { "search-results search-results--all" } else { "search-results" }, aria_live: "polite",
            match result {
                None => rsx! {
                    div { class: "search-pending",
                        div { class: "search-loading", role: "status", LoadingSpinner {} span { "Searching…" } }
                        div { class: "search-card-grid", aria_hidden: "true",
                            for _ in 0..3 { div { class: "search-skeleton-card", div { class: "search-skeleton-photo" } div { class: "search-skeleton-line" } div { class: "search-skeleton-line search-skeleton-line--short" } } }
                        }
                    }
                },
                Some(None) => rsx! {},
                Some(Some(Err(_))) => rsx! { SearchErrorState { onretry: move |_| resource.restart() } },
                Some(Some(Ok(loaded))) => render_loaded_results(loaded, &props.request, props.onpage),
            }
        }
    }
}

fn render_loaded_results(
    loaded: LoadedSearch,
    request: &ActiveSearchRequest,
    onpage: EventHandler<u32>,
) -> Element {
    match loaded {
        LoadedSearch::All(response) => rsx! {
            GroupedSection { title: "Recipes", target: scoped_search_route(request, SearchScope::Recipes), count: response.recipe_total_count, body: rsx! { div { class: "recipe-grid search-recipe-grid", for result in response.recipes { RecipeCard { key: "{result.recipe.id}", recipe: result.recipe, framed: true } } } } }
            GroupedSection { title: "Collections", target: scoped_search_route(request, SearchScope::Collections), count: response.collection_total_count, body: rsx! { div { class: "search-card-grid", for result in response.collections { CollectionResultCard { key: "{result.collection.id}", result } } } } }
            GroupedSection { title: "Cooks", target: scoped_search_route(request, SearchScope::Users), count: response.user_total_count, body: rsx! { div { class: "search-card-grid", for result in response.users { UserResultCard { key: "{result.user.id}", result } } } } }
            if response.recipe_total_count + response.collection_total_count + response.user_total_count == 0 { NoResults { blank: request.text().is_empty() } }
        },
        LoadedSearch::Recipes(response) => {
            let page = request.page();
            let page_size = match request {
                ActiveSearchRequest::Recipes(value) => value.page_size,
                _ => 20,
            };
            rsx! {
                RecipeFacetSummary { facets: response.facets.clone() }
                div { class: "results-heading", h2 { "Recipes" } span { "{response.total_count} found" } }
                if response.results.is_empty() { NoResults { blank: request.text().is_empty() } }
                else { div { class: "recipe-grid search-recipe-grid", for result in response.results { RecipeCard { key: "{result.recipe.id}", recipe: result.recipe, framed: true } } } }
                Pagination { page, page_size, total: response.total_count, onchange: onpage }
            }
        }
        LoadedSearch::Collections(response) => {
            let page = request.page();
            let page_size = match request {
                ActiveSearchRequest::Collections(value) => value.page_size,
                _ => 20,
            };
            rsx! {
                div { class: "results-heading", h2 { "Collections" } span { "{response.total_count} found" } }
                if response.results.is_empty() { NoResults { blank: request.text().is_empty() } }
                else { div { class: "search-card-grid", for result in response.results { CollectionResultCard { key: "{result.collection.id}", result } } } }
                Pagination { page, page_size, total: response.total_count, onchange: onpage }
            }
        }
        LoadedSearch::Users(response) => {
            let page = request.page();
            let page_size = match request {
                ActiveSearchRequest::Users(value) => value.page_size,
                _ => 20,
            };
            rsx! {
                div { class: "results-heading", h2 { "Cooks" } span { "{response.total_count} found" } }
                if response.results.is_empty() { NoResults { blank: request.text().is_empty() } }
                else { div { class: "search-card-grid", for result in response.results { UserResultCard { key: "{result.user.id}", result } } } }
                Pagination { page, page_size, total: response.total_count, onchange: onpage }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct GroupedSectionProps {
    title: &'static str,
    target: Option<Route>,
    count: i64,
    body: Element,
}

#[component]
fn GroupedSection(props: GroupedSectionProps) -> Element {
    if props.count == 0 {
        return rsx! {};
    }
    rsx! { section { class: "grouped-section",
        div { class: "results-heading",
            div { class: "results-heading__title", h2 { "{props.title}" } span { "{props.count} found" } }
            if let Some(target) = props.target { Link { to: target, class: "search-view-all", "View all {props.title.to_lowercase()}" span { aria_hidden: "true", " →" } } }
        }
        {props.body}
    } }
}

#[component]
fn CollectionResultCard(result: crate::types::CollectionSearchResult) -> Element {
    let parent_path = result.breadcrumb.join(" / ");
    rsx! {
        Link { to: Route::CollectionDetail { id: result.collection.id.to_string() }, class: "entity-result-card search-collection-card",
            div { class: "search-collection-cover", aria_hidden: "true",
                svg { view_box: "0 0 80 64", width: "80", height: "64", fill: "none", stroke: "currentColor", stroke_width: "2",
                    path { d: "M10 17 Q10 12 15 12 H31 L39 20 H65 Q70 20 70 25 V51 Q70 56 65 56 H15 Q10 56 10 51 Z", fill: "var(--neumo-surface)" }
                    path { d: "M20 34 H59 M20 42 H47", stroke_linecap: "round" }
                }
            }
            div { class: "entity-result-copy", small { "Collection" } h3 { "{result.collection.name}" } if let Some(description) = result.collection.description { p { "{description}" } } if !parent_path.is_empty() { p { class: "entity-path", "{parent_path}" } } span { class: "entity-result-action", "Open collection →" } }
        }
    }
}

#[component]
fn UserResultCard(result: crate::types::UserSearchResult) -> Element {
    let mut avatar_failed = use_signal(|| false);
    use_effect(use_reactive((&result.user.avatar_url,), move |_| {
        avatar_failed.set(false)
    }));
    let initial = result
        .user
        .display_name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    rsx! {
        Link { to: Route::UserProfile { username: result.user.username.clone() }, class: "entity-result-card search-cook-card",
            div { class: "search-cook-avatar", aria_hidden: "true",
                if let Some(url) = result.user.avatar_url.filter(|_| !avatar_failed()) {
                    img { src: url, alt: "", loading: "lazy", onerror: move |_| avatar_failed.set(true) }
                } else { "{initial}" }
            }
            div { class: "entity-result-copy", h3 { "{result.user.display_name}" } p { class: "entity-path", "@{result.user.username}" } if let Some(bio) = result.user.bio { p { "{bio}" } } small { "{result.user.public_recipe_count} public recipes" } span { class: "entity-result-action", "View profile →" } }
        }
    }
}

#[component]
fn RecipeFacetSummary(facets: crate::types::RecipeSearchFacets) -> Element {
    if facets.tags.is_empty()
        && facets.visibility.is_empty()
        && facets.total_time.is_empty()
        && facets.servings.is_empty()
    {
        return rsx! {};
    }
    rsx! {
        details { class: "facet-summary",
            summary { "About these results" }
            if !facets.tags.is_empty() { div { h3 { "Tags" } div { class: "facet-list", for bucket in facets.tags.iter().take(8) { span { "{bucket.tag} · {bucket.count}" } } } } }
            if !facets.visibility.is_empty() { div { h3 { "Visibility" } div { class: "facet-list", for bucket in facets.visibility { span { "{bucket.visibility:?} · {bucket.count}" } } } } }
            if !facets.total_time.is_empty() { div { h3 { "Total time" } div { class: "facet-list", for bucket in facets.total_time { span { "{time_facet_label(&bucket)} · {bucket.count}" } } } } }
            if !facets.servings.is_empty() { div { h3 { "Servings" } div { class: "facet-list", for bucket in facets.servings { span { "{servings_facet_label(&bucket)} · {bucket.count}" } } } } }
        }
    }
}

fn time_facet_label(bucket: &crate::types::IntegerRangeFacetBucket) -> String {
    match (bucket.lower_inclusive, bucket.upper_exclusive) {
        (Some(0), Some(30)) => "Under 30 min".to_string(),
        (Some(30), Some(60)) => "30–60 min".to_string(),
        (Some(60), Some(120)) => "1–2 hours".to_string(),
        (Some(120), None) => "2+ hours".to_string(),
        (lower, upper) => range_facet_label(lower, upper, "min"),
    }
}

fn servings_facet_label(bucket: &crate::types::IntegerRangeFacetBucket) -> String {
    match (bucket.lower_inclusive, bucket.upper_exclusive) {
        (Some(1), Some(3)) => "1–2".to_string(),
        (Some(3), Some(5)) => "3–4".to_string(),
        (Some(5), Some(9)) => "5–8".to_string(),
        (Some(9), None) => "9+".to_string(),
        (lower, upper) => range_facet_label(lower, upper, "servings"),
    }
}

fn range_facet_label(lower: Option<i32>, upper: Option<i32>, unit: &str) -> String {
    match (lower, upper) {
        (Some(lower), Some(upper)) => format!("{lower}–{} {unit}", upper - 1),
        (Some(lower), None) => format!("{lower}+ {unit}"),
        (None, Some(upper)) => format!("Under {upper} {unit}"),
        (None, None) => format!("Any {unit}"),
    }
}

#[derive(Props, Clone, PartialEq)]
struct PaginationProps {
    page: u32,
    page_size: u32,
    total: i64,
    onchange: EventHandler<u32>,
}

#[component]
fn Pagination(props: PaginationProps) -> Element {
    let has_next = i64::from(props.page) * i64::from(props.page_size) < props.total;
    if props.page == 1 && !has_next {
        return rsx! {};
    }
    rsx! { nav { class: "search-pagination", aria_label: "Search result pages",
        button { class: "btn btn-secondary", disabled: props.page <= 1, onclick: move |_| props.onchange.call(props.page.saturating_sub(1).max(1)), "Previous" }
        span { "Page {props.page}" }
        button { class: "btn btn-secondary", disabled: !has_next, onclick: move |_| props.onchange.call(props.page + 1), "Next" }
    } }
}

#[component]
fn NoResults(blank: bool) -> Element {
    let (title, description) = if blank {
        (
            "Nothing here yet",
            "Create recipes and collections, or search for cooks sharing public recipes.",
        )
    } else {
        (
            "No results found",
            "Try fewer filters, a different phrase, or another search type.",
        )
    };
    rsx! { EmptyState { icon: rsx! { "⌕" }, title, description } }
}

#[component]
fn SearchValidationState(message: String) -> Element {
    rsx! { div { class: "search-state-card search-state-error", role: "alert", h2 { "This search URL is invalid" } p { "{message}" } Link { to: Route::Search { query: SearchRouteQuery::default() }, class: "btn btn-primary", "Reset search" } } }
}

#[component]
fn SearchErrorState(onretry: EventHandler<()>) -> Element {
    rsx! { div { class: "search-state-card search-state-error", role: "alert", h2 { "Search is temporarily unavailable" } p { "Your search and filters are still here. Please try again." } button { class: "btn btn-secondary", onclick: move |_| onretry.call(()), "Try again" } } }
}

fn visibility_value(value: RecipeSearchVisibility) -> &'static str {
    match value {
        RecipeSearchVisibility::AllVisible => "all",
        RecipeSearchVisibility::MineOnly => "mine",
        RecipeSearchVisibility::PublicOnly => "public",
    }
}
fn range_from_time(value: &str) -> Option<IntegerRange> {
    match value {
        "0-30" => Some(IntegerRange {
            lower_inclusive: Some(0),
            upper_exclusive: Some(30),
        }),
        "30-60" => Some(IntegerRange {
            lower_inclusive: Some(30),
            upper_exclusive: Some(60),
        }),
        "60-120" => Some(IntegerRange {
            lower_inclusive: Some(60),
            upper_exclusive: Some(120),
        }),
        "120+" => Some(IntegerRange {
            lower_inclusive: Some(120),
            upper_exclusive: None,
        }),
        _ => None,
    }
}
fn time_value(range: IntegerRange) -> Option<String> {
    match (range.lower_inclusive, range.upper_exclusive) {
        (Some(0), Some(30)) => Some("0-30".into()),
        (Some(30), Some(60)) => Some("30-60".into()),
        (Some(60), Some(120)) => Some("60-120".into()),
        (Some(120), None) => Some("120+".into()),
        _ => None,
    }
}
fn range_from_servings(value: &str) -> Option<IntegerRange> {
    match value {
        "1-2" => Some(IntegerRange {
            lower_inclusive: Some(1),
            upper_exclusive: Some(3),
        }),
        "3-4" => Some(IntegerRange {
            lower_inclusive: Some(3),
            upper_exclusive: Some(5),
        }),
        "5-8" => Some(IntegerRange {
            lower_inclusive: Some(5),
            upper_exclusive: Some(9),
        }),
        "9+" => Some(IntegerRange {
            lower_inclusive: Some(9),
            upper_exclusive: None,
        }),
        _ => None,
    }
}
fn servings_value(range: IntegerRange) -> Option<String> {
    match (range.lower_inclusive, range.upper_exclusive) {
        (Some(1), Some(3)) => Some("1-2".into()),
        (Some(3), Some(5)) => Some("3-4".into()),
        (Some(5), Some(9)) => Some("5-8".into()),
        (Some(9), None) => Some("9+".into()),
        _ => None,
    }
}
fn comma_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn update_required_tags(request: &mut ActiveSearchRequest, tags: Vec<String>) {
    if let ActiveSearchRequest::Recipes(recipe) = request {
        recipe.tags_all = tags;
        recipe.page = 1;
    }
}

fn sort_value(request: &ActiveSearchRequest) -> &'static str {
    match request {
        ActiveSearchRequest::All(_) => "",
        ActiveSearchRequest::Recipes(value) => match value.sort {
            RecipeSearchSort::Relevance => "relevance",
            RecipeSearchSort::UpdatedNewest => "updated_newest",
            RecipeSearchSort::UpdatedOldest => "updated_oldest",
            RecipeSearchSort::TitleAscending => "title_ascending",
            RecipeSearchSort::TitleDescending => "title_descending",
            RecipeSearchSort::Quickest => "quickest",
            RecipeSearchSort::Longest => "longest",
        },
        ActiveSearchRequest::Collections(value) => match value.sort {
            CollectionSearchSort::Relevance => "relevance",
            CollectionSearchSort::UpdatedNewest => "updated_newest",
            CollectionSearchSort::UpdatedOldest => "updated_oldest",
            CollectionSearchSort::NameAscending => "name_ascending",
            CollectionSearchSort::NameDescending => "name_descending",
        },
        ActiveSearchRequest::Users(value) => match value.sort {
            UserSearchSort::Relevance => "relevance",
            UserSearchSort::UsernameAscending => "username_ascending",
            UserSearchSort::DisplayNameAscending => "display_name_ascending",
        },
    }
}

fn sort_options(request: &ActiveSearchRequest) -> Element {
    match request {
        ActiveSearchRequest::Recipes(_) => {
            rsx! { option { value: "relevance", "Relevance" } option { value: "updated_newest", "Newest" } option { value: "updated_oldest", "Oldest" } option { value: "title_ascending", "Title A–Z" } option { value: "title_descending", "Title Z–A" } option { value: "quickest", "Quickest" } option { value: "longest", "Longest" } }
        }
        ActiveSearchRequest::Collections(_) => {
            rsx! { option { value: "relevance", "Relevance" } option { value: "updated_newest", "Newest" } option { value: "updated_oldest", "Oldest" } option { value: "name_ascending", "Name A–Z" } option { value: "name_descending", "Name Z–A" } }
        }
        ActiveSearchRequest::Users(_) => {
            rsx! { option { value: "relevance", "Relevance" } option { value: "username_ascending", "Username" } option { value: "display_name_ascending", "Display name" } }
        }
        ActiveSearchRequest::All(_) => rsx! {},
    }
}

fn apply_sort(request: &mut ActiveSearchRequest, value: &str) {
    match request {
        ActiveSearchRequest::Recipes(request) => {
            request.sort = match value {
                "updated_newest" => RecipeSearchSort::UpdatedNewest,
                "updated_oldest" => RecipeSearchSort::UpdatedOldest,
                "title_ascending" => RecipeSearchSort::TitleAscending,
                "title_descending" => RecipeSearchSort::TitleDescending,
                "quickest" => RecipeSearchSort::Quickest,
                "longest" => RecipeSearchSort::Longest,
                _ => RecipeSearchSort::Relevance,
            }
        }
        ActiveSearchRequest::Collections(request) => {
            request.sort = match value {
                "updated_newest" => CollectionSearchSort::UpdatedNewest,
                "updated_oldest" => CollectionSearchSort::UpdatedOldest,
                "name_ascending" => CollectionSearchSort::NameAscending,
                "name_descending" => CollectionSearchSort::NameDescending,
                _ => CollectionSearchSort::Relevance,
            }
        }
        ActiveSearchRequest::Users(request) => {
            request.sort = match value {
                "username_ascending" => UserSearchSort::UsernameAscending,
                "display_name_ascending" => UserSearchSort::DisplayNameAscending,
                _ => UserSearchSort::Relevance,
            }
        }
        ActiveSearchRequest::All(_) => {}
    }
}

fn has_filters(request: &ActiveSearchRequest) -> bool {
    match request {
        ActiveSearchRequest::Recipes(value) => {
            value.visibility != RecipeSearchVisibility::AllVisible
                || !value.tags_all.is_empty()
                || value.total_time.is_some()
                || value.servings.is_some()
        }
        _ => false,
    }
}
fn filter_chips(request: &ActiveSearchRequest) -> Vec<String> {
    match request {
        ActiveSearchRequest::Recipes(value) => {
            let mut chips = value
                .tags_all
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect::<Vec<_>>();
            if value.visibility != RecipeSearchVisibility::AllVisible {
                chips.push(
                    match value.visibility {
                        RecipeSearchVisibility::MineOnly => "Mine only",
                        RecipeSearchVisibility::PublicOnly => "Public only",
                        _ => "",
                    }
                    .to_string(),
                );
            }
            if let Some(range) = value.total_time.and_then(time_value) {
                chips.push(format!("Time {range}"));
            }
            if let Some(range) = value.servings.and_then(servings_value) {
                chips.push(format!("Serves {range}"));
            }
            chips
        }
        _ => Vec::new(),
    }
}

// Chips remove one criterion while keeping the query, sort and advanced fields.
fn remove_filter(mut request: ActiveSearchRequest, chip: &str) -> ActiveSearchRequest {
    if let ActiveSearchRequest::Recipes(value) = &mut request {
        if let Some(tag) = chip.strip_prefix('#') {
            value.tags_all.retain(|candidate| candidate != tag);
        } else if matches!(chip, "Mine only" | "Public only") {
            value.visibility = RecipeSearchVisibility::AllVisible;
        } else if chip.starts_with("Time ") {
            value.total_time = None;
        } else if chip.starts_with("Serves ") {
            value.servings = None;
        }
        value.page = 1;
    }
    request
}

fn scoped_search_route(request: &ActiveSearchRequest, scope: SearchScope) -> Option<Route> {
    parsed_from_request(request.clone())
        .ok()?
        .switch_scope(scope)
        .ok()
        .map(|parsed| Route::Search {
            query: SearchRouteQuery::new(parsed.canonical_query),
        })
}
fn clear_filters(mut request: ActiveSearchRequest) -> ActiveSearchRequest {
    if let ActiveSearchRequest::Recipes(value) = &mut request {
        value.visibility = RecipeSearchVisibility::AllVisible;
        value.tags_all.clear();
        value.total_time = None;
        value.servings = None;
        value.page = 1;
    }
    request
}

fn mode_value(mode: TextMatchMode) -> &'static str {
    match mode {
        TextMatchMode::AllTerms => "all",
        TextMatchMode::AnyTerm => "any",
        TextMatchMode::ExactPhrase => "exact",
    }
}
fn form_text(event: &Event<FormData>, name: &str) -> String {
    match event.get_first(name) {
        Some(FormValue::Text(value)) => value.trim().to_string(),
        _ => String::new(),
    }
}
fn mode_from_form(event: &Event<FormData>, prefix: &str) -> TextMatchMode {
    match form_text(event, &format!("{prefix}_mode")).as_str() {
        "any" => TextMatchMode::AnyTerm,
        "exact" => TextMatchMode::ExactPhrase,
        _ => TextMatchMode::AllTerms,
    }
}
fn text_field_from_form(event: &Event<FormData>, prefix: &str) -> AdvancedTextField {
    let mode = mode_from_form(event, prefix);
    let include = form_text(event, &format!("{prefix}_include"));
    let exclude = comma_values(&form_text(event, &format!("{prefix}_exclude")))
        .into_iter()
        .map(|text| TextCriterion { text, mode })
        .collect();
    AdvancedTextField {
        include: (!include.is_empty()).then_some(TextCriterion {
            text: include,
            mode,
        }),
        exclude,
    }
}
fn date_range_from_form(event: &Event<FormData>, prefix: &str) -> Option<TimestampRange> {
    let lower = parse_date(&form_text(event, &format!("{prefix}_from")));
    let upper = parse_date(&form_text(event, &format!("{prefix}_before")));
    (lower.is_some() || upper.is_some()).then_some(TimestampRange {
        lower_inclusive: lower,
        upper_exclusive: upper,
    })
}
fn parse_date(value: &str) -> Option<chrono::DateTime<Utc>> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| Utc.from_utc_datetime(&date))
}

fn advanced_from_form(
    request: &ActiveSearchRequest,
    event: &Event<FormData>,
) -> Option<ActiveAdvancedCriteria> {
    match request {
        ActiveSearchRequest::Recipes(_) => Some(ActiveAdvancedCriteria::Recipes(
            AdvancedRecipeSearchCriteria {
                title: text_field_from_form(event, "title"),
                description: text_field_from_form(event, "description"),
                ingredients: text_field_from_form(event, "ingredients"),
                instructions: text_field_from_form(event, "instructions"),
                tags_any: comma_values(&form_text(event, "tags_any")),
                tags_excluded: comma_values(&form_text(event, "tags_excluded")),
                created_at: date_range_from_form(event, "created"),
                updated_at: date_range_from_form(event, "updated"),
            },
        )),
        ActiveSearchRequest::Collections(_) => Some(ActiveAdvancedCriteria::Collections(
            AdvancedCollectionSearchCriteria {
                name: text_field_from_form(event, "name"),
                description: text_field_from_form(event, "description"),
                updated_at: date_range_from_form(event, "updated"),
            },
        )),
        ActiveSearchRequest::Users(_) => {
            Some(ActiveAdvancedCriteria::Users(AdvancedUserSearchCriteria {
                username: text_field_from_form(event, "username"),
                display_name: text_field_from_form(event, "display_name"),
                bio: text_field_from_form(event, "bio"),
            }))
        }
        ActiveSearchRequest::All(_) => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn over_budget_search_submission_has_visible_feedback() {
        let request = crate::search_url::ActiveSearchRequest::All(crate::types::AllSearchRequest {
            text: vec!["x"; 65].join(" "),
            ..Default::default()
        });
        assert!(super::search_transition(request, super::SearchInteraction::Submit).is_err());
        let source = include_str!("search.rs");
        let component = source.split("mod tests {").next().unwrap();
        assert!(component.contains("navigation_error.set(Some(error))"));
        assert!(component.contains("navigation_error.set(None)"));
        assert!(component.contains("if let Some(message) = navigation_error()"));
        assert!(component.contains("role: \"alert\""));
    }

    use std::str::FromStr;

    use super::*;
    use crate::search_url::SearchRouteQuery;
    use crate::Route;

    #[test]
    fn search_route_preserves_the_complete_query_for_canonical_parsing() {
        let route = Route::from_str("/search?scope=users&q=chef&sort=username_ascending")
            .expect("search route should parse");
        let Route::Search { query } = route else {
            panic!("expected search route");
        };
        assert_eq!(query.as_str(), "scope=users&q=chef&sort=username_ascending");
        assert_eq!(
            Route::Search {
                query: SearchRouteQuery::new("scope=users&q=chef")
            }
            .to_string(),
            "/search?scope=users&q=chef"
        );
    }

    #[test]
    fn typing_replaces_history_while_committed_actions_push() {
        assert_eq!(
            history_action(SearchInteraction::Typing),
            HistoryAction::Replace
        );
        for interaction in [
            SearchInteraction::Submit,
            SearchInteraction::Suggestion,
            SearchInteraction::Filter,
            SearchInteraction::Sort,
            SearchInteraction::Pagination,
            SearchInteraction::Scope,
        ] {
            assert_eq!(history_action(interaction), HistoryAction::Push);
        }
    }

    #[test]
    fn url_restoration_and_navigation_use_the_component_boundaries() {
        let restored = restored_search_state(&SearchRouteQuery::new(
            "scope=collections&q=weeknight&page=2",
        ))
        .expect("history URL should restore");
        assert_eq!(
            restored,
            ("weeknight".to_string(), SearchScope::Collections)
        );

        let request = ActiveSearchRequest::Users(crate::types::UserSearchRequest {
            text: Some("maya".to_string()),
            ..crate::types::UserSearchRequest::default()
        });
        let (typing_route, typing_action) =
            search_transition(request.clone(), SearchInteraction::Typing)
                .expect("typing transition should be valid");
        assert_eq!(typing_route.to_string(), "/search?scope=users&q=maya");
        assert_eq!(typing_action, HistoryAction::Replace);

        let (_, mouse_selection_action) = search_transition(request, SearchInteraction::Suggestion)
            .expect("suggestion transition should be valid");
        assert_eq!(mouse_selection_action, HistoryAction::Push);
    }

    #[test]
    fn autocomplete_keyboard_navigation_is_bounded_and_wraps() {
        assert_eq!(next_suggestion(None, 3, SuggestionDirection::Down), Some(0));
        assert_eq!(
            next_suggestion(Some(2), 3, SuggestionDirection::Down),
            Some(0)
        );
        assert_eq!(
            next_suggestion(Some(0), 3, SuggestionDirection::Up),
            Some(2)
        );
        assert_eq!(next_suggestion(Some(1), 0, SuggestionDirection::Down), None);
    }

    #[test]
    fn typed_suggestions_navigate_to_their_entity_routes() {
        let id = uuid::Uuid::new_v4();
        let suggestion = |kind, secondary_text: Option<&str>| SearchSuggestion {
            id,
            kind,
            label: "Result".to_string(),
            secondary_text: secondary_text.map(str::to_string),
        };

        assert_eq!(
            suggestion_route(&suggestion(SearchSuggestionKind::Recipe, None)),
            Some(Route::RecipeDetail { id: id.to_string() })
        );
        assert_eq!(
            suggestion_route(&suggestion(SearchSuggestionKind::Collection, None)),
            Some(Route::CollectionDetail { id: id.to_string() })
        );
        assert_eq!(
            suggestion_route(&suggestion(SearchSuggestionKind::User, Some("@maya-spice"))),
            Some(Route::UserProfile {
                username: "maya-spice".to_string()
            })
        );
        assert_eq!(
            suggestion_route(&suggestion(
                SearchSuggestionKind::User,
                Some("missing-prefix")
            )),
            None
        );
    }

    #[test]
    fn advanced_criteria_create_one_bounded_scope_request_and_preserve_simple_text() {
        let criteria = AdvancedRecipeSearchCriteria {
            title: AdvancedTextField {
                include: Some(TextCriterion {
                    text: "silky noodles".to_string(),
                    mode: TextMatchMode::ExactPhrase,
                }),
                exclude: Vec::new(),
            },
            ..AdvancedRecipeSearchCriteria::default()
        };
        let request = ActiveSearchRequest::Recipes(Box::new(crate::types::RecipeSearchRequest {
            text: Some("weeknight".to_string()),
            advanced: Some(criteria.clone()),
            page: 3,
            ..crate::types::RecipeSearchRequest::default()
        }));

        let parsed = parsed_from_request(request).expect("advanced request should be bounded");
        let reparsed = parse_search_query(&parsed.canonical_query)
            .expect("canonical advanced request should parse");
        let ActiveSearchRequest::Recipes(request) = reparsed.request else {
            panic!("expected exactly one recipe request");
        };
        assert_eq!(request.text.as_deref(), Some("weeknight"));
        assert_eq!(request.advanced, Some(criteria));
        assert_eq!(request.page, 3);
    }

    #[test]
    fn advanced_form_names_modes_exclusions_and_dates_build_the_request() {
        use dioxus::html::{FileData, FormValue, HasFileData, HasFormData};
        use std::rc::Rc;

        #[derive(Clone)]
        struct TestFormData(Vec<(String, FormValue)>);
        impl HasFileData for TestFormData {
            fn files(&self) -> Vec<FileData> {
                Vec::new()
            }
        }
        impl HasFormData for TestFormData {
            fn value(&self) -> String {
                String::new()
            }
            fn valid(&self) -> bool {
                true
            }
            fn values(&self) -> Vec<(String, FormValue)> {
                self.0.clone()
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        let values = [
            ("title_include", "silky noodles"),
            ("title_mode", "exact"),
            ("title_exclude", "instant, boxed"),
            ("tags_any", "vegan, quick"),
            ("created_from", "2026-01-02"),
            ("created_before", "2026-02-03"),
        ]
        .into_iter()
        .map(|(key, text)| (key.to_string(), FormValue::Text(text.to_string())))
        .collect();
        let event = Event::new(Rc::new(FormData::new(TestFormData(values))), true);
        let base =
            ActiveSearchRequest::Recipes(Box::new(crate::types::RecipeSearchRequest::default()));
        let Some(ActiveAdvancedCriteria::Recipes(criteria)) = advanced_from_form(&base, &event)
        else {
            panic!("recipe form should build recipe criteria");
        };

        let title = criteria.title.include.expect("title include");
        assert_eq!(title.text, "silky noodles");
        assert_eq!(title.mode, TextMatchMode::ExactPhrase);
        assert_eq!(
            criteria
                .title
                .exclude
                .iter()
                .map(|value| value.text.as_str())
                .collect::<Vec<_>>(),
            vec!["instant", "boxed"]
        );
        assert_eq!(criteria.tags_any, vec!["vegan", "quick"]);
        let created = criteria.created_at.expect("created range");
        assert_eq!(
            created
                .lower_inclusive
                .unwrap()
                .format("%Y-%m-%d")
                .to_string(),
            "2026-01-02"
        );
        assert_eq!(
            created
                .upper_exclusive
                .unwrap()
                .format("%Y-%m-%d")
                .to_string(),
            "2026-02-03"
        );
    }

    #[test]
    fn landing_submission_builds_one_canonical_search_route() {
        let route = landing_search_route("  silky noodles  ").expect("landing query should parse");
        assert_eq!(route.to_string(), "/search?q=silky%20noodles");
        assert!(landing_search_route("   ").is_none());
    }

    #[test]
    fn noncanonical_hydration_replaces_the_url_once() {
        let parsed = parse_search_query("scope=recipes&tags=Zesty,zesty&unused=yes")
            .expect("query should canonicalize");
        let route = canonical_route_if_needed("scope=recipes&tags=Zesty,zesty&unused=yes", &parsed)
            .expect("noncanonical state should be replaced");
        assert_eq!(
            route.to_string(),
            "/search?scope=recipes&sort=updated_newest&tags=Zesty%2Czesty"
        );
        assert!(canonical_route_if_needed(&parsed.canonical_query, &parsed).is_none());
    }

    #[test]
    fn removing_one_filter_preserves_the_other_search_criteria() {
        let request = ActiveSearchRequest::Recipes(Box::new(crate::types::RecipeSearchRequest {
            text: Some("tomato & basil".into()),
            tags_all: vec!["quick".into(), "vegetarian".into()],
            total_time: range_from_time("0-30"),
            servings: range_from_servings("3-4"),
            visibility: RecipeSearchVisibility::MineOnly,
            sort: RecipeSearchSort::TitleAscending,
            page: 4,
            ..Default::default()
        }));
        let ActiveSearchRequest::Recipes(removed) = remove_filter(request.clone(), "#quick") else {
            panic!("recipe request")
        };
        assert_eq!(removed.tags_all, vec!["vegetarian"]);
        assert_eq!(removed.page, 1);
        assert_eq!(removed.text.as_deref(), Some("tomato & basil"));
        assert_eq!(removed.sort, RecipeSearchSort::TitleAscending);
        assert_eq!(removed.total_time, range_from_time("0-30"));
        assert_eq!(removed.servings, range_from_servings("3-4"));
        assert_eq!(removed.visibility, RecipeSearchVisibility::MineOnly);
        for chip in ["Time 0-30", "Serves 3-4", "Mine only"] {
            let ActiveSearchRequest::Recipes(removed) = remove_filter(request.clone(), chip) else {
                panic!("recipe request")
            };
            assert_eq!(removed.tags_all.len(), 2);
            assert_eq!(removed.total_time.is_none(), chip.starts_with("Time "));
            assert_eq!(removed.servings.is_none(), chip.starts_with("Serves "));
            assert_eq!(
                removed.visibility == RecipeSearchVisibility::AllVisible,
                chip == "Mine only"
            );
        }
    }

    #[test]
    fn grouped_result_links_keep_the_query_and_select_the_requested_scope() {
        let request = parse_search_query("q=tomato%20%26%20basil")
            .unwrap()
            .request;
        for scope in [
            SearchScope::Recipes,
            SearchScope::Collections,
            SearchScope::Users,
        ] {
            let Some(Route::Search { query }) = scoped_search_route(&request, scope) else {
                panic!("search route")
            };
            let parsed = parse_search_query(query.as_str()).unwrap();
            assert_eq!(parsed.request.text(), "tomato & basil");
            assert_eq!(parsed.request.scope(), scope);
        }
    }

    #[test]
    fn clearing_recipe_filters_resets_pagination_and_keeps_search_text() {
        let request = ActiveSearchRequest::Recipes(Box::new(crate::types::RecipeSearchRequest {
            text: Some("noodles".to_string()),
            visibility: RecipeSearchVisibility::MineOnly,
            tags_all: vec!["quick".to_string()],
            total_time: range_from_time("0-30"),
            servings: range_from_servings("3-4"),
            page: 4,
            ..crate::types::RecipeSearchRequest::default()
        }));
        let ActiveSearchRequest::Recipes(cleared) = clear_filters(request) else {
            panic!("expected recipe request");
        };
        assert_eq!(cleared.text.as_deref(), Some("noodles"));
        assert_eq!(cleared.visibility, RecipeSearchVisibility::AllVisible);
        assert!(cleared.tags_all.is_empty());
        assert!(cleared.total_time.is_none());
        assert!(cleared.servings.is_none());
        assert_eq!(cleared.page, 1);
    }

    #[test]
    fn required_tag_input_updates_recipe_filters_and_resets_pagination() {
        let mut request =
            ActiveSearchRequest::Recipes(Box::new(crate::types::RecipeSearchRequest {
                page: 4,
                ..crate::types::RecipeSearchRequest::default()
            }));

        update_required_tags(&mut request, comma_values(" vegan, weeknight, "));

        let ActiveSearchRequest::Recipes(request) = request else {
            panic!("expected recipe request");
        };
        assert_eq!(request.tags_all, vec!["vegan", "weeknight"]);
        assert_eq!(request.page, 1);
    }

    #[test]
    fn every_fixed_integer_facet_has_a_readable_label() {
        let bucket = |lower, upper| crate::types::IntegerRangeFacetBucket {
            lower_inclusive: Some(lower),
            upper_exclusive: upper,
            count: 0,
        };
        assert_eq!(time_facet_label(&bucket(0, Some(30))), "Under 30 min");
        assert_eq!(time_facet_label(&bucket(120, None)), "2+ hours");
        assert_eq!(servings_facet_label(&bucket(1, Some(3))), "1–2");
        assert_eq!(servings_facet_label(&bucket(9, None)), "9+");
    }
}
