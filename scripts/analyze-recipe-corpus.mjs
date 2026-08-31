#!/usr/bin/env node

import { createHash } from "node:crypto";
import { createReadStream, promises as fs } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const FEATURE_NAMES = [
  "clipboardFragment",
  "schemaOrgRecipe",
  "recipeHeading",
  "ingredientRegion",
  "instructionRegion",
  "groupedIngredients",
  "ingredientLists",
  "instructionLists",
  "duplicateIngredientRegion",
  "duplicateInstructionRegion",
  "formControls",
  "unitControls",
  "yieldControls",
  "notes",
  "equipment",
  "images",
  "remoteResources",
  "scripts",
  "styles",
  "eventHandlers",
  "iframes",
  "links",
  "jsonLd",
];

const FEATURE_LABELS = {
  clipboardFragment: "Clipboard fragment markers",
  schemaOrgRecipe: "Schema.org Recipe markup",
  recipeHeading: "Recipe/title heading",
  ingredientRegion: "Ingredient region",
  instructionRegion: "Instruction region",
  groupedIngredients: "Grouped ingredients",
  ingredientLists: "Ingredient list markup",
  instructionLists: "Instruction list markup",
  duplicateIngredientRegion: "Duplicate ingredient regions",
  duplicateInstructionRegion: "Duplicate instruction regions",
  formControls: "Form controls",
  unitControls: "Unit controls",
  yieldControls: "Yield/servings controls",
  notes: "Notes region",
  equipment: "Equipment region",
  images: "Images",
  remoteResources: "Remote resource references",
  scripts: "Scripts",
  styles: "Style markup",
  eventHandlers: "Inline event handlers",
  iframes: "Iframes",
  links: "Links",
  jsonLd: "JSON-LD",
};

const INGREDIENT_HEADING = /\bingredients?\b/i;
const INSTRUCTION_HEADING = /\b(directions?|instructions?|method|preparation|steps?)\b/i;
const NOTES_HEADING = /\b(notes?|tips?)\b/i;
const EQUIPMENT_HEADING = /\b(equipment|tools?|utensils?)\b/i;

function decodeHeading(value) {
  return value
    .replace(/<[^>]*>/g, " ")
    .replace(/&(?:nbsp|#160);/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&quot;|&#34;/gi, '"')
    .replace(/&#39;|&apos;/gi, "'")
    .replace(/\s+/g, " ")
    .trim();
}

function extractHeadings(html) {
  const headings = [];
  const expression = /<h([1-6])\b[^>]*>([\s\S]*?)<\/h\1\s*>/gi;
  for (const match of html.matchAll(expression)) {
    headings.push({ level: Number(match[1]), text: decodeHeading(match[2]) });
  }
  return headings;
}

function hasGroupedIngredients(headings) {
  for (let index = 0; index < headings.length; index += 1) {
    const heading = headings[index];
    if (!INGREDIENT_HEADING.test(heading.text)) continue;

    for (let nextIndex = index + 1; nextIndex < headings.length; nextIndex += 1) {
      const candidate = headings[nextIndex];
      if (INSTRUCTION_HEADING.test(candidate.text)) break;
      if (INGREDIENT_HEADING.test(candidate.text)) continue;
      if (candidate.level > heading.level && candidate.text) return true;
    }
  }
  return false;
}

function controlMarkup(html) {
  const controls = [];
  const expression = /<(?:button|select|textarea)\b[^>]*>[\s\S]*?<\/(?:button|select|textarea)\s*>|<(?:input|option|form)\b[^>]*>/gi;
  for (const match of html.matchAll(expression)) controls.push(match[0]);
  return controls.join(" ");
}

function regionContainsList(html, headingExpression) {
  const headingPattern = /<h[1-6]\b[^>]*>([\s\S]*?)<\/h[1-6]\s*>/gi;
  const matches = Array.from(html.matchAll(headingPattern));
  for (let index = 0; index < matches.length; index += 1) {
    if (!headingExpression.test(decodeHeading(matches[index][1]))) continue;
    const start = matches[index].index + matches[index][0].length;
    const end = matches[index + 1]?.index ?? html.length;
    if (/<(?:ul|ol)\b/i.test(html.slice(start, end))) return true;
  }
  return false;
}

function characterizeHtml(html) {
  const headings = extractHeadings(html);
  const ingredientHeadings = headings.filter(({ text }) => INGREDIENT_HEADING.test(text));
  const instructionHeadings = headings.filter(({ text }) => INSTRUCTION_HEADING.test(text));
  const controls = controlMarkup(html);
  const hasFormControls = /<(?:button|input|select|textarea|option|form)\b/i.test(html);

  return {
    clipboardFragment: /<!--\s*(?:Start|End)Fragment\s*-->/i.test(html),
    schemaOrgRecipe:
      /(?:https?:\/\/)?schema\.org\/Recipe\b/i.test(html) ||
      /itemtype\s*=\s*["'][^"']*Recipe\b/i.test(html) ||
      /["']@type["']\s*:\s*(?:["']Recipe["']|\[[^\]]*["']Recipe["'][^\]]*\])/i.test(html),
    recipeHeading: headings.some(({ level }) => level === 1),
    ingredientRegion: ingredientHeadings.length > 0,
    instructionRegion: instructionHeadings.length > 0,
    groupedIngredients: hasGroupedIngredients(headings),
    ingredientLists: regionContainsList(html, INGREDIENT_HEADING),
    instructionLists: regionContainsList(html, INSTRUCTION_HEADING),
    duplicateIngredientRegion: ingredientHeadings.length > 1,
    duplicateInstructionRegion: instructionHeadings.length > 1,
    formControls: hasFormControls,
    unitControls: hasFormControls && /\b(metric|imperial|units?|us customary)\b/i.test(controls),
    yieldControls: hasFormControls && /\b(servings?|yield|portions?)\b/i.test(controls),
    notes: headings.some(({ text }) => NOTES_HEADING.test(text)),
    equipment: headings.some(({ text }) => EQUIPMENT_HEADING.test(text)),
    images: /<(?:img|picture|source)\b/i.test(html),
    remoteResources: /\b(?:src|href|poster)\s*=\s*["']\s*(?:https?:)?\/\//i.test(html),
    scripts: /<script\b/i.test(html),
    styles: /<style\b|\sstyle\s*=/i.test(html),
    eventHandlers: /\son[a-z]+\s*=/i.test(html),
    iframes: /<iframe\b/i.test(html),
    links: /<a\b[^>]*\bhref\s*=/i.test(html),
    jsonLd: /<script\b[^>]*\btype\s*=\s*["']application\/ld\+json["']/i.test(html),
  };
}

function emptyFeatureCounts() {
  return Object.fromEntries(FEATURE_NAMES.map((feature) => [feature, 0]));
}

function structuralSignature(features) {
  return FEATURE_NAMES.filter((feature) => features[feature]).join("+") || "none";
}

function compareText(left, right) {
  if (left === right) return 0;
  return left < right ? -1 : 1;
}

function percentile(sortedValues, proportion) {
  if (sortedValues.length === 0) return 0;
  const rank = Math.ceil(proportion * sortedValues.length) - 1;
  return sortedValues[Math.max(0, rank)];
}

function distribution(values) {
  const sortedValues = [...values].sort((left, right) => left - right);
  const sum = sortedValues.reduce((total, value) => total + value, 0);
  return {
    min: sortedValues[0] ?? 0,
    p50: percentile(sortedValues, 0.5),
    p90: percentile(sortedValues, 0.9),
    p95: percentile(sortedValues, 0.95),
    max: sortedValues.at(-1) ?? 0,
    mean: sortedValues.length === 0 ? 0 : Math.round(sum / sortedValues.length),
  };
}

function signatureDistance(leftSignature, rightSignature) {
  const leftFeatures = new Set(leftSignature.split("+"));
  const rightFeatures = new Set(rightSignature.split("+"));
  return FEATURE_NAMES.reduce(
    (distance, feature) => distance + (leftFeatures.has(feature) !== rightFeatures.has(feature) ? 1 : 0),
    0,
  );
}

function selectFixtureSampleIds(clusters, sampleCount) {
  if (clusters.length === 0) return [];
  const targetCount = Math.min(5, sampleCount);
  const selectedClusters = [clusters[0]];

  while (selectedClusters.length < Math.min(targetCount, clusters.length)) {
    const candidates = clusters.filter((cluster) => !selectedClusters.includes(cluster));
    candidates.sort((left, right) => {
      const leftDistance = Math.min(
        ...selectedClusters.map((selected) => signatureDistance(left.signature, selected.signature)),
      );
      const rightDistance = Math.min(
        ...selectedClusters.map((selected) => signatureDistance(right.signature, selected.signature)),
      );
      return (
        rightDistance - leftDistance ||
        right.sampleCount - left.sampleCount ||
        compareText(left.signature, right.signature)
      );
    });
    selectedClusters.push(candidates[0]);
  }

  const selectedIds = selectedClusters.map((cluster) => cluster.sampleIds[0]);
  for (const cluster of clusters) {
    for (const sampleId of cluster.sampleIds) {
      if (selectedIds.length >= targetCount) return selectedIds;
      if (!selectedIds.includes(sampleId)) selectedIds.push(sampleId);
    }
  }
  return selectedIds;
}

function buildFixturePriorities(featureCounts, sampleCount) {
  const priorities = [];
  const missingIngredientRegions = sampleCount - featureCounts.ingredientRegion;
  const missingInstructionRegions = sampleCount - featureCounts.instructionRegion;
  if (missingIngredientRegions > 0 || missingInstructionRegions > 0) {
    priorities.push(
      `Exercise missing or ambiguous recipe regions (${missingIngredientRegions} ingredient; ${missingInstructionRegions} instruction).`,
    );
  }
  if (featureCounts.duplicateIngredientRegion > 0) {
    priorities.push(
      `Exercise duplicate ingredient-region disambiguation (${featureCounts.duplicateIngredientRegion}/${sampleCount} samples).`,
    );
  }
  if (featureCounts.duplicateInstructionRegion > 0) {
    priorities.push(
      `Exercise duplicate instruction-region disambiguation (${featureCounts.duplicateInstructionRegion}/${sampleCount} samples).`,
    );
  }
  if (featureCounts.unitControls > 0 && featureCounts.yieldControls > 0) {
    priorities.push(
      `Exercise unit and yield controls without importing form chrome (${featureCounts.unitControls}/${sampleCount} samples expose unit signals; ${featureCounts.yieldControls}/${sampleCount} expose yield signals).`,
    );
  } else if (featureCounts.unitControls > 0 || featureCounts.yieldControls > 0) {
    priorities.push(
      `Exercise ${featureCounts.unitControls > 0 ? "unit" : "yield"} controls without importing form chrome.`,
    );
  }
  if (featureCounts.groupedIngredients > 0) {
    priorities.push(
      `Preserve grouped ingredient boundaries (${featureCounts.groupedIngredients}/${sampleCount} samples).`,
    );
  }
  if (featureCounts.notes > 0 || featureCounts.equipment > 0) {
    priorities.push("Separate notes and equipment from ingredients and instructions.");
  }
  if (featureCounts.remoteResources > 0 || featureCounts.scripts > 0 || featureCounts.iframes > 0) {
    priorities.push("Remove active and remote page resources during sanitization.");
  }
  if (featureCounts.schemaOrgRecipe > 0) {
    priorities.push("Compare semantic Recipe markup with the visible structural fallback.");
  }
  if (priorities.length === 0) priorities.push("Exercise the dominant structural cluster as a provider baseline.");
  return priorities;
}

async function digestFile(filePath, includeContent) {
  const digest = createHash("sha256");
  const chunks = includeContent ? [] : null;
  let bytes = 0;
  for await (const chunk of createReadStream(filePath)) {
    bytes += chunk.byteLength;
    digest.update(chunk);
    if (chunks) chunks.push(chunk);
  }
  return {
    bytes,
    sha256: digest.digest("hex"),
    content: chunks ? Buffer.concat(chunks, bytes) : undefined,
  };
}

function resolveCorpusPath(dataRoot, relativePath) {
  if (typeof relativePath !== "string" || relativePath.length === 0) {
    throw new Error("Every corpus file must have a non-empty path");
  }
  const root = path.resolve(dataRoot);
  const resolved = path.resolve(root, relativePath);
  if (resolved !== root && !resolved.startsWith(`${root}${path.sep}`)) {
    throw new Error(`Corpus path escapes data root: ${relativePath}`);
  }
  return resolved;
}

async function validateFile(dataRoot, sampleId, descriptor, includeContent = false) {
  if (!descriptor || !Number.isSafeInteger(descriptor.bytes) || descriptor.bytes < 0) {
    throw new Error(`Invalid byte count for ${sampleId}`);
  }
  if (typeof descriptor.sha256 !== "string" || !/^[a-f0-9]{64}$/i.test(descriptor.sha256)) {
    throw new Error(`Invalid SHA-256 digest for ${sampleId}`);
  }

  const filePath = resolveCorpusPath(dataRoot, descriptor.path);
  let actual;
  try {
    actual = await digestFile(filePath, includeContent);
  } catch (error) {
    throw new Error(`Unable to read ${descriptor.path}: ${error.message}`, { cause: error });
  }
  if (actual.sha256.toLowerCase() !== descriptor.sha256.toLowerCase()) {
    throw new Error(`Digest mismatch for ${descriptor.path}`);
  }
  if (actual.bytes !== descriptor.bytes) {
    throw new Error(
      `Size mismatch for ${descriptor.path}: expected ${descriptor.bytes}, found ${actual.bytes}`,
    );
  }
  return actual.content;
}

async function analyzeSample(sample, dataRoot) {
  if (!sample || typeof sample.id !== "string" || typeof sample.provider !== "string") {
    throw new Error("Every corpus sample must have an id and provider");
  }
  const plainText = sample.files?.plainText;
  const html = sample.files?.html;
  if (!plainText || !html) throw new Error(`Sample ${sample.id} must reference plain-text and HTML files`);

  await validateFile(dataRoot, sample.id, plainText);
  const htmlContent = (await validateFile(dataRoot, sample.id, html, true)).toString("utf8");

  return {
    id: sample.id,
    provider: sample.provider,
    plainTextBytes: plainText.bytes,
    htmlBytes: html.bytes,
    totalBytes: plainText.bytes + html.bytes,
    features: characterizeHtml(htmlContent),
  };
}

async function mapWithConcurrency(items, concurrency, mapper) {
  const results = new Array(items.length);
  let nextIndex = 0;

  async function worker() {
    while (nextIndex < items.length) {
      const index = nextIndex;
      nextIndex += 1;
      results[index] = await mapper(items[index], index);
    }
  }

  const workerCount = Math.min(concurrency, Math.max(1, items.length));
  await Promise.all(Array.from({ length: workerCount }, () => worker()));
  return results;
}

function buildProviderAnalysis(provider, samples) {
  const sortedSamples = [...samples].sort((left, right) => compareText(left.id, right.id));
  const featureCounts = emptyFeatureCounts();
  const clustersBySignature = new Map();

  for (const sample of sortedSamples) {
    for (const feature of FEATURE_NAMES) {
      if (sample.features[feature]) featureCounts[feature] += 1;
    }
    const signature = structuralSignature(sample.features);
    const clusterSamples = clustersBySignature.get(signature) ?? [];
    clusterSamples.push(sample.id);
    clustersBySignature.set(signature, clusterSamples);
  }

  const rawClusters = Array.from(clustersBySignature, ([signature, sampleIds]) => ({
    signature,
    sampleCount: sampleIds.length,
    sampleIds,
    representativeSampleIds: sampleIds.slice(0, 3),
  })).sort(
    (left, right) =>
      right.sampleCount - left.sampleCount || compareText(left.signature, right.signature),
  );
  const slug = provider.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "provider";
  const recommendedFixtureSampleIds = selectFixtureSampleIds(rawClusters, sortedSamples.length);
  const clusters = rawClusters.map(({ sampleIds: _sampleIds, ...cluster }, index) => ({
    id: `${slug}-${String(index + 1).padStart(2, "0")}`,
    ...cluster,
  }));

  return {
    provider,
    sampleCount: sortedSamples.length,
    clusterCount: clusters.length,
    payloadBytes: {
      plainText: distribution(sortedSamples.map((sample) => sample.plainTextBytes)),
      html: distribution(sortedSamples.map((sample) => sample.htmlBytes)),
      total: distribution(sortedSamples.map((sample) => sample.totalBytes)),
    },
    featureCounts,
    clusters,
    recommendedFixtureSampleIds,
    fixturePriorities: buildFixturePriorities(featureCounts, sortedSamples.length),
  };
}

/** Analyze and integrity-check a recipe corpus without retaining recipe body content. */
export async function analyzeCorpus({ manifestPath, dataRoot } = {}) {
  if (!manifestPath) throw new Error("manifestPath is required");
  const resolvedManifestPath = path.resolve(manifestPath);
  const resolvedDataRoot = path.resolve(dataRoot ?? path.dirname(resolvedManifestPath));
  let manifest;
  try {
    manifest = JSON.parse(await fs.readFile(resolvedManifestPath, "utf8"));
  } catch (error) {
    throw new Error(`Unable to read corpus manifest: ${error.message}`, { cause: error });
  }
  if (!Array.isArray(manifest.samples)) throw new Error("Corpus manifest must contain a samples array");

  const sampleIds = new Set();
  for (const sample of manifest.samples) {
    if (sampleIds.has(sample?.id)) throw new Error(`Duplicate sample id: ${sample.id}`);
    sampleIds.add(sample?.id);
  }

  const samples = await mapWithConcurrency(manifest.samples, 4, (sample) =>
    analyzeSample(sample, resolvedDataRoot),
  );
  const providerGroups = new Map();
  for (const sample of samples) {
    const providerSamples = providerGroups.get(sample.provider) ?? [];
    providerSamples.push(sample);
    providerGroups.set(sample.provider, providerSamples);
  }
  const providers = Array.from(providerGroups, ([provider, providerSamples]) =>
    buildProviderAnalysis(provider, providerSamples),
  ).sort((left, right) => compareText(left.provider, right.provider));

  const plainTextBytes = samples.reduce((total, sample) => total + sample.plainTextBytes, 0);
  const htmlBytes = samples.reduce((total, sample) => total + sample.htmlBytes, 0);
  return {
    corpus: {
      schemaVersion: manifest.schemaVersion ?? null,
      corpusId: manifest.corpusId ?? null,
    },
    summary: {
      providers: providers.length,
      samples: samples.length,
      plainTextBytes,
      htmlBytes,
      totalBytes: plainTextBytes + htmlBytes,
      integrityVerified: true,
    },
    featureNames: [...FEATURE_NAMES],
    providers,
  };
}

function markdownCell(value) {
  return String(value).replace(/\|/g, "\\|").replace(/[\r\n]+/g, " ");
}

function formatBytes(value) {
  return new Intl.NumberFormat("en-US").format(value);
}

/** Render aggregate structural findings and representative sample IDs only. */
export function renderMarkdown(analysis) {
  const lines = [
    "# NOMS-016 Corpus Analysis",
    "",
    "This report contains aggregate measurements, structural classifications, and sample IDs only. It does not reproduce recipe body content.",
    "",
    "## Summary",
    "",
    `- Providers: ${analysis.summary.providers}`,
    `- Samples: ${analysis.summary.samples}`,
    `- Plain-text bytes: ${formatBytes(analysis.summary.plainTextBytes)}`,
    `- HTML bytes: ${formatBytes(analysis.summary.htmlBytes)}`,
    `- Total bytes: ${formatBytes(analysis.summary.totalBytes)}`,
    `- Integrity verified: ${analysis.summary.integrityVerified ? "yes" : "no"}`,
  ];
  const totalClusters = analysis.providers.reduce((total, provider) => total + provider.clusterCount, 0);
  const providersWithMultipleClusters = analysis.providers.filter(
    (provider) => provider.clusterCount > 1,
  ).length;
  const totalFeatureCount = (feature) =>
    analysis.providers.reduce((total, provider) => total + provider.featureCounts[feature], 0);

  lines.push(
    "",
    "## Key findings",
    "",
    `- Integrity checks passed for all ${analysis.summary.samples} paired plain-text and HTML samples.`,
    `- Boolean structural signatures produced ${totalClusters} clusters; ${providersWithMultipleClusters} of ${analysis.summary.providers} providers have more than one observed page shape.`,
    `- Ingredient and instruction headings were detected in ${totalFeatureCount("ingredientRegion")} and ${totalFeatureCount("instructionRegion")} samples, respectively.`,
    `- Sanitization-relevant remote resources, scripts, and iframes were detected in ${totalFeatureCount("remoteResources")}, ${totalFeatureCount("scripts")}, and ${totalFeatureCount("iframes")} samples.`,
    "",
    "## Provider payloads",
    "",
    "| Provider | Samples | Clusters | Text p50 | Text p95 | HTML p50 | HTML p95 | HTML max |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
  );

  for (const provider of analysis.providers) {
    lines.push(
      `| ${markdownCell(provider.provider)} | ${provider.sampleCount} | ${provider.clusterCount} | ${formatBytes(provider.payloadBytes.plainText.p50)} | ${formatBytes(provider.payloadBytes.plainText.p95)} | ${formatBytes(provider.payloadBytes.html.p50)} | ${formatBytes(provider.payloadBytes.html.p95)} | ${formatBytes(provider.payloadBytes.html.max)} |`,
    );
  }

  lines.push(
    "",
    "### Complete payload distributions",
    "",
    "Percentiles use the nearest-rank method; means are rounded to whole bytes.",
    "",
    "| Provider | Payload | Min | p50 | p90 | p95 | Max | Mean |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
  );
  for (const provider of analysis.providers) {
    for (const [payloadName, payload] of Object.entries(provider.payloadBytes)) {
      lines.push(
        `| ${markdownCell(provider.provider)} | ${markdownCell(payloadName)} | ${formatBytes(payload.min)} | ${formatBytes(payload.p50)} | ${formatBytes(payload.p90)} | ${formatBytes(payload.p95)} | ${formatBytes(payload.max)} | ${formatBytes(payload.mean)} |`,
      );
    }
  }

  lines.push("", "## Structural feature counts", "");
  lines.push(`| Provider | ${analysis.featureNames.map((name) => markdownCell(FEATURE_LABELS[name] ?? name)).join(" | ")} |`);
  lines.push(`| --- | ${analysis.featureNames.map(() => "---:").join(" | ")} |`);
  for (const provider of analysis.providers) {
    lines.push(
      `| ${markdownCell(provider.provider)} | ${analysis.featureNames.map((name) => provider.featureCounts[name]).join(" | ")} |`,
    );
  }

  lines.push("", "## Fixture recommendations", "");
  lines.push(
    "Selections start with each provider's dominant cluster, then maximize structural distance across remaining clusters before filling from dominant shapes. Each provider contributes at most five sample IDs.",
    "",
    "| Provider | Recommended sample IDs | Fixture priorities |",
    "| --- | --- | --- |",
  );
  for (const provider of analysis.providers) {
    lines.push(
      `| ${markdownCell(provider.provider)} | ${provider.recommendedFixtureSampleIds.map(markdownCell).join("<br>")} | ${provider.fixturePriorities.map(markdownCell).join("<br>")} |`,
    );
  }

  lines.push("", "## Structural clusters", "");
  for (const provider of analysis.providers) {
    lines.push(`### ${markdownCell(provider.provider)}`, "");
    lines.push("| Cluster | Samples | Structural signature | Representative sample IDs |", "| --- | ---: | --- | --- |");
    for (const cluster of provider.clusters) {
      lines.push(
        `| ${markdownCell(cluster.id)} | ${cluster.sampleCount} | ${markdownCell(cluster.signature)} | ${cluster.representativeSampleIds.map(markdownCell).join("<br>")} |`,
      );
    }
    lines.push("");
  }

  lines.push(
    "## Interpretation limits",
    "",
    "- Feature detection is heuristic and operates on captured clipboard HTML; a count indicates structural evidence, not successful recipe extraction.",
    "- Clusters group identical boolean feature signatures. They do not measure semantic equivalence, visual layout, or parser accuracy.",
    "- Payload percentiles characterize captured files only and should inform, but not solely determine, parser resource limits.",
    "- Recommended sample IDs are test-fixture candidates. Their recipe body content is deliberately excluded from this report and its JSON analysis.",
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
      "Usage: node scripts/analyze-recipe-corpus.mjs --manifest PATH [--data-root PATH] [--json-out PATH] [--markdown-out PATH]\n",
    );
    return;
  }
  if (!options.manifestPath) throw new Error("--manifest is required");

  const analysis = await analyzeCorpus(options);
  if (options.jsonOut) await writeOutput(options.jsonOut, `${JSON.stringify(analysis, null, 2)}\n`);
  if (options.markdownOut) await writeOutput(options.markdownOut, renderMarkdown(analysis));
  if (!options.jsonOut && !options.markdownOut) process.stdout.write(`${JSON.stringify(analysis, null, 2)}\n`);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  runCli().catch((error) => {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exitCode = 1;
  });
}
