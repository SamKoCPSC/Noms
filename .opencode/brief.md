# Task Brief

## Task Description
Implement NOMS-009 (Versioning, Drafts & Branching) as specified in the roadmap. Create a new branch off of the current one before starting.

**Issue:** roadmap/issues/NOMS-009-versioning-drafts-branching.md
**Plan:** roadmap/implementation-plans/NOMS-009-versioning-drafts-branching.md

**4 Checkpoints to implement sequentially (each goes through architect -> implement -> review):**

### CP1: Database schema changes for versioning
- RecipeVersion model with all recipe fields + metadata (version_number, created_at, updated_at, is_current, is_published)
- version_number, is_draft, current_version_id columns on Recipe model
- Migration script

### CP2: Version management backend API
- GET /api/recipe/{id}/versions - list all versions
- POST /api/recipe/{id}/versions - create new version (auto-save current state)
- PUT /api/recipe/versions/{version_id} - update a version (draft editing)
- DELETE /api/recipe/versions/{version_id} - delete a version
- Version conflict detection

### CP3: Branching & merging backend API
- POST /api/recipe/{id}/branch - create a branch
- POST /api/recipe/branches/{branch_id}/merge - merge branch back
- GET /api/recipe/{id}/branches - list branches
- Branch model (branch_id, recipe_id, name, created_from_version_id, status)

### CP4: UI integration
- Version history panel in recipe detail page
- Draft/branch indicators in UI
- UI actions: save as draft, create version, create branch, merge branch
- Version comparison view

## Phase 0: Implementation Blueprint (CP1)
<!-- written by @develop-architect for CP1 -->

## Phase 1: Implementation Details (CP1)
<!-- written by @develop-implement for CP1 -->

## Phase 2: Review Verdict (CP1)
<!-- written by @develop-review for CP1 -->

## Phase 3: Synthesis (CP1)
<!-- written by @develop-synthesize for CP1 -->

---

## CP1 Results
<!-- summary of CP1 after completion -->

## CP2 Results
<!-- summary of CP2 after completion -->

## CP3 Results
<!-- summary of CP3 after completion -->

## CP4 Results
<!-- summary of CP4 after completion -->

## Final Synthesis
<!-- final summary and commit message -->
