# Noms village — single-file mobile preview

[Download noms-village-mobile.zip](https://github.com/SamKoCPSC/Noms/raw/refs/heads/preview/terraced-village-download/noms-village-mobile.zip)

1. Download the ZIP above (about 71 MiB).
2. Extract **OPEN-VILLAGE-MOBILE.html**.
3. Open that HTML in Chrome. All twelve models are inside that file; there is no neighboring assets folder to load.

If you use **Code → Download ZIP**, extract the repository archive, then extract the `noms-village-mobile.zip` inside it.

This replaces the earlier multi-file download, which could show “Failed to fetch” when Android opened its HTML through a `content://` file viewer without access to neighboring model scripts.

Geometry bytes and texture dimensions are unchanged. The download uses WebP quality 95 compression for preview textures; original production assets and Blender masters remain unchanged. Conversion receipts are included in this branch.

The scene remains a large twelve-model layout study. Loading all models together may require substantial phone memory. Terrain and paths are provisional, and shared lighting has not been rebaked.

Source and Blender world: [PR #89](https://github.com/SamKoCPSC/Noms/pull/89).

Validation: the single-file HTML loads all twelve models in portrait browser testing with additional HTTP/HTTPS requests blocked. Direct Android `content://` launching is not available in the test environment and still needs checking on the device.
