//! Editor feedback is advisory; publication and the unique index remain authoritative.
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum NameStatus {
    Clear,
    Checking,
    Taken,
    TooLong,
    Unavailable,
}

impl NameStatus {
    pub(super) fn blocked(self) -> bool {
        matches!(self, Self::Checking | Self::Taken | Self::TooLong)
    }

    pub(super) fn invalid(self) -> bool {
        matches!(self, Self::Taken | Self::TooLong)
    }

    pub(super) fn message(self) -> &'static str {
        match self {
            Self::Clear => "",
            Self::Checking => "Checking recipe name…",
            Self::Taken => {
                "You already have a recipe with this name. Choose a different name to publish."
            }
            Self::TooLong => "Use 255 characters or fewer for the recipe name.",
            Self::Unavailable => "Could not check this name. Publishing will check again.",
        }
    }
}

pub(super) fn use_name_check(
    title: String,
    account: Option<uuid::Uuid>,
    recipe_id: Option<uuid::Uuid>,
) -> Memo<NameStatus> {
    let response = use_resource(use_reactive(
        (&title, &account, &recipe_id),
        move |(title, account, recipe_id)| async move {
            let result =
                if title.trim().is_empty() || title.chars().count() > 255 || account.is_none() {
                    Ok(true)
                } else {
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(300).await;
                    crate::api::recipe::recipe_name_available(account, title.clone(), recipe_id)
                        .await
                        .map_err(|_| ())
                };
            (title, account, recipe_id, result)
        },
    ));
    use_memo(use_reactive(
        (&title, &account, &recipe_id),
        move |(title, account, recipe_id)| {
            if title.trim().is_empty() {
                return NameStatus::Clear;
            }
            if title.chars().count() > 255 {
                return NameStatus::TooLong;
            }
            let response = response.read();
            match response.as_ref() {
                Some((query, viewer, recipe, result))
                    if query == &title && *viewer == account && *recipe == recipe_id =>
                {
                    match result {
                        Ok(true) => NameStatus::Clear,
                        Ok(false) => NameStatus::Taken,
                        Err(()) => NameStatus::Unavailable,
                    }
                }
                _ => NameStatus::Checking,
            }
        },
    ))
}

#[component]
pub(super) fn NameFeedback(status: NameStatus) -> Element {
    let message = status.message();
    rsx! {
        p { id: "recipe-name-status", class: "recipe-editor__hint", role: "status",
            aria_live: "polite", "{message}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_pending_and_oversized_names_block_publication_but_not_drafts() {
        for status in [NameStatus::Taken, NameStatus::Checking, NameStatus::TooLong] {
            assert!(status.blocked());
            assert!(!status.message().is_empty());
        }
        assert!(!NameStatus::Clear.blocked());
        assert!(!NameStatus::Unavailable.blocked());
        assert!(NameStatus::Taken.invalid());
    }
}
