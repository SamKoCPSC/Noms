import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  createSemanticReviewTemplate,
  evaluateSemanticReview,
  renderSemanticMarkdown,
} from "../evaluate-recipe-parser-semantic.mjs";

const sha256 = (data) => createHash("sha256").update(data).digest("hex");
const assessmentFields = [
  "title",
  "ingredients",
  "ingredientParsing",
  "instructions",
  "description",
  "times",
  "yield",
  "notesEquipment",
  "images",
  "overall",
];

async function createCorpus() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "noms-semantic-evaluation-"));
  const samples = [
    {
      id: "Alpha/Basic Soup",
      provider: "Alpha",
      pageTitle: "Basic Soup",
      sourceUrl: "https://alpha.example/basic-soup",
      plainText: "Basic Soup\nIngredients\n- 2 cups moonbeans\n- 1 tsp comet salt\nInstructions\n1. Warm.\n2. Serve.",
      html: "<article><h1>Basic Soup</h1><p>A quiet bowl.</p><h2>Ingredients</h2><ul><li>2 cups moonbeans</li><li>1 tsp comet salt</li></ul><h2>Instructions</h2><ol><li>Warm.</li><li>Serve.</li></ol></article>",
    },
    {
      id: "Beta/Layered Pie",
      provider: "Beta",
      pageTitle: "Layered Pie | Beta",
      sourceUrl: "https://beta.example/layered-pie",
      plainText: "Layered Pie\nIngredients\nCrust\n1 moon\nFilling\n2 stars\nMethod\nBuild\nMix.\nFinish\nBake.",
      html: "<main><h1>Layered Pie</h1><h2>Ingredients</h2><h3>Crust</h3><ul><li>1 moon</li></ul><h3>Filling</h3><ul><li>2 stars</li></ul><h2>Method</h2><h3>Build</h3><ol><li>Mix.</li></ol><h3>Finish</h3><ol><li>Bake.</li></ol></main>",
    },
  ];

  const manifestSamples = [];
  for (const sample of samples) {
    const providerDirectory = path.join(root, sample.provider);
    await fs.mkdir(providerDirectory, { recursive: true });
    const stem = sample.id.split("/").at(-1);
    const textData = Buffer.from(sample.plainText);
    const htmlData = Buffer.from(sample.html);
    const textPath = path.join(providerDirectory, `${stem}.txt`);
    const htmlPath = path.join(providerDirectory, `${stem}.html`);
    await fs.writeFile(textPath, textData);
    await fs.writeFile(htmlPath, htmlData);
    manifestSamples.push({
      id: sample.id,
      provider: sample.provider,
      pageTitle: sample.pageTitle,
      sourceUrl: sample.sourceUrl,
      files: {
        plainText: {
          path: path.relative(root, textPath),
          bytes: textData.byteLength,
          sha256: sha256(textData),
        },
        html: {
          path: path.relative(root, htmlPath),
          bytes: htmlData.byteLength,
          sha256: sha256(htmlData),
        },
      },
    });
  }

  const manifestPath = path.join(root, "corpus-manifest.json");
  await fs.writeFile(manifestPath, JSON.stringify({
    schemaVersion: 1,
    corpusId: "synthetic-semantic-evaluation",
    samples: manifestSamples,
  }));
  return { root, manifestPath };
}

test("creates a content-free semantic review template for explicit samples", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));

  const template = await createSemanticReviewTemplate({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
    sampleIds: ["Alpha/Basic Soup", "Beta/Layered Pie"],
  });

  assert.deepEqual(template.corpus, {
    schemaVersion: 1,
    corpusId: "synthetic-semantic-evaluation",
  });
  assert.equal(template.schemaVersion, 1);
  assert.equal(template.reviewType, "noms-recipe-import-semantic-v1");
  assert.deepEqual(template.samples.map(({ id }) => id), ["Alpha/Basic Soup", "Beta/Layered Pie"]);
  for (const sample of template.samples) {
    assert.match(sample.candidateSha256, /^[a-f0-9]{64}$/u);
    assert.deepEqual(Object.keys(sample.assessment), assessmentFields);
    assert.deepEqual(Object.values(sample.assessment), assessmentFields.map(() => null));
    assert.deepEqual(Object.keys(sample), [
      "id",
      "provider",
      "candidateSha256",
      "parserId",
      "parserTier",
      "status",
      "assessment",
    ]);
  }

  const serialized = JSON.stringify(template);
  assert.doesNotMatch(serialized, /moonbeans|comet salt|stardrop|alpha\.example/iu);
});

test("scores reviewed fields and rejects stale candidate assessments", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  const review = await createSemanticReviewTemplate({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
    sampleIds: ["Alpha/Basic Soup", "Beta/Layered Pie"],
  });
  Object.assign(review.samples[0].assessment, {
    title: "correct",
    ingredients: "correct",
    ingredientParsing: "correct",
    instructions: "correct",
    description: "correct",
    times: "not_applicable",
    yield: "not_applicable",
    notesEquipment: "not_applicable",
    images: "not_applicable",
    overall: "usable_without_correction",
  });
  Object.assign(review.samples[1].assessment, {
    title: "correct",
    ingredients: "partial",
    ingredientParsing: "incorrect",
    instructions: "correct",
    description: "not_applicable",
    times: "not_reviewed",
    yield: "not_reviewed",
    notesEquipment: "not_applicable",
    images: "not_reviewed",
    overall: "usable_after_correction",
  });
  const reviewManifestPath = path.join(corpus.root, "semantic-review.json");
  await fs.writeFile(reviewManifestPath, JSON.stringify(review));

  const evaluation = await evaluateSemanticReview({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
    reviewManifestPath,
  });
  assert.deepEqual(evaluation.summary, {
    reviewedSamples: 2,
    staleCandidates: 0,
    coreFieldsReviewed: 8,
    coreFieldsCorrect: 6,
    coreFieldsPartial: 1,
    coreFieldsIncorrect: 1,
    coreFieldAccuracyPercent: 75,
    allCoreFieldsCorrectSamples: 1,
    usableWithoutCorrection: 1,
    usableAfterCorrection: 1,
    unusable: 0,
  });
  assert.deepEqual(evaluation.fields.title, {
    correct: 2, partial: 0, incorrect: 0, notApplicable: 0, notReviewed: 0,
  });
  assert.deepEqual(evaluation.fields.ingredients, {
    correct: 1, partial: 1, incorrect: 0, notApplicable: 0, notReviewed: 0,
  });
  assert.deepEqual(evaluation.fields.ingredientParsing, {
    correct: 1, partial: 0, incorrect: 1, notApplicable: 0, notReviewed: 0,
  });
  assert.deepEqual(evaluation.fields.instructions, {
    correct: 2, partial: 0, incorrect: 0, notApplicable: 0, notReviewed: 0,
  });
  assert.deepEqual(evaluation.fields.description, {
    correct: 1, partial: 0, incorrect: 0, notApplicable: 1, notReviewed: 0,
  });
  assert.deepEqual(evaluation.fields.times, {
    correct: 0, partial: 0, incorrect: 0, notApplicable: 1, notReviewed: 1,
  });
  assert.deepEqual(evaluation.providers.map(({ provider, reviewedSamples }) => ({ provider, reviewedSamples })), [
    { provider: "Alpha", reviewedSamples: 1 },
    { provider: "Beta", reviewedSamples: 1 },
  ]);

  const serialized = JSON.stringify(evaluation);
  assert.doesNotMatch(serialized, /moonbeans|comet salt|stardrop|alpha\.example/iu);
  const report = renderSemanticMarkdown(evaluation);
  assert.match(report, /Core field accuracy: 6 of 8 \(75%\)/u);
  assert.match(report, /## Optional fields/u);
  assert.match(report, /\| description \| 1 \| 0 \| 0 \| 1 \| 0 \|/u);
  assert.match(report, /Human labels apply only to the candidate digest that was reviewed/u);
  assert.doesNotMatch(report, /moonbeans|comet salt|stardrop/iu);

  review.samples[0].candidateSha256 = "0".repeat(64);
  await fs.writeFile(reviewManifestPath, JSON.stringify(review));
  await assert.rejects(
    evaluateSemanticReview({
      manifestPath: corpus.manifestPath,
      dataRoot: corpus.root,
      reviewManifestPath,
    }),
    /candidate changed since manual review/u,
  );
});

test("requires complete, recognized human assessment labels", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  const review = await createSemanticReviewTemplate({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
    sampleIds: ["Alpha/Basic Soup"],
  });
  review.samples[0].assessment.title = "probably";
  const reviewManifestPath = path.join(corpus.root, "invalid-review.json");
  await fs.writeFile(reviewManifestPath, JSON.stringify(review));

  await assert.rejects(
    evaluateSemanticReview({
      manifestPath: corpus.manifestPath,
      dataRoot: corpus.root,
      reviewManifestPath,
    }),
    /assessment\.title/u,
  );

  review.samples[0].assessment.title = "not_applicable";
  await fs.writeFile(reviewManifestPath, JSON.stringify(review));
  await assert.rejects(
    evaluateSemanticReview({
      manifestPath: corpus.manifestPath,
      dataRoot: corpus.root,
      reviewManifestPath,
    }),
    /assessment\.title/u,
  );
});
