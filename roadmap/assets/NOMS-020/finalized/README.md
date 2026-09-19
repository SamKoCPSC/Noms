# Finalized proposals — selected implementation baselines

[Catalog](../README.md) · [Implementation plan](../../../implementation-plans/NOMS-020-visual-design-refinement.md)

These are user-selected directions, not pixel-perfect specifications. Notes and existing behavior take precedence over generated artwork. Mobile/tablet adaptations will be discovered per page during or after its desktop work, as recorded in the implementation plan on 2026-09-17; this supersedes earlier blanket deferrals in historical notes. Dark-mode redesign remains deferred. P1/P8 and N1/N4 are selected combinations, not approval of every feature in either image.

## Brand, shell and pages

| Area | Selected artwork | Decision / review notes |
| --- | --- | --- |
| Planned logo | [S1 Rounded cream](../logo/spine-options-s1-s3-v1.md) | Canonical 2026-09-18; replaces F2; leftmost book in sheet; standalone production asset/SVG/favicon/3D model pending; use actual Fredoka |
| Shared navigation | [N1](../navigation-n01-v1.png) + [N4](../navigation-n04-v1.png) | [Mixed exploration notes](../navigation-directions-v1.md); combine N1 layout and N4 silhouette, exclude props/incidental destinations |
| Search | [S2 scopes](../search-s2-scopes-v1.png), [S2 states](../search-s2-states-v1.png) | [Notes](../search-s2-scopes-and-states-v1.md); reconcile current search contracts |
| Public pages | [Home](../public-home-cozy-v2.png), [About](../public-about-cozy-v2.png), [How-to](../public-how-to-cozy-v2.png) | [Cozy v2 notes](../public-cozy-v2-prompts.md) |
| Collections | [C3 Kitchen Albums](../collections-c3-v1.png) | [Mixed exploration notes](../collections-directions-v1.md) |
| Collections icon | [CT4 Lived-in shelves](../page-icons/collections-three-level-ct1-ct4-v3.md) | Selected 2026-09-18; bottom-right in sheet; three-level bookcase; supersedes CL1 preference; production assets and integration pending |
| Profile/settings | [P1 Kitchen Passport](../settings-profile-p01-v1.png) + [P8 Cozy Accordion](../settings-profile-p08-v1.png) | [Mixed notes](../settings-profile-directions-v1.md); disclosure behavior remains a checkpoint decision |
| Profile icon | [PB2 Rounded rim](../page-icons/profile-coral-inner-border-v3.md) | Selected 2026-09-18; center in sheet; olive landscape identity card with cream face and continuous coral inner rim, no scattered accents; production assets and integration pending |
| Settings icon | [ST4B Compact control panel](../page-icons/settings-st4b-compact-v3.md) | Selected 2026-09-18; three horizontal sliders, one rotary knob and three buttons; compact olive housing, cream face and coral inner rim; supersedes wider ST4; production assets and integration pending |
| Recipe detail/history | [R3A Soft Album](../recipe-album-r3a-v2.png) | [Notes](../recipe-album-neumorphic-v2.md) |
| History icon | [HI5 Timeline ledger + clock](../page-icons/history-hi5-ledger-clock-v1.md) | Selected 2026-09-18; cream ledger, coral timeline nodes and clock; production assets and integration pending; not a separate branching graph icon |
| Recipe editor | [E3T1 raised section panels v5](../recipe-editor-e3t1-panels-v5.png) | [Notes](../recipe-editor-e3t1-panels-v5.md); supersedes earlier E3/E3T compositions |
| Recipe editor icon | [ED3 Editing clipboard](../page-icons/editor-ed1-ed4-v1.md) | Selected 2026-09-18; bottom-left in sheet; olive clipboard, cream sheet, coral clip and pencil; production assets and integration pending |
| Image editor | [I1B coral v3](../image-editor-i1b-coral-v3.png) | [Notes](../image-editor-i1b-coral-v3.md); all controls visible |
| Dashboard | [Central Workspace v1](../dashboard-central-workspace-v1.png) | [Notes](../dashboard-central-workspace-v1.md); selected visual target, functional expansion separately gated |
| Dashboard icon | [P1 Classic pinboard](../page-icons/dashboard-pinboards-p1-p6-v3.md) | Selected 2026-09-18; top-left in sheet; active-workspace metaphor, not storage; production assets and integration pending |
| Explore | [X7C framed vertical cards](../explore-x7c-vertical-v5.png) | [Notes](../explore-vertical-cards-v5.md); 16:9 photos and adaptive columns |
| Search icon | [EX3 Lifted pages](../page-icons/explore-open-book-ex1-ex4-v1.md) | Reassigned from Explore to Search 2026-09-18; bottom-left in originally Explore-titled sheet; open book and magnifier; production assets and integration pending |
| Explore icon | [EC1 Pocket compass](../page-icons/explore-compasses-ec1-ec6-v1.md) | Selected 2026-09-18; top-left in sheet; olive circular case and hanging loop, cream dial, coral/olive needle; production assets and integration pending |
| Sign-in | [L6B guest/account access](../signin-l6b-guest-access-v4.png) | [Notes](../signin-l6b-guest-access-v4.md) |

## Overlay components — M4

[M4 Warm Frosted Glass](../modals-m4-warm-glass-v1.png) is the selected modal design and general overlay material direction ([decision and notes](../modals-m4-warm-glass-v1.md)). Use warm translucent ivory, blur, fine borders and floating shadows for overlays; retain readable, more opaque fields and content. Menus/popovers/tooltips/toasts inherit the surface treatment but still need their own layout/state specifications. M4 supersedes M2. Non-overlay page surfaces retain neumorphism.

### Tooltips and toasts — T2 Soft Capsule

[T2 artwork](../overlays-t2-soft-capsule-v1.png) and [decision/behavior notes](../tooltip-toast-directions-v1.md). Selected for its softer rounded silhouette and small toast icon wells, retaining M4 warm glass and actual Nunito typography. T1/T3 are not selected. Positioning, timing, stacking, long-text handling and accessibility still require implementation specification; backgrounds and enlarged presentation scale are not UI requirements.

### Menus and collection picker — overlay P1–P3

Selected 2026-09-17: [P1 Account menu](../overlays-p1-account-menu-v1.png), [P2 Action menus](../overlays-p2-action-menus-v1.png), and [P3 Collection picker](../overlays-p3-collection-picker-v1.png). Follow the [decision and behavior notes](../menu-popover-examples-v1.md). Preserve grouped actions, permission-dependent edit/delete, exact graph actions, independent collection choices and explicit additive confirmation. Search, existing memberships, partial failure, positioning and keyboard details still need implementation contracts. These P labels refer to overlays, not profile/settings boards.

## Version graph — G4A

The integrated page is the primary composition reference. Component studies explain mutually exclusive states, not extra simultaneous page sections.

| Reference | Artwork | Notes |
| --- | --- | --- |
| Full desktop light page | [Integrated v1](../g4a-integrated-page-light-v1.png) | [Composition/review](../g4a-integrated-page-light-v1.md) |
| Base G4A | [Balanced Cards](../version-graph-g4a-v4.png) | [Original selection](../version-graph-g4-variations-v4.md); integrated page adds results |
| Selection | [Snapshot menus](../g4a-selection-states-v1.png) | [State contracts](../g4a-selection-comparison-states-v1.md) |
| Ready/loading | [Comparison states](../g4a-comparison-ready-v1.png) | Same state contracts |
| Results | [Success/no differences/failure](../g4a-comparison-results-v1.png) | Same state contracts; alternative states are separate |
| Density | [Dense graph](../g4a-dense-graph-v1.png) | [Density/expansion notes](../g4a-dense-expansion-states-v1.md); correct connector crossing in implementation |
| Expansion loading | [Loading](../g4a-expansion-loading-v1.png) | Same expansion notes |
| Expansion failure | [Retry](../g4a-expansion-error-v1.png) | Same expansion notes |

## Limits of approval

The current app's typography, colors and cozy neumorphic personality take precedence over generated styling. Preserve actual Fredoka/Nunito and existing palette/accent roles; use these proposals primarily for layout, hierarchy and component composition. Consolidate existing tokens rather than replacing the identity. Material visual-identity changes require user review.

Use actual data, fonts and the selected S1 logo reference. Earlier mockups' A3/F2 marks are superseded. Preserve omitted actions, permission boundaries and accessibility. Exact tokens, async race protection, routing, keyboard behavior and viewport constraints must be validated in implementation. No new designs need to be generated merely to begin the shared-foundation slice.
