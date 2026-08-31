#!/usr/bin/env node

import { promises as fs } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const STATUSES = new Set(["ready", "needs_review", "unsupported"]);
const CONFIDENCES = new Set(["high", "medium", "low"]);
const TIERS = new Set(["generic_html", "generic_text", "source_adapter", "none"]);
const REPRESENTATIONS = new Set(["text/html", "text/plain", "source_url"]);
const SEVERITIES = new Set(["info", "warning", "error"]);

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function validateAllowedKeys(value, at, allowedKeys, errors) {
  if (!isObject(value)) return;
  const allowed = new Set(allowedKeys);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) errors.push(`${at}.${key} is not allowed`);
  }
}

function validateProvenanceArray(value, at, errors, { required = true } = {}) {
  if (!Array.isArray(value)) {
    errors.push(`${at} must be a provenance array`);
    return;
  }
  if (required && value.length === 0) {
    errors.push(`${at} must contain source evidence`);
  }
  value.forEach((entry, index) => {
    const location = `${at}[${index}]`;
    if (!isObject(entry)) {
      errors.push(`${location} must be an object`);
      return;
    }
    validateAllowedKeys(entry, location, ["representation", "locator"], errors);
    if (!REPRESENTATIONS.has(entry.representation)) {
      errors.push(`${location}.representation is not supported`);
    }
    if (!nonEmptyString(entry.locator)) {
      errors.push(`${location}.locator must be a non-empty string`);
    }
  });
}

function validateProvenance(value, at, errors, options) {
  validateProvenanceArray(value, `${at}.provenance`, errors, options);
}

function validateScalarField(value, at, errors, valueValidator = () => true) {
  if (value === null) return;
  if (!isObject(value)) {
    errors.push(`${at} must be null or an object`);
    return;
  }
  validateAllowedKeys(value, at, ["raw", "value", "confidence", "provenance"], errors);
  if (!nonEmptyString(value.raw)) errors.push(`${at}.raw must be a non-empty string`);
  if (!valueValidator(value.value)) errors.push(`${at}.value has an invalid normalized value`);
  if (!CONFIDENCES.has(value.confidence)) errors.push(`${at}.confidence is invalid`);
  validateProvenance(value.provenance, at, errors);
}

function validateSourceUrl(value, at, errors) {
  if (value === null) return;
  if (!nonEmptyString(value)) {
    errors.push(`${at} must be null or a non-empty URL`);
    return;
  }
  try {
    const parsed = new URL(value);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
      errors.push(`${at} uses an unsafe URL scheme`);
    }
  } catch {
    errors.push(`${at} must be a valid URL`);
  }
}

function validateCandidateLine(value, at, errors, normalizedKey = "value", extraKeys = []) {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`);
    return;
  }
  validateAllowedKeys(value, at, ["raw", normalizedKey, "confidence", "provenance", ...extraKeys], errors);
  if (!nonEmptyString(value.raw)) errors.push(`${at}.raw must be a non-empty string`);
  if (!nonEmptyString(value[normalizedKey])) {
    errors.push(`${at}.${normalizedKey} must be a non-empty string`);
  }
  if (!CONFIDENCES.has(value.confidence)) errors.push(`${at}.confidence is invalid`);
  validateProvenance(value.provenance, at, errors);
}

function validateInstruction(value, at, errors) {
  validateCandidateLine(value, at, errors, "text", ["substeps"]);
  if (!isObject(value)) return;
  validateAllowedKeys(value, at, ["raw", "text", "confidence", "provenance", "substeps"], errors);
  if (!Array.isArray(value.substeps)) {
    errors.push(`${at}.substeps must be an array`);
    return;
  }
  value.substeps.forEach((step, index) =>
    validateInstruction(step, `${at}.substeps[${index}]`, errors),
  );
}

export function validateImportCandidate(candidate) {
  const errors = [];
  if (!isObject(candidate)) return ["candidate must be an object"];
  validateAllowedKeys(
    candidate,
    "candidate",
    ["schemaVersion", "status", "parser", "source", "fields", "unclassifiedLines", "competingCandidates", "warnings", "discardedContent"],
    errors,
  );
  if (candidate.schemaVersion !== 1) errors.push("schemaVersion must equal 1");
  if (!STATUSES.has(candidate.status)) errors.push("status is invalid");

  if (!isObject(candidate.parser)) {
    errors.push("parser must be an object");
  } else {
    validateAllowedKeys(candidate.parser, "parser", ["id", "version", "tier", "sourceAdapter"], errors);
    if (!nonEmptyString(candidate.parser.id)) errors.push("parser.id must be a non-empty string");
    if (!nonEmptyString(candidate.parser.version)) errors.push("parser.version must be a non-empty string");
    if (!TIERS.has(candidate.parser.tier)) errors.push("parser.tier is invalid");
    if (candidate.parser.sourceAdapter !== null && !nonEmptyString(candidate.parser.sourceAdapter)) {
      errors.push("parser.sourceAdapter must be null or a non-empty string");
    }
  }

  if (!isObject(candidate.source)) {
    errors.push("source must be an object");
  } else {
    validateAllowedKeys(
      candidate.source,
      "source",
      ["suppliedUrl", "normalizedUrl", "provider", "author", "attribution", "provenance"],
      errors,
    );
    validateSourceUrl(candidate.source.suppliedUrl, "source.suppliedUrl", errors);
    validateSourceUrl(candidate.source.normalizedUrl, "source.normalizedUrl", errors);
    for (const key of ["provider", "author", "attribution"]) {
      if (candidate.source[key] !== null && !nonEmptyString(candidate.source[key])) {
        errors.push(`source.${key} must be null or a non-empty string`);
      }
    }
    const sourceKeys = ["suppliedUrl", "normalizedUrl", "provider", "author", "attribution"];
    if (!isObject(candidate.source.provenance)) {
      errors.push("source.provenance must be an object");
    } else {
      validateAllowedKeys(candidate.source.provenance, "source.provenance", sourceKeys, errors);
      for (const key of sourceKeys) {
        const provenance = candidate.source.provenance[key];
        validateProvenanceArray(provenance, `source.provenance.${key}`, errors, {
          required: candidate.source[key] !== null,
        });
        if (candidate.source[key] === null && Array.isArray(provenance) && provenance.length > 0) {
          errors.push(`source.provenance.${key} must be empty when source.${key} is null`);
        }
      }
    }
  }

  const fields = candidate.fields;
  if (!isObject(fields)) {
    errors.push("fields must be an object");
  } else {
    validateAllowedKeys(
      fields,
      "fields",
      ["title", "description", "prepTime", "cookTime", "totalTime", "yield", "ingredientSections", "instructionSections", "equipment", "notes", "images"],
      errors,
    );
    const stringValue = (value) => nonEmptyString(value);
    const minuteValue = (value) => value === null || (Number.isInteger(value) && value >= 0);
    const yieldValue = (value) =>
      value === null ||
      (isObject(value) &&
        (value.servings === null || (Number.isInteger(value.servings) && value.servings > 0)) &&
        nonEmptyString(value.display));
    validateScalarField(fields.title, "fields.title", errors, stringValue);
    validateScalarField(fields.description, "fields.description", errors, stringValue);
    validateScalarField(fields.prepTime, "fields.prepTime", errors, minuteValue);
    validateScalarField(fields.cookTime, "fields.cookTime", errors, minuteValue);
    validateScalarField(fields.totalTime, "fields.totalTime", errors, minuteValue);
    validateScalarField(fields.yield, "fields.yield", errors, yieldValue);

    if (!Array.isArray(fields.ingredientSections)) {
      errors.push("fields.ingredientSections must be an array");
    } else {
      fields.ingredientSections.forEach((section, sectionIndex) => {
        const at = `fields.ingredientSections[${sectionIndex}]`;
        if (!isObject(section)) {
          errors.push(`${at} must be an object`);
          return;
        }
        validateAllowedKeys(section, at, ["name", "items"], errors);
        if (section.name !== null && !nonEmptyString(section.name)) {
          errors.push(`${at}.name must be null or a non-empty string`);
        }
        if (!Array.isArray(section.items) || section.items.length === 0) {
          errors.push(`${at}.items must be a non-empty array`);
          return;
        }
        section.items.forEach((item, itemIndex) => {
          const itemAt = `${at}.items[${itemIndex}]`;
          if (!isObject(item)) {
            errors.push(`${itemAt} must be an object`);
            return;
          }
          validateAllowedKeys(item, itemAt, ["raw", "amount", "unit", "name", "confidence", "provenance"], errors);
          if (!nonEmptyString(item.raw)) errors.push(`${itemAt}.raw must be a non-empty string`);
          for (const key of ["amount", "unit", "name"]) {
            if (item[key] !== null && !nonEmptyString(item[key])) {
              errors.push(`${itemAt}.${key} must be null or a non-empty string`);
            }
          }
          if (!nonEmptyString(item.name)) errors.push(`${itemAt}.name is required`);
          if (!CONFIDENCES.has(item.confidence)) errors.push(`${itemAt}.confidence is invalid`);
          validateProvenance(item.provenance, itemAt, errors);
        });
      });
    }

    if (!Array.isArray(fields.instructionSections)) {
      errors.push("fields.instructionSections must be an array");
    } else {
      fields.instructionSections.forEach((section, sectionIndex) => {
        const at = `fields.instructionSections[${sectionIndex}]`;
        if (!isObject(section)) {
          errors.push(`${at} must be an object`);
          return;
        }
        validateAllowedKeys(section, at, ["name", "steps"], errors);
        if (section.name !== null && !nonEmptyString(section.name)) {
          errors.push(`${at}.name must be null or a non-empty string`);
        }
        if (!Array.isArray(section.steps) || section.steps.length === 0) {
          errors.push(`${at}.steps must be a non-empty array`);
          return;
        }
        section.steps.forEach((step, index) => validateInstruction(step, `${at}.steps[${index}]`, errors));
      });
    }

    for (const key of ["equipment", "notes"]) {
      if (!Array.isArray(fields[key])) {
        errors.push(`fields.${key} must be an array`);
      } else {
        fields[key].forEach((item, index) =>
          validateCandidateLine(item, `fields.${key}[${index}]`, errors),
        );
      }
    }

    if (!Array.isArray(fields.images)) {
      errors.push("fields.images must be an array");
    } else {
      fields.images.forEach((image, index) => {
        const at = `fields.images[${index}]`;
        if (!isObject(image)) {
          errors.push(`${at} must be an object`);
          return;
        }
        validateAllowedKeys(image, at, ["sourceUrl", "alt", "credit", "provenance"], errors);
        validateSourceUrl(image.sourceUrl, `${at}.sourceUrl`, errors);
        if (image.sourceUrl === null) errors.push(`${at}.sourceUrl is required`);
        for (const key of ["alt", "credit"]) {
          if (image[key] !== null && !nonEmptyString(image[key])) {
            errors.push(`${at}.${key} must be null or a non-empty string`);
          }
        }
        validateProvenance(image.provenance, at, errors);
      });
    }
  }

  for (const key of ["unclassifiedLines", "competingCandidates", "warnings", "discardedContent"]) {
    if (!Array.isArray(candidate[key])) errors.push(`${key} must be an array`);
  }
  if (Array.isArray(candidate.unclassifiedLines)) {
    candidate.unclassifiedLines.forEach((line, index) => {
      const at = `unclassifiedLines[${index}]`;
      if (!isObject(line) || !nonEmptyString(line.raw)) errors.push(`${at}.raw must be a non-empty string`);
      if (isObject(line)) {
        validateAllowedKeys(line, at, ["raw", "provenance"], errors);
        validateProvenance(line.provenance, at, errors);
      }
    });
  }
  if (Array.isArray(candidate.competingCandidates)) {
    candidate.competingCandidates.forEach((competing, index) => {
      const at = `competingCandidates[${index}]`;
      if (!isObject(competing)) {
        errors.push(`${at} must be an object`);
        return;
      }
      validateAllowedKeys(competing, at, ["title", "reason", "provenance"], errors);
      if (competing.title !== null && !nonEmptyString(competing.title)) {
        errors.push(`${at}.title must be null or a non-empty string`);
      }
      if (!nonEmptyString(competing.reason)) errors.push(`${at}.reason must be a non-empty string`);
      validateProvenance(competing.provenance, at, errors);
    });
  }
  if (Array.isArray(candidate.warnings)) {
    candidate.warnings.forEach((warning, index) => {
      const at = `warnings[${index}]`;
      if (!isObject(warning)) {
        errors.push(`${at} must be an object`);
        return;
      }
      validateAllowedKeys(warning, at, ["code", "severity", "path", "message"], errors);
      if (!/^[A-Z][A-Z0-9_]*$/.test(warning.code ?? "")) errors.push(`${at}.code is invalid`);
      if (!SEVERITIES.has(warning.severity)) errors.push(`${at}.severity is invalid`);
      if (!nonEmptyString(warning.path)) errors.push(`${at}.path must be a non-empty string`);
      if (!nonEmptyString(warning.message)) errors.push(`${at}.message must be a non-empty string`);
    });
  }
  if (Array.isArray(candidate.discardedContent)) {
    candidate.discardedContent.forEach((discarded, index) => {
      const at = `discardedContent[${index}]`;
      if (!isObject(discarded)) {
        errors.push(`${at} must be an object`);
        return;
      }
      validateAllowedKeys(discarded, at, ["kind", "count", "reason"], errors);
      if (!nonEmptyString(discarded.kind)) errors.push(`${at}.kind must be a non-empty string`);
      if (!Number.isInteger(discarded.count) || discarded.count < 1) {
        errors.push(`${at}.count must be a positive integer`);
      }
      if (!nonEmptyString(discarded.reason)) errors.push(`${at}.reason must be a non-empty string`);
    });
  }

  const ingredientCount = Array.isArray(fields?.ingredientSections)
    ? fields.ingredientSections.reduce((count, section) => count + (section.items?.length ?? 0), 0)
    : 0;
  const instructionCount = Array.isArray(fields?.instructionSections)
    ? fields.instructionSections.reduce((count, section) => count + (section.steps?.length ?? 0), 0)
    : 0;
  if (candidate.status !== "unsupported" && ingredientCount === 0) {
    errors.push("supported candidates require at least one ingredient");
  }
  if (candidate.status !== "unsupported" && instructionCount === 0) {
    errors.push("supported candidates require at least one instruction");
  }
  if (candidate.status === "unsupported") {
    if ((candidate.unclassifiedLines?.length ?? 0) === 0) {
      errors.push("unsupported candidates must preserve unclassified source evidence");
    }
    if (!candidate.warnings?.some((warning) => warning.severity === "error")) {
      errors.push("unsupported candidates must include an error warning");
    }
  }
  return errors;
}

function resolveInside(root, relativePath) {
  if (!nonEmptyString(relativePath) || path.isAbsolute(relativePath)) {
    throw new Error(`Fixture path must be relative: ${relativePath}`);
  }
  const resolved = path.resolve(root, relativePath);
  if (resolved !== root && !resolved.startsWith(`${root}${path.sep}`)) {
    throw new Error(`Fixture path escapes corpus root: ${relativePath}`);
  }
  return resolved;
}

async function readOptional(root, relativePath) {
  return relativePath === null ? null : fs.readFile(resolveInside(root, relativePath), "utf8");
}

export async function loadAndValidateFixtureCorpus(fixtureRoot) {
  const root = path.resolve(fixtureRoot);
  const manifest = JSON.parse(await fs.readFile(path.join(root, "manifest.json"), "utf8"));
  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.fixtures)) {
    throw new Error("Fixture manifest must use schemaVersion 1 and contain fixtures");
  }
  const schema = JSON.parse(
    await fs.readFile(resolveInside(root, manifest.contract), "utf8"),
  );
  if (schema.properties?.schemaVersion?.const !== manifest.schemaVersion) {
    throw new Error("Fixture manifest and import candidate contract versions differ");
  }
  const seen = new Set();
  const fixtures = [];
  for (const fixture of manifest.fixtures) {
    if (!nonEmptyString(fixture.id) || seen.has(fixture.id)) {
      throw new Error(`Fixture IDs must be non-empty and unique: ${fixture.id}`);
    }
    seen.add(fixture.id);
    if (!Array.isArray(fixture.risks) || fixture.risks.length === 0) {
      throw new Error(`Fixture ${fixture.id} must name at least one risk`);
    }
    const html = await readOptional(root, fixture.input.html);
    const plainText = await readOptional(root, fixture.input.plainText);
    if (html === null && plainText === null) throw new Error(`Fixture ${fixture.id} has no input`);
    if (html !== null && Buffer.byteLength(html) >= 20_000) throw new Error(`Fixture ${fixture.id} HTML is not compact`);
    if (plainText !== null && Buffer.byteLength(plainText) >= 10_000) throw new Error(`Fixture ${fixture.id} text is not compact`);
    const expected = JSON.parse(await fs.readFile(resolveInside(root, fixture.expected), "utf8"));
    const errors = validateImportCandidate(expected);
    if (errors.length > 0) {
      throw new Error(`Fixture ${fixture.id} expected candidate is invalid:\n- ${errors.join("\n- ")}`);
    }
    fixtures.push({ fixture, input: { html, plainText }, expected });
  }
  return { manifest, schema, fixtures };
}

async function main() {
  const defaultRoot = fileURLToPath(
    new URL("../tests/fixtures/recipe-import/v1", import.meta.url),
  );
  const fixtureRoot = process.argv[2] ? path.resolve(process.argv[2]) : defaultRoot;
  const corpus = await loadAndValidateFixtureCorpus(fixtureRoot);
  process.stdout.write(`Validated ${corpus.fixtures.length} recipe import fixtures.\n`);
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : null;
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error.stack ?? error}\n`);
    process.exitCode = 1;
  });
}
