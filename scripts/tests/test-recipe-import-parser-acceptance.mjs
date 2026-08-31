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
