# NOMS-016: Recipe Import & Export — Implementation Plan

**Status:** Placeholder — discovery and implementation planning required
**Created:** 2026-08-15
**Depends on:** Current recipe, draft, versioning, collection, and image models
**Coordination:** NOMS-017 structured metadata must define how imported and exported taxonomy fields are represented.
**Approach:** To be finalized after the supported import sources, portable export contract, security boundaries, and failure behavior are validated against the current application.

---

## Intended outcome

Users can bring recipes into Noms without retyping them and can take their recipe data out in durable, documented formats.

## Proposed planning scope

- Import recipes from supported URLs, prioritizing Schema.org `Recipe` JSON-LD.
- Present parsed data for review and correction before saving it as a draft.
- Define safe fetch, parsing, size, timeout, redirect, and attribution rules.
- Export individual recipes and collections in a documented structured format.
- Provide print-friendly and PDF-oriented output without making PDF the canonical data format.
- Define image portability and ownership behavior without copying third-party media implicitly.

## Questions to resolve

- Which URL sources and fallback parsing strategies are supported initially?
- Which fields must round-trip without loss across import and export?
- How are versions, forks, collections, and locally owned images represented?
- What rate limits and SSRF protections apply to server-side URL fetching?
- Which export formats are required for the first release?

## Required implementation-plan deliverables

- Current-state code and schema baseline.
- Import and export contracts with explicit trust boundaries.
- Test-first checkpoints, migrations, UI states, and rollback considerations.
- Compatibility rules for NOMS-017 metadata.
- Automated, manual, security, and portability verification matrix.

## Definition of ready

- [ ] Product scope and supported formats are approved.
- [ ] Import security and resource limits are specified.
- [ ] The canonical portable recipe schema is documented.
- [ ] Checkpoints, acceptance evidence, and release gates are defined.

