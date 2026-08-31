import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  loadAndValidateFixtureCorpus,
  validateImportCandidate,
} from "../validate-recipe-import-fixtures.mjs";

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const fixtureRoot = path.join(
  repositoryRoot,
  "tests",
  "fixtures",
  "recipe-import",
  "v1",
);

test("the v1 corpus covers the first distinct parser risks", async () => {
  const corpus = await loadAndValidateFixtureCorpus(fixtureRoot);

  assert.equal(corpus.manifest.schemaVersion, 1);
  assert.deepEqual(
    corpus.fixtures.map(({ fixture }) => fixture.id).sort(),
    [
      "basic-visible-recipe",
      "competing-recipe-regions",
      "grouped-components",
      "notes-equipment-and-media",
      "plain-text-fallback",
      "remote-content-and-controls",
      "unit-yield-and-duration-ambiguity",
      "unsupported-access-shell",
    ],
  );

  const coveredRisks = new Set(
    corpus.fixtures.flatMap(({ fixture }) => fixture.risks),
  );
  for (const risk of [
    "generic-heading-pair",
    "grouped-ingredients",
    "competing-regions",
    "active-variants",
    "suspicious-duration",
    "notes",
    "equipment",
    "images",
    "remote-resources",
    "form-controls",
    "plain-text-only",
    "no-recipe-region",
  ]) {
    assert.ok(coveredRisks.has(risk), `missing fixture risk: ${risk}`);
  }
});

test("every fixture is compact, synthetic, and has a valid expected candidate", async () => {
  const corpus = await loadAndValidateFixtureCorpus(fixtureRoot);

  for (const { fixture, input, expected } of corpus.fixtures) {
    assert.ok(input.html === null || Buffer.byteLength(input.html) < 20_000);
    assert.ok(input.plainText === null || Buffer.byteLength(input.plainText) < 10_000);
    assert.equal(expected.schemaVersion, 1);
    assert.ok(["ready", "needs_review", "unsupported"].includes(expected.status));
    assert.equal(validateImportCandidate(expected).length, 0);

    const serialized = JSON.stringify({ fixture, input, expected });
    assert.doesNotMatch(
      serialized,
      /allrecipes|bonappetit|budgetbytes|epicurious|food52|foodnetwork|food\.com|kingarthur|nytimes|recipetineats|sallysbakingaddiction|seriouseats|simplyrecipes|tasty/i,
      `fixture ${fixture.id} contains provider-derived branding`,
    );
    assert.doesNotMatch(
      serialized,
      /https?:\/\/(?!example\.(?:test|invalid)(?:[/:]|$))/i,
      `fixture ${fixture.id} contains a non-reserved URL`,
    );
  }
});

test("candidate validation rejects lossy or unsafe contract shapes", () => {
  const invalidCandidate = {
    schemaVersion: 1,
    status: "ready",
    unexpected: true,
    parser: {
      id: "generic-html",
      version: "1",
      tier: "generic_html",
      sourceAdapter: null,
    },
    source: {
      suppliedUrl: null,
      normalizedUrl: "https://example.test/recipe",
      provider: null,
      author: null,
      attribution: null,
      provenance: {
        suppliedUrl: [],
        normalizedUrl: [],
        provider: [],
        author: [],
        attribution: [],
      },
    },
    fields: {
      title: {
        raw: "Unsafe Stew",
        value: "Unsafe Stew",
        confidence: "high",
        provenance: [],
      },
      description: null,
      prepTime: null,
      cookTime: null,
      totalTime: null,
      yield: null,
      ingredientSections: [
        {
          name: null,
          items: [
            {
              raw: "a dash of maybe",
              amount: "1",
              unit: "dash",
              name: "maybe",
              confidence: "high",
              provenance: [],
            },
          ],
        },
      ],
      instructionSections: [],
      equipment: [],
      notes: [],
      images: [{ sourceUrl: "javascript:alert(1)", alt: null, credit: null }],
    },
    unclassifiedLines: [],
    competingCandidates: [{ title: "", reason: "", provenance: [] }],
    warnings: [],
    discardedContent: [{ kind: "", count: 0, reason: "" }],
  };

  const errors = validateImportCandidate(invalidCandidate);
  assert.ok(errors.some((error) => /provenance/i.test(error)));
  assert.ok(errors.some((error) => /instruction/i.test(error)));
  assert.ok(errors.some((error) => /image|url|scheme/i.test(error)));
  assert.ok(errors.some((error) => /source\.provenance\.normalizedUrl/i.test(error)));
  assert.ok(errors.some((error) => /competingCandidates/i.test(error)));
  assert.ok(errors.some((error) => /discardedContent/i.test(error)));
  assert.ok(errors.some((error) => /unexpected.*not allowed/i.test(error)));
});

test("unsupported candidates preserve evidence and explain why parsing stopped", () => {
  const candidate = {
    schemaVersion: 1,
    status: "unsupported",
    parser: {
      id: "none",
      version: "1",
      tier: "none",
      sourceAdapter: null,
    },
    source: {
      suppliedUrl: null,
      normalizedUrl: null,
      provider: null,
      author: null,
      attribution: null,
      provenance: {
        suppliedUrl: [],
        normalizedUrl: [],
        provider: [],
        author: [],
        attribution: [],
      },
    },
    fields: {
      title: null,
      description: null,
      prepTime: null,
      cookTime: null,
      totalTime: null,
      yield: null,
      ingredientSections: [],
      instructionSections: [],
      equipment: [],
      notes: [],
      images: [],
    },
    unclassifiedLines: [
      {
        raw: "Sign in to continue",
        provenance: [{ representation: "text/plain", locator: "line 1" }],
      },
    ],
    competingCandidates: [],
    warnings: [
      {
        code: "NO_RECIPE_REGION",
        severity: "error",
        path: "fields",
        message: "No ingredient and instruction regions were found.",
      },
    ],
    discardedContent: [],
  };

  assert.deepEqual(validateImportCandidate(candidate), []);
});
