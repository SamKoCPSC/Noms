# NOMS-025: Collection import name conflict recovery

**Status:** ✅ Done — editable import preview delivered

**Parent:** [NOMS-016](../implementation-plans/NOMS-016-recipe-import-export.md)

**Delivered:** 2026-11-14 ([PR #80](https://github.com/SamKoCPSC/Noms/pull/80))

Collection JSON import allows the user to rename imported collections and recipes
in the preview and retry with the original file. This resolves names already in
the importing account's library without editing a checksum-protected archive.

- The preview flags existing names, shows editable collection names and recipe
  titles, and retains the choices after a rejected import.
- The server verifies the original archive and nested recipe checksums before
  applying bounded name overrides. Name checks follow the importing owner's
  normalized namespace. Automatic root-copy naming also accounts for names
  selected for imported children.
- A conflict that appears after preview is handled during save. Rejected imports
  roll back all writes. The export format and schema remain compatible.

The PR records 141 passing collection tests with three existing ignores. Tests
cover checksum enforcement, invalid overrides, owner scope, late conflicts and
preview inputs. The component was rendered with synthetic data. The live
authenticated browser flow remained unverified in that run.
