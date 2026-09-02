# NOMS-013 Content Inventory

**Drafted:** 2026-08-15
**Status:** First product-truth pass

This inventory connects proposed marketing claims to behavior present in the current application. Copy marked `available` may be used on public pages after a final manual replay. Copy marked `future` must appear only in a clearly labelled future-facing section and must not link to an unfinished workflow.

## Voice principles

- Warm and useful, not cute for its own sake.
- Specific about what Noms helps someone do; avoid generic claims such as “revolutionize your cooking.”
- Treat recipes as working knowledge, not merely content to collect.
- Prefer short sentences and familiar cooking language over software terminology.
- Use “you” sparingly and naturally. Avoid invented user counts, testimonials, founder history, or community scale.
- Say “version history,” “compare,” and “restore” in product instructions. Use “keep improving” in higher-level marketing copy.
- Use “fork” only where the interface uses that label; explain it in plain language the first time.

## Claim inventory

| Page/section | Proposed claim | Availability | Evidence | CTA | Review result |
|---|---|---|---|---|---|
| Home hero | Capture the details that make a recipe yours, then keep improving it over time. | available | `/recipes/new`, recipe editor, published version history | Signed out: `/login?redirect_uri=/recipes/new`; signed in: `/dashboard` | Code verified; manual replay pending |
| Home / Capture | Save ingredients, nested instructions, timing, servings, equipment, tags, commentary, and photos. | available | `/recipes/new`; editor state and image upload UI | `/recipes/new` | Code verified; manual replay pending |
| Home / Drafts | New recipes and edits can be kept as private, autosaved work in progress until publication. | available | new/edit recipe autosave flows; Dashboard Drafts section | `/dashboard` | Code verified; manual replay pending |
| Home / Organize | Group recipes into collections and nested subcollections. | available | `/collections`, `/collections/new`, child collection creation | `/collections` | Code verified; manual replay pending |
| Home / Organize | Reorder recipes and child collections to keep a library arranged your way. | available | collection detail reorder controls and endpoints | `/collections` | Code verified; manual replay pending |
| Home / Evolve | Every published update becomes part of a recipe’s version history. | available | recipe detail Version history | Recipe detail, context-specific | Code verified; manual replay pending |
| Home / Evolve | Compare published versions, inspect snapshots, and restore an earlier version without erasing history. | available | recipe history comparison, snapshot route, Restore action | Recipe detail, context-specific | Code verified; manual replay pending |
| Home / Cook | Scale ingredients, servings, and timing while viewing a recipe. | available | recipe detail scaler | Recipe detail, context-specific | Code verified; manual replay pending |
| Home / Share | Keep a recipe private, share it by unlisted link, or publish it to Explore. | available | recipe visibility options and public recipe endpoint | `/explore` | Code verified; manual replay pending |
| Home / Discover | Browse public recipes and filter the recipes currently loaded by title or description. | available | `/explore` | `/explore` | Code verified; manual replay pending |
| Home / Community | Visit creator profiles with their public recipes and public collections. | available | `/u/:username` | `/explore` | Code verified; manual replay pending |
| Home / Remix | Fork a recipe into your own private draft and keep attribution to its source. | available | recipe detail Fork action, fork lineage, draft redirect | Recipe detail, context-specific | Code verified; manual replay pending |
| About / Principles | Recipes change as people cook them; the product preserves published history instead of treating every edit as an overwrite. | available | version history, comparison, restore | `/how-to#versions` | Code verified; manual replay pending |
| About / Control | The author chooses private, unlisted, or public visibility for recipes. | available | recipe visibility controls | `/how-to#visibility` | Code verified; manual replay pending |
| About / Control | Collections support private, unlisted, and public visibility, with shared links for non-private collections. | available | collection visibility and Copy link/Share controls | `/how-to#collections` | Code verified; manual replay pending |
| About / Future | Richer search across recipes, collections, and creators. | future | NOMS-012; no canonical `/search` route currently exists | none | Must remain future-facing |

## Claims to avoid

- “Search your entire library” or “Search recipes, collections, and people.” The current Explore filter only filters already-loaded public recipes; NOMS-012 is not present.
- “Collaborative editing,” “real-time collaboration,” or “work together on a recipe.” No shared editing workflow exists.
- “Offline access,” “sync across devices,” “import from any website,” “meal planning,” or “shopping lists.” These workflows are not verified in the current application.
- “Never lose a change.” Autosave and published history exist, but absolute-loss guarantees require operational evidence that marketing copy does not have.
- “Share private collections.” Private collections are owner-only; only non-private collections expose sharing controls.
- Claims about the number of recipes, creators, collections, or active users.

## CTA contract

| Context | Primary label | Destination | Secondary label | Destination |
|---|---|---|---|---|
| Signed out Home hero | Start creating | `/login?redirect_uri=/recipes/new` | Explore recipes | `/explore` |
| Signed in Home hero | Open dashboard | `/dashboard` | Create a recipe | `/recipes/new` |
| Home final CTA, signed out | Start your first recipe | `/login?redirect_uri=/recipes/new` | See how Noms works | `/how-to` |
| Home final CTA, signed in | Create a recipe | `/recipes/new` | Open dashboard | `/dashboard` |
| About final CTA, signed out | Explore recipes | `/explore` | Learn how it works | `/how-to` |
| About final CTA, signed in | Open your recipe library | `/dashboard` | Create a recipe | `/recipes/new` |

Implementation must use typed `Route` destinations where possible. Login redirects may use the existing safe relative `redirect_uri` contract.

## Open editorial decisions

1. Confirm the leading brand promise: “Recipes worth keeping, built to keep changing.”
2. Decide whether “Fork” belongs on Home or only on About/How-to; it is accurate but may sound technical to first-time visitors.
3. Decide how prominent public sharing should be relative to the private personal-library story.
4. Choose whether the future-facing search section is valuable enough to include on About, or whether to omit future functionality entirely.
