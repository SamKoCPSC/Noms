#!/usr/bin/env node

import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { parseRecipeClipboard } from "./recipe-import-parser.mjs";
import {
  validateImportCandidate,
  validateJsonSchema,
} from "./validate-recipe-import-fixtures.mjs";

const schemaPath = fileURLToPath(
  new URL("../tests/fixtures/recipe-import/v1/import-candidate.schema.json", import.meta.url),
);
const reviewType = "noms-recipe-import-semantic-v1";
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
const coreFields = ["title", "ingredients", "ingredientParsing", "instructions"];
const coreFieldLabels = new Set(["correct", "partial", "incorrect"]);
const optionalFieldLabels = new Set([
  "correct",
  "partial",
  "incorrect",
  "not_applicable",
  "not_reviewed",
]);
const overallLabels = new Set([
  "usable_without_correction",
  "usable_after_correction",
  "unusable",
]);
const fieldCountKeys = new Map([
  ["correct", "correct"],
  ["partial", "partial"],
  ["incorrect", "incorrect"],
  ["not_applicable", "notApplicable"],
  ["not_reviewed", "notReviewed"],
]);

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function compareText(left, right) {
  if (left === right) return 0;
  return left < right ? -1 : 1;
}

function percent(numerator, denominator) {
  if (denominator === 0) return 0;
  return Math.round((numerator / denominator) * 1_000) / 10;
}

function canonicalJson(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  return `{${Object.keys(value)
    .sort(compareText)
    .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
    .join(",")}}`;
}

function candidateDigest(candidate) {
  return createHash("sha256").update(canonicalJson(candidate)).digest("hex");
}

async function readVerifiedFile(dataRoot, rootRealPath, sampleId, descriptor) {
  if (!descriptor || !nonEmptyString(descriptor.path) || path.isAbsolute(descriptor.path)) {
    throw new Error(`Invalid corpus path for ${sampleId}`);
  }
  if (!Number.isSafeInteger(descriptor.bytes) || descriptor.bytes < 0) {
    throw new Error(`Invalid byte count for ${sampleId}`);
  }
  if (!/^[a-f0-9]{64}$/iu.test(descriptor.sha256 ?? "")) {
    throw new Error(`Invalid SHA-256 digest for ${sampleId}`);
  }

  const resolved = path.resolve(dataRoot, descriptor.path);
  if (resolved !== dataRoot && !resolved.startsWith(`${dataRoot}${path.sep}`)) {
    throw new Error(`Corpus path escapes corpus root: ${descriptor.path}`);
  }

  let realFilePath;
  try {
    realFilePath = await fs.realpath(resolved);
  } catch (error) {
    throw new Error(`Unable to read ${descriptor.path}: ${error.message}`, { cause: error });
  }
  if (realFilePath !== rootRealPath && !realFilePath.startsWith(`${rootRealPath}${path.sep}`)) {
    throw new Error(`Corpus path escapes corpus root through a symbolic link: ${descriptor.path}`);
  }

  const content = await fs.readFile(realFilePath);
  const digest = createHash("sha256").update(content).digest("hex");
  if (digest !== descriptor.sha256.toLowerCase()) {
    throw new Error(`${descriptor.path} digest mismatch`);
  }
  if (content.byteLength !== descriptor.bytes) {
    throw new Error(
      `${descriptor.path} byte count mismatch: expected ${descriptor.bytes}, found ${content.byteLength}`,
    );
  }
  return content.toString("utf8");
}

async function loadCorpus({ manifestPath, dataRoot }) {
  if (!manifestPath) throw new Error("manifestPath is required");
  const resolvedManifestPath = path.resolve(manifestPath);
  const resolvedDataRoot = path.resolve(dataRoot ?? path.dirname(resolvedManifestPath));
  const rootRealPath = await fs.realpath(resolvedDataRoot);

  let manifest;
  try {
    manifest = JSON.parse(await fs.readFile(resolvedManifestPath, "utf8"));
  } catch (error) {
    throw new Error(`Unable to read corpus manifest: ${error.message}`, { cause: error });
  }
  if (!Array.isArray(manifest.samples)) {
    throw new Error("Corpus manifest must contain a samples array");
  }
  if (!nonEmptyString(manifest.corpusId)) {
    throw new Error("Corpus manifest must contain a non-empty corpusId");
  }

  const samplesById = new Map();
  for (const sample of manifest.samples) {
    if (!sample || !nonEmptyString(sample.id) || samplesById.has(sample.id)) {
      throw new Error(`Corpus sample IDs must be non-empty and unique: ${sample?.id}`);
    }
    if (!nonEmptyString(sample.provider)) {
      throw new Error(`Corpus sample ${sample.id} must have a non-empty provider`);
    }
    if (!sample.files?.plainText || !sample.files?.html) {
      throw new Error(`Sample ${sample.id} must reference paired plain-text and HTML files`);
    }
    samplesById.set(sample.id, sample);
  }

  return {
    manifest,
    samplesById,
    dataRoot: resolvedDataRoot,
    rootRealPath,
    schema: JSON.parse(await fs.readFile(schemaPath, "utf8")),
  };
}

async function acceptedCandidate(corpus, sample) {
  const plainText = await readVerifiedFile(
    corpus.dataRoot,
    corpus.rootRealPath,
    sample.id,
    sample.files.plainText,
  );
  const html = await readVerifiedFile(
    corpus.dataRoot,
    corpus.rootRealPath,
    sample.id,
    sample.files.html,
  );

  let candidate;
  try {
    candidate = await parseRecipeClipboard({
      html,
      plainText,
      sourceUrl: nonEmptyString(sample.sourceUrl) ? sample.sourceUrl : null,
    });
  } catch (error) {
    if (error?.message === "Clipboard input does not contain a recognizable recipe.") {
      throw new Error(`Corpus sample ${sample.id} is not an accepted parser candidate`);
    }
    throw new Error(`Parser failed for corpus sample ${sample.id}: ${error.message}`, { cause: error });
  }

  if (candidate === null || candidate === undefined || candidate.status === "unsupported") {
    throw new Error(`Corpus sample ${sample.id} is not an accepted parser candidate`);
  }
  const validationErrors = [
    ...validateJsonSchema(candidate, corpus.schema),
    ...validateImportCandidate(candidate),
  ];
  if (validationErrors.length > 0) {
    throw new Error(`Corpus sample ${sample.id} produced an invalid parser candidate`);
  }
  return candidate;
}

function blankAssessment() {
  return Object.fromEntries(assessmentFields.map((field) => [field, null]));
}

function corpusIdentity(manifest) {
  return {
    schemaVersion: manifest.schemaVersion ?? null,
    corpusId: manifest.corpusId,
  };
}

/** Create a content-free form whose labels are cryptographically bound to parser output. */
export async function createSemanticReviewTemplate({ manifestPath, dataRoot, sampleIds } = {}) {
  if (!Array.isArray(sampleIds) || sampleIds.length === 0) {
    throw new Error("sampleIds must be a non-empty array");
  }
  const requestedIds = new Set();
  for (const sampleId of sampleIds) {
    if (!nonEmptyString(sampleId) || requestedIds.has(sampleId)) {
      throw new Error(`Requested sample IDs must be non-empty and unique: ${sampleId}`);
    }
    requestedIds.add(sampleId);
  }

  const corpus = await loadCorpus({ manifestPath, dataRoot });
  const samples = [];
  for (const sampleId of sampleIds) {
    const sample = corpus.samplesById.get(sampleId);
    if (!sample) throw new Error(`Corpus sample does not exist: ${sampleId}`);
    const candidate = await acceptedCandidate(corpus, sample);
    samples.push({
      id: sample.id,
      provider: sample.provider,
      candidateSha256: candidateDigest(candidate),
      parserId: candidate.parser.id,
      parserTier: candidate.parser.tier,
      status: candidate.status,
      assessment: blankAssessment(),
    });
  }

  return {
    schemaVersion: 1,
    reviewType,
    corpus: corpusIdentity(corpus.manifest),
    samples,
  };
}

function validateAssessment(sample) {
  if (!sample.assessment || typeof sample.assessment !== "object" || Array.isArray(sample.assessment)) {
    throw new Error(`Sample ${sample.id} must contain an assessment object`);
  }
  const unexpectedFields = Object.keys(sample.assessment).filter(
    (field) => !assessmentFields.includes(field),
  );
  if (unexpectedFields.length > 0) {
    throw new Error(`Sample ${sample.id} has unexpected assessment.${unexpectedFields[0]}`);
  }
  for (const field of assessmentFields) {
    const label = sample.assessment[field];
    const allowed = field === "overall"
      ? overallLabels
      : coreFields.includes(field)
        ? coreFieldLabels
        : optionalFieldLabels;
    if (!allowed.has(label)) {
      throw new Error(`Sample ${sample.id} has invalid assessment.${field}`);
    }
  }
}

function emptyFieldCounts() {
  return {
    correct: 0,
    partial: 0,
    incorrect: 0,
    notApplicable: 0,
    notReviewed: 0,
  };
}

function emptySummary() {
  return {
    reviewedSamples: 0,
    staleCandidates: 0,
    coreFieldsReviewed: 0,
    coreFieldsCorrect: 0,
    coreFieldsPartial: 0,
    coreFieldsIncorrect: 0,
    coreFieldAccuracyPercent: 0,
    allCoreFieldsCorrectSamples: 0,
    usableWithoutCorrection: 0,
    usableAfterCorrection: 0,
    unusable: 0,
  };
}

function addReviewedAssessment(summary, fields, assessment) {
  summary.reviewedSamples += 1;
  let allCoreFieldsCorrect = true;
  for (const field of assessmentFields) {
    if (field === "overall") continue;
    const label = assessment[field];
    if (fields) fields[field][fieldCountKeys.get(label)] += 1;
    if (coreFields.includes(field)) {
      if (label === "correct" || label === "partial" || label === "incorrect") {
        summary.coreFieldsReviewed += 1;
        summary[`coreFields${label[0].toUpperCase()}${label.slice(1)}`] += 1;
      }
      if (label !== "correct") allCoreFieldsCorrect = false;
    }
  }
  if (allCoreFieldsCorrect) summary.allCoreFieldsCorrectSamples += 1;
  if (assessment.overall === "usable_without_correction") summary.usableWithoutCorrection += 1;
  if (assessment.overall === "usable_after_correction") summary.usableAfterCorrection += 1;
  if (assessment.overall === "unusable") summary.unusable += 1;
}

function finalizeSummary(summary) {
  summary.coreFieldAccuracyPercent = percent(
    summary.coreFieldsCorrect,
    summary.coreFieldsReviewed,
  );
}

function validateReviewHeader(review, corpus) {
  if (review?.schemaVersion !== 1 || review.reviewType !== reviewType) {
    throw new Error(`Review manifest must use schema version 1 and review type ${reviewType}`);
  }
  if (
    !review.corpus ||
    review.corpus.schemaVersion !== (corpus.manifest.schemaVersion ?? null) ||
    review.corpus.corpusId !== corpus.manifest.corpusId
  ) {
    throw new Error("Review manifest corpus identity does not match the corpus manifest");
  }
  if (!Array.isArray(review.samples) || review.samples.length === 0) {
    throw new Error("Review manifest must contain a non-empty samples array");
  }
}

async function readReviewManifest(reviewManifestPath) {
  if (!reviewManifestPath) throw new Error("reviewManifestPath is required");
  try {
    return JSON.parse(await fs.readFile(path.resolve(reviewManifestPath), "utf8"));
  } catch (error) {
    throw new Error(`Unable to read semantic review manifest: ${error.message}`, { cause: error });
  }
}

/** Score human labels only after verifying corpus bytes and the reviewed candidate digest. */
export async function evaluateSemanticReview({ manifestPath, dataRoot, reviewManifestPath } = {}) {
  const corpus = await loadCorpus({ manifestPath, dataRoot });
  const review = await readReviewManifest(reviewManifestPath);
  validateReviewHeader(review, corpus);

  const reviewedIds = new Set();
  const fields = Object.fromEntries(
    assessmentFields.filter((field) => field !== "overall").map((field) => [field, emptyFieldCounts()]),
  );
  const summary = emptySummary();
  const providerSummaries = new Map();
  const samples = [];

  for (const reviewedSample of review.samples) {
    if (!reviewedSample || !nonEmptyString(reviewedSample.id) || reviewedIds.has(reviewedSample.id)) {
      throw new Error(`Reviewed sample IDs must be non-empty and unique: ${reviewedSample?.id}`);
    }
    reviewedIds.add(reviewedSample.id);
    const corpusSample = corpus.samplesById.get(reviewedSample.id);
    if (!corpusSample) throw new Error(`Reviewed corpus sample does not exist: ${reviewedSample.id}`);
    if (reviewedSample.provider !== corpusSample.provider) {
      throw new Error(`Reviewed sample provider does not match corpus: ${reviewedSample.id}`);
    }
    if (!/^[a-f0-9]{64}$/u.test(reviewedSample.candidateSha256 ?? "")) {
      throw new Error(`Reviewed sample has an invalid candidate digest: ${reviewedSample.id}`);
    }
    validateAssessment(reviewedSample);

    const candidate = await acceptedCandidate(corpus, corpusSample);
    if (candidateDigest(candidate) !== reviewedSample.candidateSha256) {
      throw new Error(`Sample ${reviewedSample.id} candidate changed since manual review`);
    }
    if (
      reviewedSample.parserId !== candidate.parser.id ||
      reviewedSample.parserTier !== candidate.parser.tier ||
      reviewedSample.status !== candidate.status
    ) {
      throw new Error(`Sample ${reviewedSample.id} parser metadata changed since manual review`);
    }

    addReviewedAssessment(summary, fields, reviewedSample.assessment);
    let providerSummary = providerSummaries.get(corpusSample.provider);
    if (!providerSummary) {
      providerSummary = { provider: corpusSample.provider, summary: emptySummary() };
      providerSummaries.set(corpusSample.provider, providerSummary);
    }
    addReviewedAssessment(providerSummary.summary, null, reviewedSample.assessment);
    samples.push({
      id: reviewedSample.id,
      provider: corpusSample.provider,
      candidateSha256: reviewedSample.candidateSha256,
      assessment: Object.fromEntries(
        assessmentFields.map((field) => [field, reviewedSample.assessment[field]]),
      ),
    });
  }

  finalizeSummary(summary);
  const providers = Array.from(providerSummaries.values(), ({ provider, summary: providerSummary }) => {
    finalizeSummary(providerSummary);
    return { provider, ...providerSummary };
  }).sort((left, right) => compareText(left.provider, right.provider));

  return {
    schemaVersion: 1,
    evaluationType: reviewType,
    corpus: corpusIdentity(corpus.manifest),
    summary,
    fields,
    providers,
    samples,
  };
}

function markdownCell(value) {
  return String(value)
    .replace(/&/gu, "&amp;")
    .replace(/</gu, "&lt;")
    .replace(/>/gu, "&gt;")
    .replace(/([\[\]])/gu, "\\$1")
    .replace(/\|/gu, "\\|")
    .replace(/[\r\n]+/gu, " ");
}

/** Render only aggregate scores and provider names; no candidate content is retained. */
export function renderSemanticMarkdown(evaluation) {
  const { summary } = evaluation;
  const lines = [
    "# NOMS-016 Semantic Parser Evaluation",
    "",
    "This privacy-preserving report contains human review scores and corpus identifiers only. It does not contain source URLs or extracted recipe content.",
    "",
    "## Summary",
    "",
    `- Reviewed samples: ${summary.reviewedSamples}`,
    `- Core field accuracy: ${summary.coreFieldsCorrect} of ${summary.coreFieldsReviewed} (${summary.coreFieldAccuracyPercent}%)`,
    `- Samples with all core fields correct: ${summary.allCoreFieldsCorrectSamples}`,
    `- Usable without correction: ${summary.usableWithoutCorrection}`,
    `- Usable after correction: ${summary.usableAfterCorrection}`,
    `- Unusable: ${summary.unusable}`,
    "",
    "Human labels apply only to the candidate digest that was reviewed. Changed parser output must be reviewed again.",
    "",
    "## Core fields",
    "",
    "| Field | Correct | Partial | Incorrect |",
    "| --- | ---: | ---: | ---: |",
  ];
  for (const field of coreFields) {
    const counts = evaluation.fields[field];
    lines.push(
      `| ${markdownCell(field)} | ${counts.correct} | ${counts.partial} | ${counts.incorrect} |`,
    );
  }
  lines.push(
    "",
    "## Optional fields",
    "",
    "| Field | Correct | Partial | Incorrect | Not applicable | Not reviewed |",
    "| --- | ---: | ---: | ---: | ---: | ---: |",
  );
  for (const field of assessmentFields.filter(
    (assessmentField) => assessmentField !== "overall" && !coreFields.includes(assessmentField),
  )) {
    const counts = evaluation.fields[field];
    lines.push(
      `| ${markdownCell(field)} | ${counts.correct} | ${counts.partial} | ${counts.incorrect} | ${counts.notApplicable} | ${counts.notReviewed} |`,
    );
  }
  lines.push(
    "",
    "## Providers",
    "",
    "| Provider | Reviewed | Core accuracy | Without correction | After correction | Unusable |",
    "| --- | ---: | ---: | ---: | ---: | ---: |",
  );
  for (const provider of evaluation.providers) {
    lines.push(
      `| ${markdownCell(provider.provider)} | ${provider.reviewedSamples} | ${provider.coreFieldsCorrect} of ${provider.coreFieldsReviewed} (${provider.coreFieldAccuracyPercent}%) | ${provider.usableWithoutCorrection} | ${provider.usableAfterCorrection} | ${provider.unusable} |`,
    );
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

function parseCliArguments(argumentsList) {
  const options = { sampleIds: [] };
  for (let index = 0; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--help" || argument === "-h") return { help: true };
    const optionName = {
      "--manifest": "manifestPath",
      "--data-root": "dataRoot",
      "--review": "reviewManifestPath",
      "--json-out": "jsonOut",
      "--markdown-out": "markdownOut",
      "--template-out": "templateOut",
      "--sample": "sampleId",
    }[argument];
    if (!optionName) throw new Error(`Unknown argument: ${argument}`);
    const value = argumentsList[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${argument}`);
    if (optionName === "sampleId") options.sampleIds.push(value);
    else options[optionName] = value;
    index += 1;
  }
  return options;
}

async function writeOutput(outputPath, content) {
  const resolvedPath = path.resolve(outputPath);
  await fs.mkdir(path.dirname(resolvedPath), { recursive: true });
  await fs.writeFile(resolvedPath, content);
}

async function runCli() {
  const options = parseCliArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(
      "Usage: node scripts/evaluate-recipe-parser-semantic.mjs --manifest PATH [--data-root PATH] (--template-out PATH --sample ID... | --review PATH [--json-out PATH] [--markdown-out PATH])\n",
    );
    return;
  }
  if (!options.manifestPath) throw new Error("--manifest is required");
  if (options.templateOut) {
    if (options.reviewManifestPath || options.jsonOut || options.markdownOut) {
      throw new Error("--template-out cannot be combined with review output options");
    }
    const template = await createSemanticReviewTemplate(options);
    await writeOutput(options.templateOut, `${JSON.stringify(template, null, 2)}\n`);
    return;
  }
  if (!options.reviewManifestPath) throw new Error("--review is required when not creating a template");
  if (options.sampleIds.length > 0) throw new Error("--sample is only valid with --template-out");
  const evaluation = await evaluateSemanticReview(options);
  if (options.jsonOut) {
    await writeOutput(options.jsonOut, `${JSON.stringify(evaluation, null, 2)}\n`);
  }
  if (options.markdownOut) {
    await writeOutput(options.markdownOut, renderSemanticMarkdown(evaluation));
  }
  if (!options.jsonOut && !options.markdownOut) {
    process.stdout.write(`${JSON.stringify(evaluation, null, 2)}\n`);
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  runCli().catch((error) => {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exitCode = 1;
  });
}
