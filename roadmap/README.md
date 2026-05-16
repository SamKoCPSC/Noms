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

## Conventions

- **Issue IDs:** `NOMS-NNN` — stable references for linking from commits, PRs, or DESIGN.md
- **Small items** live entirely in the table above (one-liner notes are fine)
- **Complex items** get their own file under `issues/NOMS-NNN-slug.md` with acceptance criteria and technical details
- **Phases** map to the implementation priority defined in DESIGN.md (Phase 1–6)
