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
  evaluateParserCorpus,
  renderMarkdown,
} from "../evaluate-recipe-parser-corpus.mjs";

const execFileAsync = promisify(execFile);
const evaluatorPath = fileURLToPath(new URL("../evaluate-recipe-parser-corpus.mjs", import.meta.url));
const sha256 = (data) => createHash("sha256").update(data).digest("hex");

async function createCorpus() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "noms-parser-evaluation-"));
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
      id: "Alpha/Text Stew",
      provider: "Alpha",
      pageTitle: "Text Stew",
      sourceUrl: "https://alpha.example/text-stew",
      plainText: "Text Stew\nIngredients\n- 1 stardrop\nDirections\n1. Stir.",
      html: "<div><p>Clipboard shell without recipe structure.</p></div>",
    },
    {
      id: "Beta/Layered Pie",
      provider: "Beta",
      pageTitle: "Layered Pie | Beta",
      sourceUrl: "https://beta.example/layered-pie",
      plainText: "Layered Pie\nIngredients\nCrust\n1 moon\nFilling\n2 stars\nMethod\nBuild\nMix.\nFinish\nBake.",
      html: "<main><h1>Layered Pie</h1><h2>Ingredients</h2><h3>Crust</h3><ul><li>1 moon</li></ul><h3>Filling</h3><ul><li>2 stars</li></ul><h2>Method</h2><h3>Build</h3><ol><li>Mix.</li></ol><h3>Finish</h3><ol><li>Bake.</li></ol></main>",
    },
    {
      id: "Beta/Access Shell",
      provider: "Beta",
      pageTitle: "Access Shell",
      sourceUrl: "https://beta.example/access-shell",
      plainText: "Sign in to continue.",
      html: "<main><h1>Subscribers only</h1><button>Sign in</button></main>",
    },
  ];

  const manifestSamples = [];
  for (const sample of samples) {
    const providerDirectory = path.join(root, sample.provider);
    await fs.mkdir(providerDirectory, { recursive: true });
    const stem = sample.id.split("/").at(-1);
    const textPath = path.join(providerDirectory, `${stem}.txt`);
    const htmlPath = path.join(providerDirectory, `${stem}.html`);
    const textData = Buffer.from(sample.plainText);
    const htmlData = Buffer.from(sample.html);
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
    corpusId: "synthetic-parser-evaluation",
    samples: manifestSamples,
  }));
  return { root, manifestPath };
}

test("evaluates parser coverage without retaining recipe bodies", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));

  const evaluation = await evaluateParserCorpus({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
  });

  assert.deepEqual(evaluation.corpus, {
    schemaVersion: 1,
    corpusId: "synthetic-parser-evaluation",
  });
  assert.deepEqual(evaluation.summary, {
    providers: 2,
    samples: 4,
    accepted: 3,
    rejected: 1,
    invalidCandidates: 0,
    unexpectedErrors: 0,
    acceptancePercent: 75,
    htmlAccepted: 2,
    plainTextAccepted: 1,
    ready: 2,
    needsReview: 1,
    unsupported: 0,
    titlePresent: 3,
    descriptionPresent: 1,
    titleExact: 2,
    titleContained: 1,
    titleMismatch: 0,
    ingredientSections: 4,
    ingredientItems: 5,
    instructionSections: 4,
    instructionSteps: 5,
    integrityVerified: true,
  });

  assert.deepEqual(evaluation.providers, [
    {
      provider: "Alpha",
      sampleCount: 2,
      accepted: 2,
      rejected: 0,
      invalidCandidates: 0,
      unexpectedErrors: 0,
      acceptancePercent: 100,
      htmlAccepted: 1,
      plainTextAccepted: 1,
      ready: 2,
      needsReview: 0,
      unsupported: 0,
      titlePresent: 2,
      descriptionPresent: 1,
      titleExact: 2,
      titleContained: 0,
      titleMismatch: 0,
      ingredientItems: 3,
      instructionSteps: 3,
      reviewSampleIds: ["Alpha/Basic Soup", "Alpha/Text Stew"],
      rejectedSampleIds: [],
    },
    {
      provider: "Beta",
      sampleCount: 2,
      accepted: 1,
      rejected: 1,
      invalidCandidates: 0,
      unexpectedErrors: 0,
      acceptancePercent: 50,
      htmlAccepted: 1,
      plainTextAccepted: 0,
      ready: 0,
      needsReview: 1,
      unsupported: 0,
      titlePresent: 1,
      descriptionPresent: 0,
      titleExact: 0,
      titleContained: 1,
      titleMismatch: 0,
      ingredientItems: 2,
      instructionSteps: 2,
      reviewSampleIds: ["Beta/Layered Pie", "Beta/Access Shell"],
      rejectedSampleIds: ["Beta/Access Shell"],
    },
  ]);

  assert.equal(evaluation.samples.length, 4);
  assert.deepEqual(evaluation.samples[0], {
    id: "Alpha/Basic Soup",
    provider: "Alpha",
    outcome: "accepted",
    parserId: "generic-headings",
    parserTier: "generic_html",
    status: "ready",
    titlePresent: true,
    descriptionPresent: true,
    titleAgreement: "exact",
    ingredientSections: 1,
    ingredientItems: 2,
    instructionSections: 1,
    instructionSteps: 2,
    warningCodes: [],
  });
  assert.deepEqual(evaluation.samples.at(-1), {
    id: "Beta/Access Shell",
    provider: "Beta",
    outcome: "rejected",
    reason: "unrecognized_recipe",
  });

  const serialized = JSON.stringify(evaluation);
  for (const bodyText of ["moonbeans", "comet salt", "stardrop", "Subscribers only"]) {
    assert.doesNotMatch(serialized, new RegExp(bodyText, "iu"));
  }
  assert.deepEqual(
    evaluation,
    await evaluateParserCorpus({ manifestPath: corpus.manifestPath, dataRoot: corpus.root }),
    "evaluation must be deterministic",
  );
});

test("renders an aggregate compatibility report with explicit interpretation limits", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  const evaluation = await evaluateParserCorpus({
    manifestPath: corpus.manifestPath,
    dataRoot: corpus.root,
  });

  const report = renderMarkdown(evaluation);
  assert.match(report, /^# NOMS-016 Real-Corpus Parser Evaluation/mu);
  assert.match(report, /Structural acceptance: 3 of 4 \(75%\)/u);
  assert.match(report, /\| Alpha \| 2 \| 2 \| 100% \| 1 \| 1 \| 0 \|/u);
  assert.match(report, /Beta\/Access Shell/u);
  assert.match(report, /Structural acceptance does not prove semantic correctness/u);
  assert.doesNotMatch(report, /moonbeans|comet salt|stardrop/iu);
});

test("rejects tampered files and paths outside the corpus root", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  await fs.appendFile(path.join(corpus.root, "Alpha", "Basic Soup.html"), "tampered");
  await assert.rejects(
    evaluateParserCorpus({ manifestPath: corpus.manifestPath, dataRoot: corpus.root }),
    /digest mismatch/u,
  );

  const escaped = await createCorpus();
  context.after(() => fs.rm(escaped.root, { recursive: true, force: true }));
  const manifest = JSON.parse(await fs.readFile(escaped.manifestPath, "utf8"));
  manifest.samples[0].files.html.path = "../outside.html";
  await fs.writeFile(escaped.manifestPath, JSON.stringify(manifest));
  await assert.rejects(
    evaluateParserCorpus({ manifestPath: escaped.manifestPath, dataRoot: escaped.root }),
    /escapes corpus root/u,
  );
});

test("CLI writes JSON and Markdown reports", async (context) => {
  const corpus = await createCorpus();
  context.after(() => fs.rm(corpus.root, { recursive: true, force: true }));
  const outputDirectory = path.join(corpus.root, "evaluation");
  const jsonOutput = path.join(outputDirectory, "parser-evaluation.json");
  const markdownOutput = path.join(outputDirectory, "parser-evaluation.md");

  await execFileAsync(process.execPath, [
    evaluatorPath,
    "--manifest",
    corpus.manifestPath,
    "--data-root",
    corpus.root,
    "--json-out",
    jsonOutput,
    "--markdown-out",
    markdownOutput,
  ]);

  const evaluation = JSON.parse(await fs.readFile(jsonOutput, "utf8"));
  assert.equal(evaluation.summary.accepted, 3);
  assert.match(await fs.readFile(markdownOutput, "utf8"), /## Provider coverage/u);
});
