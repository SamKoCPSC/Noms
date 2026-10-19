# NOMS-016: Standalone HTML export

The saved-recipe export dialog offers two formats with explicit purpose and limitations:

| Format | Purpose | Images and limitations |
| --- | --- | --- |
| JSON | Structured backup and re-import into Noms | Canonical v3 image references and edit metadata remain intact. Accessible images can be reused on import; unavailable ones need manual recovery. The optional ZIP also contains saved original image files. |
| HTML | Read, share, and print an offline recipe | A single file with inline CSS and optional embedded saved originals. It cannot be re-imported, edited in place through Noms, or used as a history backup. |

HTML follows the app's warm cream, coral, and sage palette, rounded cards and serif headings. System fonts avoid network dependencies. Ingredients, equipment, nested instructions and step images, tags, times, servings, author and notes come from the current saved recipe. Drafts and history are excluded. Browser printing provides paper/PDF output with simplified styling.

Saved images are included by default; unchecking the option creates a text-only file. External URLs are never fetched. Inaccessible, unsupported, oversized or timed-out images produce a per-placement warning before download; the user must explicitly choose **Download HTML anyway**. The downloaded file retains the same omission notice. Crops and adjustments are not baked into embedded originals, and this limitation appears both in the dialog and the HTML footer.

The server reuses the authorized, consistent-snapshot JSON export read. Private recipe access is checked before storage reads. Only managed full-size recipe WebP objects in configured storage are eligible. Retrieval is limited to 10 MiB per object, three seconds per object and fifteen seconds overall, within existing database and expensive-read concurrency budgets. A URL is fetched once, while every repeated embedding counts against the 32 MiB file limit. Text-only export makes no image-storage requests.

All authored text is escaped; the HTML includes no script or remote resource links. Its content security policy permits only embedded image data and inline styles, with forms and other resource loads blocked. Original external image URLs do not appear in the reading copy. JSON remains the canonical portability format and its image-reference behavior is unchanged.

The WebAssembly download helper is written in Rust using `web-sys`, `gloo-net` and `gloo-timers`, without generating or evaluating JavaScript source. JSON and HTML become typed browser Blobs; ZIP responses are fetched with same-origin credentials and read directly into a browser Blob without copying the archive through WebAssembly memory. Failed responses do not start a download. Temporary anchors are removed immediately, and object URLs are revoked after a one-second grace period or immediately if preparing the anchor fails. Cleanup runs independently of the export dialog's lifetime. Browser bindings are compiled only for WebAssembly; server-side rendering does not execute download handlers, and native clients require a separate download implementation.

## Verification

Focused native tests cover access checks before storage reads, public/unlisted/owner access, text-only export, repeated image deduplication and size accounting, nested content, unavailable images, external-link omission, and hostile authored text. Existing JSON and ZIP regressions are also exercised.

Validation on 2026-10-19: all 20 focused HTML/JSON/ZIP native tests passed (three HTML, fifteen JSON, two ZIP), as did formatting, server and WebAssembly Clippy, and the repository security audit. Clippy used the existing Rust 1.99 compatibility exception for the unrelated `chunks_exact_to_as_chunks` rule. The audit reported only the existing eight allowed warnings; no dependency version was changed. This is focused verification, not a new full-suite or cross-browser acceptance claim.

Chromium exercised the actual app's HTML download with an embedded image, text-only HTML, omission review before download, retained warnings in the downloaded copy, canonical v3 JSON and saved-image ZIP downloads. The downloaded HTML bytes were loaded directly into a document with browser networking disabled: all images decoded, no external requests occurred, desktop and 390px layouts had no horizontal overflow, and A4 PDF output was inspected on both pages. Direct `file:` and `data:` navigation are blocked by this environment's managed browser policy, so opening through the operating system's file association was not tested. The app reported no browser page errors.

After converting the download helper to Rust, server and WebAssembly Clippy, formatting and the fullstack app build passed. Chromium reverified real JSON, embedded-image HTML, text-only HTML and ZIP downloads, including filenames, MIME types, Unicode text and ZIP integrity. A simulated anchor-attachment error revoked its object URL immediately and allowed retry; a simulated ZIP HTTP 503 created neither a file nor an object URL and also allowed retry. Same-origin cookies accompanied the ZIP request. All five object URLs created across these checks were revoked, temporary anchors were removed, and no browser page errors occurred. No additional dependency version was introduced.

Synthetic recipes and a synthetic illustration were used in an isolated temporary PostgreSQL database; no existing recipe data was modified. The synthetic warning recipe's visibility was corrected to public after its canonical-import helper correctly defaulted to private. An initial fake-storage query-string handling error was also corrected before the successful final browser run. Visual inspection found no clipping or overlapping content in the final captures.

Screenshot evidence:

- [JSON purpose and limitations](../assets/html-export/dialog-json.png)
- [HTML purpose, limitations, and image option](../assets/html-export/dialog-html.png)
- [Mobile export dialog](../assets/html-export/dialog-mobile.png)
- [Explicit image-omission confirmation](../assets/html-export/dialog-omissions.png)
- [Desktop offline reading copy](../assets/html-export/offline-desktop.png)
- [Mobile offline reading copy](../assets/html-export/offline-mobile.png)
- [Print page 1](../assets/html-export/print-1.png) and [page 2](../assets/html-export/print-2.png)
