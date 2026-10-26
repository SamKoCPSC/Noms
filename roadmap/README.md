# Noms — Product Roadmap

A lightweight Kanban board for tracking work. Every item lives here; no external tools required.

## Statuses

| Indicator | Meaning |
|-----------|---------|
| ⚪ Backlog | Valid idea, not yet prioritized or scoped |
| 🔵 Ready | Scoped and approved, blocked only on capacity |
| 🟡 In Progress | Actively being worked on right now |
| ✅ Done | Shipped and verified |

## Board

| ID | Title | Status | Phase | Notes |
|----|-------|--------|-------|-------|
| [NOMS-000](issues/NOMS-000-open-code-editor.md) | Open code editor | ✅ Done | Pre-pre-work | The foundational step upon which all other steps depend. Without it, nothing else is possible. |
| [NOMS-001](issues/NOMS-001-design-document.md) | Write design document outlining the high-level plan | ✅ Done | Pre-work | `DESIGN.md` — 2,338 lines covering product vision, architecture, data model, visual design, offline strategy |
| [NOMS-002](issues/NOMS-002-initial-project-setup.md) | Initial project setup — repository, workspace & infrastructure | ✅ Done | Pre-work | Rust workspace, Railway project + Postgres, R2 bucket, CI skeleton, local dev tooling |
| [NOMS-003](issues/NOMS-003-ui-scaffold.md) | UI scaffold & application shell | ⚪ Backlog | Phase 1 | Route architecture, layout components, design system foundations, template cleanup |
| [NOMS-008](issues/NOMS-008-recipe-crud.md) | Recipe CRUD & Versioning | 🔵 Ready | Phase 1 | Core recipe creation, viewing, editing, deletion, and version history. [Implementation plan](implementation-plans/NOMS-008-recipe-crud.md) — 8 checkpoints, bottom-up by dependency |
| [NOMS-014](issues/NOMS-014-isolated-development-environments.md) | Fully isolated `just up` development environments | ✅ Done | Developer infrastructure | Disposable per-invocation app and infrastructure stacks for concurrent agents. [Implementation plan](implementation-plans/NOMS-014-isolated-development-environments.md) — 7 checkpoints |
| [NOMS-015](issues/NOMS-015-security-hardening.md) | Security hardening to a good-enough baseline | 🔵 Ready | Security | Close the confirmed OAuth account-takeover, cross-tenant media deletion, and global OAuth rate-limit paths. [Implementation plan](implementation-plans/NOMS-015-security-hardening.md) — 8 test-first checkpoints with independent adversarial review. |
| [NOMS-016](implementation-plans/NOMS-016-recipe-import-export.md) | Recipe Import & Export | 🟡 In Progress | Portability | Clipboard import manually accepted. Recipe JSON/ZIP and offline HTML export, image recovery, and private collection-tree JSON round trips are implemented. Collection HTML supports a unified cookbook or individual recipe documents in a ZIP. Schema.org interoperability remains planned. |
| [NOMS-018](implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md) | Kitchen Workspace, Pantries, Grocery Lists & Recipe Feasibility | 🟡 In Progress | Kitchen | Independent database-backed grocery lists implemented with Workspace 2 / Create list 10, manual and accessible-recipe sources, serving-aware exact name/unit totals, full item edits, and original recipe context. Locally verified with 50 focused tests, schema upgrade, and 18 browser checks. Merged in [PR #21](https://github.com/SamKoCPSC/Noms/pull/21). [Pantry inventory](implementation-plans/NOMS-018-pantry-inventory.md) is implemented and locally verified with selected W01/C01, 58 focused tests, 18 browser checks, and fresh/upgrade schema checks; merged in [PR #25](https://github.com/SamKoCPSC/Noms/pull/25). [Kitchen workspace unification](implementation-plans/NOMS-018-kitchen-workspace.md) brings both views together and renames inventory profiles to pantries. [Recipe detail pantry checks](implementation-plans/NOMS-018-recipe-pantry-check.md) add an ingredient-only, presence-based pantry selector. Cookable search, shortage-list generation, and offline sync remain later slices. |
| [NOMS-021](issues/NOMS-021-production-readiness-follow-ups.md) | Production readiness and reliability follow-ups | 🔵 Ready | Production hardening | Resolve the final NOMS-015 review findings, restore anonymous public search and route-correct SPA metadata, complete a bounded readability pass, and formalize manual regression coverage with automated E2E tests. |
| [NOMS-022](issues/NOMS-022-mobile-responsive-design.md) | Mobile responsive design | 🟡 In Progress | User experience | Implemented mobile layouts across existing pages, preserving neumorphic components and Fredoka/Nunito. Browser-verified with 65 captured states and 506 focused UI tests; [PR #6](https://github.com/SamKoCPSC/Noms/pull/6) awaiting review and merge. |

## Conventions

- **Issue IDs:** `NOMS-NNN` — stable references for linking from commits, PRs, or DESIGN.md
- **Small items** live entirely in the table above (one-liner notes are fine)
- **Complex items** get their own file under `issues/NOMS-NNN-slug.md` with acceptance criteria and technical details
- **Phases** map to the implementation priority defined in DESIGN.md (Phase 1–6)
