# Dashboard, Explore, sign-in and version graph — exploration briefs

2026-09-15. Planning only; no images generated in this pass and no application code changes. None of the directions below is selected. Source review used current main's `src/pages/dashboard.rs`, `explore.rs`, `login.rs` and `recipe_version_graph.rs`; browser validation of representative populated states remains required before implementation.

## Shared constraints

Subsequent evidence: see the [2026-09-15 browser audit and corrected proposals](page-audit-and-revised-proposals-v2.md). This qualifies the initial source review below, including Explore's unwired tag filtering, Dashboard pagination, graph interaction semantics and incomplete state coverage.

Use Kitchen Console cream, sage/olive and restrained coral, Fredoka/Nunito, the approved A3 book mark and N1/N4 navigation. Raised cards contain grouped content; fields/canvases are inset and reading areas calm. Preserve actual behavior and accessible state labels, not shadow/color alone. Begin with desktop; new mobile/tablet exploration remains deferred. Distinguish proposals from approved features. Use consistent sample content within each page's alternatives for fair comparison.

## Dashboard — personal recipe workspace

Current source includes My Recipes, New Recipe, private autosaved drafts with revision/source information, published recipe cards, a CollectionSidebar and load-more/count feedback. Keep drafts distinct from published recipes; published does not mean public. Do not invent activity feeds, cooking statistics or recommendations.

- **D1 — Kitchen Desk:** prominent raised draft section above the recipe grid, with a compact C3-inspired collection panel alongside. The strongest continuation of E3T1's working-document feel.
- **D2 — Personal Recipe Albums:** a compact collection overview above clearly separated draft and published grids. Reorganizes existing collection access into album imagery, without inventing collection ranking or recency.
- **D3 — Quiet Library:** compact draft rows, generous published recipe grid and a narrow collection index. Less decorative and more efficient for large libraries.

Recommend exploring D1 first. Compare populated, drafts-only and first-use states. Also specify partial loading/failure of drafts versus recipes versus collections; preserve available content when another region fails. No arbitrary timestamps or “recent” ranking unless supported by data.

## Explore — public discovery

Current source exposes public recipe cards, text search, selectable tag chips and Load more. This is discovery of public recipes, not the multi-scope Search page. No private drafts, invented ratings, trending ranks, editorial curation or follow controls.

- **X1 — Community Recipe Album:** photo-led raised cream grid beneath a compact S2-inspired inset search and tag strip; warm introduction without a large hero.
- **X2 — Sage Discovery Shelf:** same results and controls, with a muted sage header band framing the search area and cream recipe cards below. Stronger page identity with unchanged retrieval behavior.
- **X3 — Quiet Recipe Index:** horizontal photo-and-text cards with more room for existing descriptions and metadata; compact search/tag controls above. Prioritizes scanning over large imagery.

Recommend X1 first. Compare populated, selected-tag, no-match, no-public-content, initial loading and incremental loading/error states. Preserve current tag/filter semantics; any desired behavior change belongs in separate feature work. Creator links/metadata should appear only where actual card data supports them.

## Sign-in — welcoming but direct

Current page provides Continue with Google, Continue with GitHub, Back to home and a validated return destination. Do not introduce passwords, email registration, new providers or legal consent text.

- **L1 — Welcome Bookplate:** centered softly raised cream card, modest A3 logo, concise welcome copy and two equal-prominence provider buttons. Quietest option.
- **L2 — Cozy Kitchen Welcome:** restrained book/kitchen illustration beside a compact raised provider card; reuse public-page character without distracting from sign-in.
- **L3 — Sage Welcome Panel:** centered cream sign-in card within a broader sage panel, subtle book motif and minimal supporting copy. More color, little illustration.

Recommend L1 for clarity; L2 is the more characterful alternative. Plan default, leaving-for-provider, cancelled/failed authentication and return-to-requested-page messaging. Treat new feedback rendering as a proposed state requirement, not an already verified feature. Preserve provider identity, redirect safeguards and keyboard access; no fake provider-branded artwork in production.

## Version graph — readable recipe lineage

Current source includes distinct version-history and fork relationships, pan/zoom/Fit, graph expansion, lineage filtering, snapshot actions and comparison selection. NOMS-010 owns semantics. Published versions and forks must not become generic merge branches, contributor lanes or a chronological list that loses source relationships. Private drafts must not be exposed by illustrative nodes.

- **G1 — Soft Lineage Canvas:** inset neutral canvas with compact raised snapshot cards; sage selected node, restrained coral comparison marker, and a detached details panel plus comparison area. Closest to the selected album/editor system.
- **G2 — Recipe Family Lanes:** group nodes into clearly labelled recipe lineages, with explicit fork-source connectors between lanes. Lanes represent recipes, not authors; version order and relationships remain intact. Avoid implying merge support.
- **G3 — Guided Lineage Focus:** visually emphasize the selected snapshot's ancestors/descendants using existing lineage filtering, with a clear Show full graph action. Compact cards and a raised details panel reduce clutter without silently hiding relationships.

Recommend G1 as the baseline, then compare G2/G3 on a dense example. Use the same sample graph for all three: one recipe with v1–v4 and two distinct recipes forked from labelled source snapshots. Mark Current, Selected and comparison A/B independently. Show fork edges distinctly from version edges, including a text legend. Add a dense/partially expanded state with Show next 5, loading/failure of expansions, and no-data/unavailable states. Keep zoom/Fit and comparison actions reachable without covering nodes. Validate selection, cross-recipe comparison and visibility rules against actual behavior; generated diagrams are never semantic specifications.

## Generated examples

The [12 desktop concepts and review notes](page-studies-dashboard-explore-signin-graph-v1.md) were generated on 2026-09-15. No direction is selected yet. The notes flag generated copy, navigation and graph-edge inaccuracies that must not become implementation requirements.

## Next deliverables

Generate three comparable desktop concepts per page (12 total) when requested, labelled D1–D3, X1–X3, L1–L3 and G1–G3. Select a composition for each, then refine important states and dark mode rather than treating every exploratory board as approved. Record choices in the canonical NOMS-020 plan. Implementation checkpoints and responsive acceptance criteria follow selection and state validation.
