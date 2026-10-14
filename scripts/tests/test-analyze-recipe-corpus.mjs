import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

import {
  analyzeCorpus,
  renderMarkdown,
} from "../analyze-recipe-corpus.mjs";

const sha256 = (data) => createHash("sha256").update(data).digest("hex");
const execFileAsync = promisify(execFile);
const analyzerPath = fileURLToPath(new URL("../analyze-recipe-corpus.mjs", import.meta.url));

async function createCorpus() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "noms-corpus-analysis-"));
  const samples = [
    {
      id: "Alpha/Basic Soup",
      provider: "Alpha",
      text: "Basic Soup\nIngredients\n1 moonbean\nDirections\nStir it.",
      html: "<article><h1>Basic Soup</h1><h2>Ingredients</h2><ul><li>1 moonbean</li></ul><h2>Directions</h2><ol><li>Stir it.</li></ol></article>",
    },
    {
      id: "Alpha/Layered Pie",
      provider: "Alpha",
      text: "Layered Pie\nIngredients\nCrust\n1 stardust\nMethod\nFold it.",
      html: "<!--StartFragment--><main><h1>Layered Pie</h1><button>Metric</button><input aria-label=\"servings\"><h2>Ingredients</h2><h3>Crust</h3><ul><li>1 stardust</li></ul><h2>Ingredients</h2><h2>Method</h2><ol><li>Fold it.</li></ol><h2>Method</h2><h2>Notes</h2><img src=\"https://example.test/pie.jpg\"></main><!--EndFragment-->",
    },
    {
      id: "Beta/Cloud Bread",
      provider: "Beta",
      text: "Cloud Bread\nIngredients\n1 cloud\nPreparation\nBake it.",
      html: "<section itemscope itemtype=\"https://schema.org/Recipe\"><h1>Cloud Bread</h1><h2>Ingredients</h2><p>1 cloud</p><h2>Process</h2><p>Bake it.</p><h2>Equipment</h2><p>One silver bowl</p></section>",
    },
  ];

  const manifestSamples = [];
  for (const sample of samples) {
    const providerDirectory = path.join(root, sample.provider);
    await fs.mkdir(providerDirectory, { recursive: true });
    const stem = sample.id.split("/").at(-1);
    const textPath = path.join(providerDirectory, `${stem}.txt`);
    const htmlPath = path.join(providerDirectory, `${stem}.html`);
    const textData = Buffer.from(sample.text);
    const htmlData = Buffer.from(sample.html);
    await fs.writeFile(textPath, textData);
    await fs.writeFile(htmlPath, htmlData);
    manifestSamples.push({
      id: sample.id,
      provider: sample.provider,
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
  await fs.writeFile(
    manifestPath,
    JSON.stringify({
      schemaVersion: 1,
      corpusId: "synthetic-test-corpus",
      samples: manifestSamples,
    }),
  );
  return { root, manifestPath };
}

test("characterizes providers, clusters, and structural features", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));

  const analysis = await analyzeCorpus({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
  });

  assert.deepEqual(analysis.summary, {
    providers: 2,
    samples: 3,
    plainTextBytes: 161,
    htmlBytes: 637,
    totalBytes: 798,
    integrityVerified: true,
  });

  const alpha = analysis.providers.find((provider) => provider.provider === "Alpha");
  assert.equal(alpha.sampleCount, 2);
  assert.equal(alpha.clusterCount, 2);
  assert.equal(alpha.featureCounts.clipboardFragment, 1);
  assert.equal(alpha.featureCounts.formControls, 1);
  assert.equal(alpha.featureCounts.unitControls, 1);
  assert.equal(alpha.featureCounts.yieldControls, 1);
  assert.equal(alpha.featureCounts.notes, 1);
  assert.equal(alpha.featureCounts.images, 1);
  assert.equal(alpha.featureCounts.duplicateIngredientRegion, 1);
  assert.equal(alpha.featureCounts.duplicateInstructionRegion, 1);
  assert.deepEqual(
    alpha.clusters.flatMap((cluster) => cluster.representativeSampleIds).sort(),
    ["Alpha/Basic Soup", "Alpha/Layered Pie"],
  );
  assert.deepEqual(alpha.recommendedFixtureSampleIds.sort(), [
    "Alpha/Basic Soup",
    "Alpha/Layered Pie",
  ]);
  assert.match(alpha.fixturePriorities.join(" "), /duplicate ingredient/i);
  assert.match(alpha.fixturePriorities.join(" "), /unit.*yield|yield.*unit/i);
  assert.doesNotMatch(alpha.fixturePriorities.join(" "), /expose both/i);

  const beta = analysis.providers.find((provider) => provider.provider === "Beta");
  assert.equal(beta.featureCounts.schemaOrgRecipe, 1);
  assert.equal(beta.featureCounts.equipment, 1);
  assert.equal(beta.featureCounts.instructionRegion, 0);
  assert.match(beta.fixturePriorities.join(" "), /missing.*instruction/i);
});

test("renders a structural report without copied recipe content", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  const analysis = await analyzeCorpus({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
  });

  const markdown = renderMarkdown(analysis);
  assert.match(markdown, /^# NOMS-016 Corpus Analysis/m);
  assert.match(markdown, /\| Alpha \| 2 \|/);
  assert.match(markdown, /Alpha\/Layered Pie/);
  assert.match(markdown, /^## Key findings$/m);
  assert.match(markdown, /^## Fixture recommendations$/m);
  assert.match(markdown, /^## Interpretation limits$/m);
  assert.doesNotMatch(markdown, /moonbean|stardust|Stir it|Fold it/);
});

test("rejects a corpus whose file digest no longer matches", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  await fs.appendFile(path.join(corpus.root, "Alpha", "Basic Soup.html"), "tampered");

  await assert.rejects(
    analyzeCorpus({ manifestPath: corpus.manifestPath, dataRoot: corpus.root }),
    /Digest mismatch.*Alpha\/Basic Soup\.html/,
  );
});

test("CLI writes JSON and Markdown analysis artifacts", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  const jsonOutput = path.join(corpus.root, "analysis", "corpus-analysis.json");
  const markdownOutput = path.join(corpus.root, "analysis", "corpus-analysis.md");

  await execFileAsync(process.execPath, [
    analyzerPath,
    "--manifest",
    corpus.manifestPath,
    "--data-root",
    corpus.root,
    "--json-out",
    jsonOutput,
    "--markdown-out",
    markdownOutput,
  ]);

  const analysis = JSON.parse(await fs.readFile(jsonOutput, "utf8"));
  const markdown = await fs.readFile(markdownOutput, "utf8");
  assert.equal(analysis.summary.samples, 3);
  assert.match(markdown, /^## Fixture recommendations$/m);
});
