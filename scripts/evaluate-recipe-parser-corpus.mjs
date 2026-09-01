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
const outcomeOrder = new Map([
  ["accepted", 0],
  ["unsupported", 1],
  ["rejected", 2],
  ["invalid_candidate", 3],
  ["unexpected_error", 4],
]);

function compareText(left, right) {
  if (left === right) return 0;
  return left < right ? -1 : 1;
}

function percent(numerator, denominator) {
  if (denominator === 0) return 0;
  return Math.round((numerator / denominator) * 1_000) / 10;
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function normalizeTitle(value) {
  return String(value ?? "")
    .normalize("NFKC")
    .replace(/\s+/gu, " ")
    .trim()
    .toLocaleLowerCase("en-US");
}

function titleAgreement(candidateTitle, pageTitle) {
  const candidate = normalizeTitle(candidateTitle);
  const page = normalizeTitle(pageTitle);
  if (!candidate) return null;
  if (candidate === page) return "exact";
  if (page && (page.includes(candidate) || candidate.includes(page))) return "contained";
  return "mismatch";
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

function itemCount(sections, itemKey) {
  return sections.reduce((total, section) => total + section[itemKey].length, 0);
}

function summarizeAcceptedSample(sample, candidate) {
  const ingredientSections = candidate.fields.ingredientSections.length;
  const instructionSections = candidate.fields.instructionSections.length;
  return {
    id: sample.id,
    provider: sample.provider,
    outcome: "accepted",
    parserId: candidate.parser.id,
    parserTier: candidate.parser.tier,
    status: candidate.status,
    titlePresent: candidate.fields.title !== null,
    descriptionPresent: candidate.fields.description !== null,
    titleAgreement: titleAgreement(candidate.fields.title?.value, sample.pageTitle),
    ingredientSections,
    ingredientItems: itemCount(candidate.fields.ingredientSections, "items"),
    instructionSections,
    instructionSteps: itemCount(candidate.fields.instructionSections, "steps"),
    warningCodes: [...new Set(candidate.warnings.map(({ code }) => code))].sort(compareText),
  };
}

async function evaluateSample(sample, dataRoot, rootRealPath, schema) {
  if (!sample || !nonEmptyString(sample.id) || !nonEmptyString(sample.provider)) {
    throw new Error("Every corpus sample must have a non-empty id and provider");
  }
  if (!sample.files?.plainText || !sample.files?.html) {
    throw new Error(`Sample ${sample.id} must reference paired plain-text and HTML files`);
  }

  const plainText = await readVerifiedFile(
    dataRoot,
    rootRealPath,
    sample.id,
    sample.files.plainText,
  );
  const html = await readVerifiedFile(dataRoot, rootRealPath, sample.id, sample.files.html);

  let candidate;
  try {
    candidate = await parseRecipeClipboard({
      html,
      plainText,
      sourceUrl: nonEmptyString(sample.sourceUrl) ? sample.sourceUrl : null,
    });
  } catch (error) {
    if (error?.message === "Clipboard input does not contain a recognizable recipe.") {
      return {
        id: sample.id,
        provider: sample.provider,
        outcome: "rejected",
        reason: "unrecognized_recipe",
      };
    }
    return {
      id: sample.id,
      provider: sample.provider,
      outcome: "unexpected_error",
      reason: "parser_error",
    };
  }
  if (candidate === null) {
    return {
      id: sample.id,
      provider: sample.provider,
      outcome: "rejected",
      reason: "unrecognized_recipe",
    };
  }
  if (candidate === undefined) {
    return {
      id: sample.id,
      provider: sample.provider,
      outcome: "unexpected_error",
      reason: "parser_returned_no_result",
    };
  }

  let schemaErrors;
  let semanticErrors;
  try {
    schemaErrors = validateJsonSchema(candidate, schema);
    semanticErrors = validateImportCandidate(candidate);
  } catch {
    return {
      id: sample.id,
      provider: sample.provider,
      outcome: "invalid_candidate",
      reason: "candidate_validation_failed",
      validationErrorCount: 1,
    };
  }
  const validationErrorCount = schemaErrors.length + semanticErrors.length;
  if (validationErrorCount > 0) {
    return {
      id: sample.id,
      provider: sample.provider,
      outcome: "invalid_candidate",
      reason: "candidate_validation_failed",
      validationErrorCount,
    };
  }
  if (candidate.status === "unsupported") {
    return {
      id: sample.id,
      provider: sample.provider,
      outcome: "unsupported",
      reason: "parser_reported_unsupported",
      parserId: candidate.parser.id,
      parserTier: candidate.parser.tier,
      status: candidate.status,
      warningCodes: [...new Set(candidate.warnings.map(({ code }) => code))].sort(compareText),
    };
  }
  return summarizeAcceptedSample(sample, candidate);
}

function emptyCounts() {
  return {
    accepted: 0,
    rejected: 0,
    invalidCandidates: 0,
    unexpectedErrors: 0,
    htmlAccepted: 0,
    plainTextAccepted: 0,
    ready: 0,
    needsReview: 0,
    unsupported: 0,
    titlePresent: 0,
    descriptionPresent: 0,
    titleExact: 0,
    titleContained: 0,
    titleMismatch: 0,
    ingredientSections: 0,
    ingredientItems: 0,
    instructionSections: 0,
    instructionSteps: 0,
  };
}

function addSample(counts, sample) {
  if (sample.outcome === "rejected") {
    counts.rejected += 1;
    return;
  }
  if (sample.outcome === "invalid_candidate") {
    counts.invalidCandidates += 1;
    return;
  }
  if (sample.outcome === "unexpected_error") {
    counts.unexpectedErrors += 1;
    return;
  }
  if (sample.outcome === "unsupported") {
    counts.unsupported += 1;
    return;
  }

  counts.accepted += 1;
  if (sample.parserTier === "generic_html") counts.htmlAccepted += 1;
  if (sample.parserTier === "generic_text") counts.plainTextAccepted += 1;
  if (sample.status === "ready") counts.ready += 1;
  if (sample.status === "needs_review") counts.needsReview += 1;
  if (sample.titlePresent) counts.titlePresent += 1;
  if (sample.descriptionPresent) counts.descriptionPresent += 1;
  if (sample.titleAgreement === "exact") counts.titleExact += 1;
  if (sample.titleAgreement === "contained") counts.titleContained += 1;
  if (sample.titleAgreement === "mismatch") counts.titleMismatch += 1;
  counts.ingredientSections += sample.ingredientSections;
  counts.ingredientItems += sample.ingredientItems;
  counts.instructionSections += sample.instructionSections;
  counts.instructionSteps += sample.instructionSteps;
}

function reviewSignature(sample) {
  return [sample.outcome, sample.parserTier, sample.status, sample.titleAgreement].join(":");
}

function selectReviewSampleIds(samples) {
  const ordered = [...samples].sort(
    (left, right) =>
      outcomeOrder.get(left.outcome) - outcomeOrder.get(right.outcome) ||
      compareText(left.id, right.id),
  );
  const selected = [];
  const signatures = new Set();
  for (const sample of ordered) {
    const signature = reviewSignature(sample);
    if (signatures.has(signature)) continue;
    signatures.add(signature);
    selected.push(sample.id);
    if (selected.length === 5) return selected;
  }
  for (const sample of ordered) {
    if (!selected.includes(sample.id)) selected.push(sample.id);
    if (selected.length === 5) break;
  }
  return selected;
}

function buildProvider(provider, samples) {
  const counts = emptyCounts();
  for (const sample of samples) addSample(counts, sample);
  return {
    provider,
    sampleCount: samples.length,
    accepted: counts.accepted,
    rejected: counts.rejected,
    invalidCandidates: counts.invalidCandidates,
    unexpectedErrors: counts.unexpectedErrors,
    acceptancePercent: percent(counts.accepted, samples.length),
    htmlAccepted: counts.htmlAccepted,
    plainTextAccepted: counts.plainTextAccepted,
    ready: counts.ready,
    needsReview: counts.needsReview,
    unsupported: counts.unsupported,
    titlePresent: counts.titlePresent,
    descriptionPresent: counts.descriptionPresent,
    titleExact: counts.titleExact,
    titleContained: counts.titleContained,
    titleMismatch: counts.titleMismatch,
    ingredientItems: counts.ingredientItems,
    instructionSteps: counts.instructionSteps,
    reviewSampleIds: selectReviewSampleIds(samples),
    rejectedSampleIds: samples
      .filter(({ outcome }) => outcome === "rejected")
      .map(({ id }) => id)
      .sort(compareText),
  };
}

/** Evaluate structural coverage without retaining extracted titles, URLs, or recipe bodies. */
export async function evaluateParserCorpus({ manifestPath, dataRoot } = {}) {
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

  const sampleIds = new Set();
  for (const sample of manifest.samples) {
    if (!nonEmptyString(sample?.id) || sampleIds.has(sample.id)) {
      throw new Error(`Corpus sample IDs must be non-empty and unique: ${sample?.id}`);
    }
    sampleIds.add(sample.id);
  }

  const schema = JSON.parse(await fs.readFile(schemaPath, "utf8"));
  const samples = [];
  // Sequential evaluation bounds peak memory for the large external clipboard corpus.
  for (const sample of manifest.samples) {
    samples.push(await evaluateSample(sample, resolvedDataRoot, rootRealPath, schema));
  }

  const providerGroups = new Map();
  for (const sample of samples) {
    const providerSamples = providerGroups.get(sample.provider) ?? [];
    providerSamples.push(sample);
    providerGroups.set(sample.provider, providerSamples);
  }
  const providers = Array.from(providerGroups, ([provider, providerSamples]) =>
    buildProvider(provider, providerSamples),
  ).sort((left, right) => compareText(left.provider, right.provider));

  const counts = emptyCounts();
  for (const sample of samples) addSample(counts, sample);
  return {
    corpus: {
      schemaVersion: manifest.schemaVersion ?? null,
      corpusId: manifest.corpusId ?? null,
    },
    summary: {
      providers: providers.length,
      samples: samples.length,
      accepted: counts.accepted,
      rejected: counts.rejected,
      invalidCandidates: counts.invalidCandidates,
      unexpectedErrors: counts.unexpectedErrors,
      acceptancePercent: percent(counts.accepted, samples.length),
      htmlAccepted: counts.htmlAccepted,
      plainTextAccepted: counts.plainTextAccepted,
      ready: counts.ready,
      needsReview: counts.needsReview,
      unsupported: counts.unsupported,
      titlePresent: counts.titlePresent,
      descriptionPresent: counts.descriptionPresent,
      titleExact: counts.titleExact,
      titleContained: counts.titleContained,
      titleMismatch: counts.titleMismatch,
      ingredientSections: counts.ingredientSections,
      ingredientItems: counts.ingredientItems,
      instructionSections: counts.instructionSections,
      instructionSteps: counts.instructionSteps,
      integrityVerified: true,
    },
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

function formatPercent(value) {
  return `${value}%`;
}

/** Render aggregate results, sample IDs, and explicit manual-review limits. */
export function renderMarkdown(evaluation) {
  const { summary } = evaluation;
  const lines = [
    "# NOMS-016 Real-Corpus Parser Evaluation",
    "",
    "This report contains aggregate parser measurements and corpus sample IDs only. It does not add extracted titles or retain source URLs, ingredients, or instruction text; corpus-assigned IDs can contain human-readable recipe names.",
    "",
    "## Summary",
    "",
    `- Structural acceptance: ${summary.accepted} of ${summary.samples} (${formatPercent(summary.acceptancePercent)})`,
    `- Rejected as unrecognized: ${summary.rejected}`,
    `- Invalid candidates: ${summary.invalidCandidates}`,
    `- Unexpected parser errors: ${summary.unexpectedErrors}`,
    `- Explicitly unsupported candidates: ${summary.unsupported}`,
    `- HTML / plain-text accepted: ${summary.htmlAccepted} / ${summary.plainTextAccepted}`,
    `- Accepted candidates with a title: ${summary.titlePresent} of ${summary.accepted}`,
    `- Captured-page title mismatches: ${summary.titleMismatch}`,
    `- Integrity verified: ${summary.integrityVerified ? "yes" : "no"}`,
    "",
    "## Provider coverage",
    "",
    "| Provider | Samples | Accepted | Acceptance | HTML | Text | Rejected | Invalid | Errors | Needs review | Title | Title mismatch | Unsupported |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
  ];
  for (const provider of evaluation.providers) {
    lines.push(
      `| ${markdownCell(provider.provider)} | ${provider.sampleCount} | ${provider.accepted} | ${formatPercent(provider.acceptancePercent)} | ${provider.htmlAccepted} | ${provider.plainTextAccepted} | ${provider.rejected} | ${provider.invalidCandidates} | ${provider.unexpectedErrors} | ${provider.needsReview} | ${provider.titlePresent} | ${provider.titleMismatch} | ${provider.unsupported} |`,
    );
  }

  lines.push(
    "",
    "## Representative manual-review samples",
    "",
    "These deterministic IDs cover observed parser modes and rejection outcomes; accepted candidates still require semantic review.",
    "",
    "| Provider | Sample IDs |",
    "| --- | --- |",
  );
  for (const provider of evaluation.providers) {
    lines.push(
      `| ${markdownCell(provider.provider)} | ${provider.reviewSampleIds.map(markdownCell).join("<br>")} |`,
    );
  }

  lines.push("", "## Rejected sample IDs", "");
  const rejectedProviders = evaluation.providers.filter(
    ({ rejectedSampleIds }) => rejectedSampleIds.length > 0,
  );
  if (rejectedProviders.length === 0) {
    lines.push("No samples were rejected.");
  } else {
    for (const provider of rejectedProviders) {
      lines.push(`### ${markdownCell(provider.provider)}`, "");
      for (const sampleId of provider.rejectedSampleIds) lines.push(`- ${markdownCell(sampleId)}`);
      lines.push("");
    }
  }

  lines.push(
    "## Interpretation limits",
    "",
    "- Structural acceptance does not prove semantic correctness. A candidate may contain the wrong title, ingredients, instructions, section boundaries, or quantities.",
    "- Title agreement is a normalized string comparison with the captured browser page title; it is a triage signal, not a correctness oracle.",
    "- Rejection means the current parser did not recognize the captured clipboard representations. It does not establish that the provider is unsupported in every browser or page shape.",
    "- The HTML and plain-text files were not necessarily captured in the same clipboard event, so representation fallback results must be interpreted accordingly.",
    "- Representative IDs require human review before they become golden expectations or compatibility claims.",
    "",
  );
  return `${lines.join("\n").trimEnd()}\n`;
}

function parseCliArguments(argumentsList) {
  const options = {};
  for (let index = 0; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--help" || argument === "-h") return { help: true };
    const optionName = {
      "--manifest": "manifestPath",
      "--data-root": "dataRoot",
      "--json-out": "jsonOut",
      "--markdown-out": "markdownOut",
    }[argument];
    if (!optionName) throw new Error(`Unknown argument: ${argument}`);
    const value = argumentsList[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${argument}`);
    options[optionName] = value;
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
      "Usage: node scripts/evaluate-recipe-parser-corpus.mjs --manifest PATH [--data-root PATH] [--json-out PATH] [--markdown-out PATH]\n",
    );
    return;
  }
  if (!options.manifestPath) throw new Error("--manifest is required");

  const evaluation = await evaluateParserCorpus(options);
  if (options.jsonOut) {
    await writeOutput(options.jsonOut, `${JSON.stringify(evaluation, null, 2)}\n`);
  }
  if (options.markdownOut) await writeOutput(options.markdownOut, renderMarkdown(evaluation));
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
