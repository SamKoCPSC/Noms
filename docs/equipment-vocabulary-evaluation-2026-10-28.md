# Cooking equipment vocabulary evaluation — 2026-10-28

Wikidata is a useful source of identifiers and candidate names, but its equipment
hierarchy is not a ready-to-import kitchen vocabulary. This research produces an
initial shortlist and explicit review queues. It adds no production seed, schema,
API, autocomplete, recipe edits or historical-data migration.

## Capture and scope

Discovery follows both `subclass of` and `instance of/subclass of` paths beneath
[culinary equipment](https://www.wikidata.org/wiki/Q26037047),
[kitchenware](https://www.wikidata.org/wiki/Q1521410),
[kitchen utensil](https://www.wikidata.org/wiki/Q3773693) and
[cookware and bakeware](https://www.wikidata.org/wiki/Q154038).
Queries run separately per root to avoid the combined query's timeouts.

| Stage | Records |
| --- | ---: |
| Distinct discovered item IDs | 25,455 |
| Detailed, revision-pinned entity capture | 2,312 |
| Captured entities with English labels | 2,058 |
| Captured entities without English labels | 254 |
| Captured entities without English descriptions | 505 |
| Instance-only records outside the detailed capture | 23,143 |

The detailed selection is the union of 2,114 equipment-class IDs and 221
English-Wikipedia-linked instance IDs; 23 occur in both groups. The other
23,143 records remain in the raw discovery archive. They were not individually
label-reviewed or declared unsuitable: that group can include individual museum
objects and other instances rather than reusable equipment types. Useful types
can also be absent from these hierarchies, as the targeted follow-up demonstrates.

Each detailed entity has a pinned revision, revision URL, English label,
description, aliases and selected ranked semantic claims. Deprecated claims do
not define truthy relationships. The snapshot is 2,662,397 bytes with SHA-256
`d8a0111adfd8c40dbcf7ed8ad041a1e6f67b5074bf30b677b9ad3c6e4a3f5c1a`.
Discovery and entity revisions are not one atomic global dump. Structured
[Wikidata data is CC0](https://www.wikidata.org/wiki/Wikidata:Licensing);
Wikipedia article prose and images are not imported.

## Candidate screening

All 2,058 English labels were screened. Available definitions were checked for
the explicit shortlist; 15 shortlisted records lack English descriptions and
still need stronger definition evidence before seeding. Other records receive
conservative, policy-assisted dispositions.
This is an initial editorial shortlist, not semantic certification of all records
and not approval to import source aliases.

| Disposition | Records |
| --- | ---: |
| Core candidates | 142 |
| Specialist/regional candidates | 94 |
| Supplies/safety items requiring a scope decision | 13 |
| Explicitly excluded by scope rules | 167 |
| Held for further identity, translation or scope review | 1,896 |

The 236 equipment candidates cover cookware, bakeware, preparation tools,
appliances, measuring tools, storage and bar tools. Examples include frying pans,
stockpots, food processors, immersion blenders, bannetons, tamagoyaki pans,
donabe, karahi and suribachi. The separate 13-item supply/safety queue includes
parchment paper, aluminium foil, cheesecloth, skewers, oven mitts and pot holders.

There are 42 proposed preferred-label changes, including `stock pot` →
`stockpot`, `sheet pan` → `baking sheet`, `makiyakinabe` → `tamagoyaki pan`,
`lame` → `bread lame`, and `Spatula kitchen` → `spatula`. These are proposals;
they do not establish equivalence between separate source records.

Scope noise includes Olympic cauldrons, ceremonial drinking vessels, cosmetic
and laboratory tools, brands, broad catalogue groups and replacement parts.
Some functional attachments, such as dough hooks and steamer baskets, remain
useful candidates because recipes can require them. The policy does not reject
every accessory indiscriminately.

## Identity and naming problems

- **29 duplicate primary-label groups**, including cake pans, carving forks,
  corkscrews, sandwich makers, teapots and trivets. Separate IDs remain separate.
- **122 shared label/alias groups** require review. None are automatically merged.
- The generic **mixer** item lists **stand mixer** as an alias. That must not
  erase the distinction between stand and hand mixers.
- **Pasta machine/pasta maker**, **stockpot/soup pot**, and **banneton/dough
  basket** need definition-level reconciliation rather than text deduplication.
- The hierarchy's **hot-air deep fryer** and the separately found **air fryer**
  item need reconciliation before choosing a canonical identity.
- **Bain-marie** has **double boiler** as a source alias; a general heated bath
  does not by itself establish the identity of nested double-boiler vessels.
- Two plausible labels were removed from the shortlist after checking meaning:
  **clay pot** is described as a storage vessel with amphora/pitcher aliases;
  **hibachi** is described as a Japanese heating device, not a cooking grill.

## Coverage checks

These are three overlapping checklists, not random recipe samples. The match
counts use exact normalized source labels/aliases and proposed shortlist labels;
they do not prove exact semantic coverage. Editorial-only name matches are
reported separately in `coverage.csv`. Matching normalizes case, Unicode and
whitespace; it does not guess missing plurals or hyphen variants. For example,
`measuring spoons` and `potato-ricer` expose naming gaps rather than missing tools.

| Checklist | Rows | Shortlist name match | Other source name match | No name match |
| --- | ---: | ---: | ---: | ---: |
| Noms common-kitchen checklist | 141 | 111 | 1 | 29 |
| Wikibooks equipment pages | 127 | 76 | 13 | 38 |
| PRISM cooking equipment terms | 21 | 17 | 0 | 4 |

The common-kitchen checklist has 107 raw source-name/alias hits, plus five
editorial-only matches. One raw hit points only to an unshortlisted identity.
Even a shortlisted hit can be too broad, as with the stand-mixer alias above.

The [Wikibooks equipment categories](https://en.wikibooks.org/wiki/Category:Cookbook_equipment)
provide 130 unique cookbook page titles; the equipment index and two policy
templates are excluded from comparison. Titles are retained only as a reference
checklist, with CC BY-SA attribution. The
[PRISM specification, section 5.2](https://www.w3.org/submissions/2020/SUBM-prism-20200910/prism-cvs.html)
provides 21 term identifiers under its documented W3C Document License; its
definitions are not copied into a seed. PRISM is a W3C Member Submission, not a
W3C Recommendation. Princeton WordNet was not ingested in this pass.

The 30 unresolved common-checklist rows have individual notes in `gap-review.json`:

| Gap classification | Rows | Examples |
| --- | ---: | --- |
| Found outside this hierarchy capture | 8 | Kitchen scales, cooling rack, convection oven, refrigerator, freezer, ice cream maker, food dehydrator, plastic wrap |
| Likely naming/identity review | 6 | Spider strainer, vegetable peeler, roasting rack, proofing basket, tagine, sushi rolling mat |
| Only broader or grouped evidence confirmed | 10 | Instant-read/probe thermometers, hand mixer, silicone spatula, nonstick frying pan, silicone baking mat, soufflé dish and generic tongs |
| No specific item confirmed in this pass | 6 | Oven thermometer, Bundt pan, piping tip, canning funnel, canning tongs and kitchen twine |

Targeted searches are also archived: no search result is not proof that Wikidata
lacks a concept. Thirteen supplemental equipment-related entities were pinned separately;
they do not alter the original hierarchy denominator or shortlist counts.

## Recommendation

Use the 236 equipment candidates as the starting review pool, then resolve the
identity cases and incorporate appropriate supplemental concepts. Review and add
natural aliases explicitly, and curate remaining familiar types where upstream
classification or specificity is inadequate. Decide the 13-item supply/safety
scope separately. Keep useful functional and regional distinctions; neither
shared names nor ancestry imply substitutability.

Only after that review should a canonical seed receive stable Noms identifiers,
approved aliases and source mappings. Existing authored equipment text and
historical snapshots should remain unchanged.

## Verification and artifacts

- Four focused Python tests cover offline evaluation, snapshot drift,
  duplicate/unknown review identities, invalid English labels, exact name
  matching, editorial-name provenance and deterministic output.
- All 2,312 entities were restored from the immutable revision cache with
  network access disabled; the snapshot checksum matched exactly.
- The complete evaluation can be regenerated from the pinned snapshot,
  descriptor, review policy and benchmark. Byte-for-byte replay is checked.
- The downloadable research bundle contains all candidate CSVs, shortlisted
  candidates, coverage and duplicate queues, source snapshots and descriptors,
  raw discovery queries/results, reference snapshots and attribution, the gap
  ledger, scripts and this report. Large generated files stay outside Git.

Reproduction commands are in
[the equipment research guide](../scripts/vocabulary/EQUIPMENT.md).
