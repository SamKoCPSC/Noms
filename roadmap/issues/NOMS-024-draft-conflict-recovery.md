# NOMS-024: Recipe draft conflict recovery

**Status:** ✅ Done — revision-guarded review and merge delivered

**Parent:** [NOMS-010](NOMS-010-versioning-drafts-branching.md)

**Delivered:** 2026-11-12 ([PR #77](https://github.com/SamKoCPSC/Noms/pull/77))

Autosave used to replay a stale editor snapshot after a revision conflict, which
could overwrite another tab's changes. Both recipe editors now keep the last
confirmed snapshot with its revision and pause autosave while a conflict is open.

- Three-way review compares the common base, local edits and latest saved draft.
  Independent edits are suggested automatically. Competing edits require a choice.
- Ingredients, steps and their photos, equipment, gallery photos and tags are
  reviewed as whole ordered lists. Pending gallery files must be uploaded or
  removed before review, and the gallery reloads from the merged result.
- Confirmation saves against the reviewed revision and resumes autosave. A newer
  write refreshes the comparison and requires confirmation again. Publication
  remains a separate action.

The PR records 30 focused tests, a passing full test job, and desktop/mobile
component previews. Authenticated multi-tab browser replay remained pending.
The audit and image-build failures noted on the original run were tracked
separately and addressed in [PR #79](https://github.com/SamKoCPSC/Noms/pull/79).
