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
- Original source-worktree browser acceptance was blocked by the URL security policy; no bypass was attempted. The [2026-09-27 integrated pass](NOMS-020-checkpoint-8-integration.md) subsequently verified create/edit/save, populated membership and selected narrow/tablet geometry, and found a subcollection parent-context defect. Uploads, complete keyboard/visual acceptance and remaining states are still open; tests alone are not visual approval.

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

Integrated: source tip `aa5280e` is an ancestor of integration revision `74c4200`, verified 2026-09-27. See the [combined review](NOMS-020-checkpoint-8-integration.md) for acceptance, fixtures and the nested-creation follow-up. Finish nested detail, cover upload, owner-versus-visitor controls, keyboard and light/dark/responsive review before final acceptance; merge status does not close those checks.

Follow-up `9b6d667` (2026-09-27): parent is now a typed creation-route parameter,
shared by SSR/client form state, Cancel and submission. Regression tests and the
browser confirmed nested creation, parent breadcrumb/membership, and top-level
fallback. The [integration report](NOMS-020-checkpoint-8-integration.md#parent-context-fix-validation--2026-09-27)
records exact commands, runtime evidence and a separate stale-content issue when
navigating between collection IDs; that issue is not fixed by this change.
