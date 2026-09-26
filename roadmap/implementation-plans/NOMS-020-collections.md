# NOMS-020 Collections implementation checkpoint

Implemented on `codex/NOMS-020-collections`, based on integrated design commit `7b2dab2`.

## Delivered

- Collection list: C3 Kitchen Albums cards with existing photo mosaics or uploaded covers, real counts, visibility badges, coral bookmarks and a top-level name/description filter. Explicit loading, retry, empty and no-match states.
- Collection details: wrapping breadcrumbs, title/owner/visibility header, 16:9 cover, compact subcollection albums and framed recipe cards. Owner actions stay capability-gated; existing pagination, reordering, membership removal, sharing and typed recursive-delete safeguards remain unchanged.
- New collection: separate raised details, location/sharing and cover panels, inset fields and explicit cancel/create actions. Private default, nested creation and upload-on-save retained.
- Edit collection: the same panel system, existing parent selection with descendant exclusion, sharing preview and deferred cover upload. Existing partial-save failure explanations retained; cancel returns to the collection.

The four routes use existing Fredoka/Nunito typography and global neumorphic tokens. Styles are scoped in `assets/collections.css` and `assets/collection-forms.css`; no global token, backend or schema changes. Grids adapt to available width and form panels stack below 760px. Dark redesign and production page-icon preparation remain deferred.

The new filter covers only already-loaded top-level collections; it does not claim to search nested content. Album covers and counts come from permission-filtered responses, not fabricated display data. The details page's Add recipes action leads to the existing dashboard workflow, not a new direct picker.

## Acceptance and evidence

- Collection data and navigation: rendered album tests cover real counts, visibility, escaped user text, compact/full variants and one navigation target; reorder controls remain outside that link.
- List filtering: tests cover names, descriptions, case, Unicode, whitespace and no match.
- Existing behavior: collection regression suite covers hierarchy, permissions, membership, cover privacy, quotas, pagination, reordering, typed deletion, validation and save failures.
- Independent source review: PASS; the informational long-description wrapping issue was fixed.
- Visual and interactive browser acceptance: **pending**. The browser URL security policy blocked the fresh local preview. No bypass was attempted. Desktop/mobile/tablet appearance, live form submission, uploads and keyboard walkthroughs still need browser verification; passing tests and static responsive CSS are not visual approval.

## Automated validation

Run inside the fresh development container `noms-noms020-collections-20260925-app-1` (Rust 1.97.1). PostgreSQL 15 test binaries and Clippy were installed only in that disposable container. Tests require a UTF-8 locale; the running preview's cookie namespace and local insecure-cookie mode must not leak into tests that expect production cookie defaults.

```sh
docker exec -e LANG=C.UTF-8 -e LC_ALL=C.UTF-8 -e SQLX_OFFLINE=true -e BASE_URL=http://localhost:3000 noms-noms020-collections-20260925-app-1 env -u NOMS_SESSION_COOKIE_NAME -u NOMS_ENV /usr/local/cargo/bin/cargo test --features server -- --test-threads=4
```

- Full server suite: **763 passed, 0 failed, 12 ignored** (existing ignored tests).
- Focused `cargo test --features server collection -- --test-threads=4`: **89 passed, 0 failed, 2 ignored** (ParadeDB search integration prerequisites).
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings`: passed.
- `cargo clippy --target wasm32-unknown-unknown -- -D warnings`: passed.
- `cargo fmt --check` and `git diff --check`: passed.

Preview started with `just up noms020-collections-20260925`, at `http://127.0.0.1:32770`. After restarting for the final source, the Dioxus build completed and server logs reported HTTP 200 for `/`. This is startup evidence only. This is a fresh isolated deployment, not the integration deployment. No browser fixtures or successful browser interactions are claimed.

## Integration

This branch is not merged into the NOMS-020 integration worktree. Keep the shared album component and four route compositions together as one cohesive implementation commit, followed by the planning/evidence checkpoint. Before accepting the slice, visually review populated and empty lists, nested detail pages, new/edit forms, narrow layouts and light/dark usability; exercise cover upload, cancel/save and owner-versus-visitor controls.
