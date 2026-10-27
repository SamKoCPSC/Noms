//! Only the public environment marker crosses SSR hydration; credentials never do.
#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct StagingEnvironment(pub bool);

pub(crate) fn from_fullstack() -> StagingEnvironment {
    #[cfg(feature = "server")]
    if let Some(context) = dioxus::fullstack::FullstackContext::current() {
        return context
            .extension::<StagingEnvironment>()
            .unwrap_or_default();
    }
    StagingEnvironment::default()
}
