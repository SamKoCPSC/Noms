# NOMS-007: Minor UI Fixes and Routing Cleanups

**Type:** Task / Bugfix
**Priority:** Low
**Status:** In Progress
**Created:** 2026-06-06

---

## Description

Address minor UI inconsistencies and missing edge-case handling
identified during systematic code review of all 38 manual test
scenarios (TC-01 through TC-37).

---

## Sub-tasks

### [x] TC-36: Add 404 catch-all route

No branded 404 page existed. Unknown routes returned a plain
framework default.

**Fix:** Created `src/pages/not_found.rs` with themed UI, wired
`#[route("/:..segments")]` catch-all in `src/main.rs`, added
`.not-found` CSS class.

**Files:**
- `src/pages/not_found.rs` (new)
- `src/pages/mod.rs`
- `src/main.rs`
- `assets/main.css`

### [x] not_found.rs: Replace inline CSS with class

Component used inline styles on every element while a `.not-found`
CSS class existed but was never applied.

**Fix:** Replaced nested div wrapper with single
`div.not-found.container`, removed inline styles covered by CSS
class.

**Files:**
- `src/pages/not_found.rs`

### [ ] TC-07: Add avatar to profile settings page

Avatar renders in navbar but not on `/settings/profile` page itself.
User has no visual identity cue on the page where they edit their
profile.

**Fix:** Add `Avatar` component to profile page header, below
`PageHeader`, using `auth_context.current_user.avatar_url` and
`display_name` for initials fallback.

**Files:**
- `src/pages/settings/settings_profile.rs`

---

## Acceptance Criteria

- [ ] All sub-tasks verified against their respective test case in
  `docs/manual-test-guide.md`
- [ ] No regressions in existing test suite
- [ ] Builds pass on both `server` and `web` targets
