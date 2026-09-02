import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  loadAndValidateFixtureCorpus,
  validateImportCandidate,
  validateJsonSchema,
} from "../validate-recipe-import-fixtures.mjs";

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const fixtureRoot = path.join(
  repositoryRoot,
  "tests",
  "fixtures",
  "recipe-import",
  "v1",
);

let parseRecipeClipboard;
let parserLoadError;
try {
  ({ parseRecipeClipboard } = await import("../recipe-import-parser.mjs"));
} catch (error) {
  parserLoadError = error;
}

const corpus = await loadAndValidateFixtureCorpus(fixtureRoot);

for (const { fixture, input, expected } of corpus.fixtures) {
  test(`parses ${fixture.id} into its reviewed candidate`, async () => {
    assert.ifError(parserLoadError);
    assert.equal(
      typeof parseRecipeClipboard,
      "function",
      "scripts/recipe-import-parser.mjs must export parseRecipeClipboard(input)",
    );

    const firstCandidate = await parseRecipeClipboard(input);
    const secondCandidate = await parseRecipeClipboard(input);

    assert.deepEqual(
      secondCandidate,
      firstCandidate,
      `${fixture.id} must parse deterministically`,
    );
    assert.deepEqual(
      validateJsonSchema(firstCandidate, corpus.schema),
      [],
      `${fixture.id} must satisfy the versioned JSON Schema`,
    );
    assert.deepEqual(
      validateImportCandidate(firstCandidate),
      [],
      `${fixture.id} must satisfy semantic candidate invariants`,
    );
    assert.deepEqual(
      firstCandidate,
      expected,
      `${fixture.id} must preserve the reviewed extraction and warning decisions`,
    );
  });
}

test("unit-yield-and-duration-ambiguity sums legitimate repeated duration units", async () => {
  assert.ifError(parserLoadError);
  const candidate = await parseRecipeClipboard({
    html: "<article><h1>Chilled Dough</h1><div>Cook: 1 hr 30 min plus 2 hr chilling</div><h2>Ingredients</h2><ul><li>1 cup flour</li></ul><h2>Directions</h2><ol><li>Chill and bake.</li></ol></article>",
    plainText: null,
    sourceUrl: null,
  });

  assert.deepEqual(candidate.fields.cookTime, {
    raw: "Cook: 1 hr 30 min plus 2 hr chilling",
    value: 210,
    confidence: "high",
    provenance: [{ representation: "text/html", locator: "article > div:nth-of-type(1)" }],
  });
});

test("unit-yield-and-duration-ambiguity resolves associated and accessible yield selects", async () => {
  assert.ifError(parserLoadError);
  const cases = [
    {
      control: "<label for=portion-count>Yield</label><select id=portion-count><option selected>8 bowls</option></select>",
      display: "8 bowls",
      servings: 8,
    },
    {
      control: "<select aria-label=Servings><option selected>4 plates</option></select>",
      display: "4 plates",
      servings: 4,
    },
  ];

  for (const fixture of cases) {
    const candidate = await parseRecipeClipboard({
      html: `<article><h1>Portioned Soup</h1>${fixture.control}<h2>Ingredients</h2><ul><li>1 cup broth</li></ul><h2>Directions</h2><ol><li>Serve.</li></ol></article>`,
      plainText: null,
      sourceUrl: null,
    });
    assert.deepEqual(candidate.fields.yield?.value, {
      display: fixture.display,
      servings: fixture.servings,
    });
  }
});

test("unit-yield-and-duration-ambiguity preserves ambiguous visible unit lists", async () => {
  assert.ifError(parserLoadError);
  const candidate = await parseRecipeClipboard({
    html: "<article><h1>Two-System Cake</h1><h2>Ingredients</h2><ul data-unit=us><li>1 cup flour</li></ul><ul data-unit=metric><li>120 g flour</li></ul><h2>Directions</h2><ol><li>Mix.</li></ol></article>",
    plainText: null,
    sourceUrl: null,
  });

  assert.equal(candidate.status, "needs_review");
  assert.equal(candidate.fields.ingredientSections[0].items[0].raw, "1 cup flour");
  assert.deepEqual(candidate.competingCandidates, [{
    reason: "Ambiguous metric measurement variant",
    title: null,
    provenance: [{ representation: "text/html", locator: "ul[data-unit=metric]" }],
  }]);
});

test("remote-content-and-controls never parses markup-looking raw content", async () => {
  assert.ifError(parserLoadError);
  const fakeRecipe = "<article><h1>Injected Recipe</h1><h2>Ingredients</h2><ul><li>1 cup noise</li></ul><h2>Directions</h2><ol><li>Ignore.</li></ol></article>";
  const realRecipe = "<article><h1>Real Recipe</h1><h2>Ingredients</h2><ul><li>1 cup rice</li></ul><h2>Directions</h2><ol><li>Cook.</li></ol></article>";
  const closedCandidate = await parseRecipeClipboard({
    html: `<script>${fakeRecipe}</script>${realRecipe}`,
    plainText: null,
    sourceUrl: null,
  });
  assert.equal(closedCandidate.fields.title.value, "Real Recipe");
  assert.equal(closedCandidate.competingCandidates.length, 0);

  for (const tag of ["script", "iframe"]) {
    await assert.rejects(
      Promise.resolve().then(() => parseRecipeClipboard({
        html: `<${tag}>${fakeRecipe}`,
        plainText: null,
        sourceUrl: null,
      })),
      /does not contain a recognizable recipe/u,
    );
  }
});
