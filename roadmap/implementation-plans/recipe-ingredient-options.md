# Recipe ingredient authoring options

Authors can mark ingredients optional, add free text before or after the ingredient name, and list explicit substitutes. Substitutes have independent names, vocabulary references, amounts, units, optional flags, and text modifiers. Alternatives are a flat list; a substitute does not have its own nested substitutes.

The shared create/edit editor keeps optional status beside amount and unit. A compact Modifiers disclosure shares the ingredient input row and opens before/after text with the helper “add extra details about the ingredient”. The final display text stays visible below. An independent single-line Substitutions disclosure follows, showing “add alternative ingredients” only when expanded. On mobile, Modifiers sits beside the ingredient name, with amount, unit and Optional together below. Substitution rows use the same layout. The displayed preview preserves ingredient identity separately from brands and preparation instructions. Selecting a vocabulary suggestion changes only that ingredient's name/reference. Typing a custom name clears only that reference.

Primary and substitution units share the existing unit autocomplete, canonicalization, and validation policy.

The existing ingredient JSON gains `optional`, `text_before`, `text_after`, and `substitutes`. Missing fields default to false, empty strings, and an empty list, and default values are omitted when serialized. Existing recipe snapshots and portable JSON keep their original shape. No database migration is needed: the existing ingredient JSON, draft, publication, history, fork, and export paths carry the fields.

Server validation permits at most ten substitutes per primary ingredient, 500 characters per modifier, and the existing name/amount/unit bounds on every alternative. Drafts can retain unnamed alternatives while editing. Publication requires a name for each substitute and for its primary ingredient. The existing aggregate snapshot size limit remains in effect. Decoded history memory accounting includes modifiers and alternative allocations.

Recipe details, historical versions, mobile cooking views, session scaling, and HTML reading copies display the authored metadata. Substitute quantities scale independently using their authored amounts. JSON exports retain all structured fields.

Kitchen behavior is intentionally unchanged in this phase: pantry matching still checks every primary ingredient's name, including optional ingredients; alternatives do not satisfy pantry checks. Grocery lists still copy primary names and quantities as before. A later change can explicitly define optional and substitute handling in those workflows.

Validation covers legacy JSON shape, structured export/import, draft/publish validation, immutable history/fork preservation, unchanged pantry matching, and the real create/edit browser workflow at mobile and desktop widths.

The ingredient editor has a local Done/Edit workflow. Done collapses a complete ingredient to its authored display line and substitution count; Edit reopens every field. Complete existing rows start compact, while new or incomplete rows start open. Substitution rows can be confirmed independently. Names and supported units are required to confirm each primary and alternative; quantities retain their existing optional semantics. Confirmation is presentation state, separate from draft saving and publication, and does not change stored JSON. Removing an earlier row cannot apply its confirmation to a different ingredient.

Compact rows place pencil and filled destructive delete icons at the top right. The visible controls are 32px with 44px hit areas and accessible labels; expanded rows use the same delete treatment. Delete inherits the shared danger button styling, including its raised material rim around the inset fill.

Confirmed rows use 6px vertical padding and 6px list gaps, with tighter ingredient panel spacing. Icon controls retain their 44px hit areas.
