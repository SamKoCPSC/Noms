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
const JSON_SCHEMA_TYPES = new Set(["array", "boolean", "integer", "null", "number", "object", "string"]);
const JSON_SCHEMA_KEYWORDS = new Set([
  "$schema",
  "$id",
  "$ref",
  "$defs",
  "title",
  "description",
  "type",
  "const",
  "enum",
  "required",
  "properties",
  "additionalProperties",
  "oneOf",
  "allOf",
  "if",
  "then",
  "else",
  "items",
  "minItems",
  "maxItems",
  "minLength",
  "minimum",
  "pattern",
  "format",
]);

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function schemaChildEntries(schema) {
  const children = [];
  for (const keyword of ["properties", "$defs"]) {
    if (isObject(schema[keyword])) children.push(...Object.entries(schema[keyword]));
  }
  for (const keyword of ["items", "additionalProperties", "if", "then", "else"]) {
    if (isObject(schema[keyword])) children.push([keyword, schema[keyword]]);
  }
  for (const keyword of ["oneOf", "allOf"]) {
    if (Array.isArray(schema[keyword])) {
      schema[keyword].forEach((child, index) => children.push([`${keyword}[${index}]`, child]));
    }
  }
  return children;
}

function assertSupportedSchema(schema, at = "schema") {
  if (!isObject(schema)) throw new Error(`${at} must be a schema object`);
  for (const keyword of Object.keys(schema)) {
    if (!JSON_SCHEMA_KEYWORDS.has(keyword)) {
      throw new Error(`${at} uses unsupported JSON Schema keyword: ${keyword}`);
    }
  }
  const types = Array.isArray(schema.type) ? schema.type : schema.type === undefined ? [] : [schema.type];
  for (const type of types) {
    if (!JSON_SCHEMA_TYPES.has(type)) throw new Error(`${at}.type is unsupported: ${type}`);
  }
  if (schema.format !== undefined && schema.format !== "uri") {
    throw new Error(`${at}.format is unsupported: ${schema.format}`);
  }
  if (schema.$ref !== undefined && (!nonEmptyString(schema.$ref) || !schema.$ref.startsWith("#/"))) {
    throw new Error(`${at} supports only local JSON Pointer references`);
  }
  if (schema.pattern !== undefined) {
    try {
      new RegExp(schema.pattern);
    } catch {
      throw new Error(`${at}.pattern is not a valid regular expression`);
    }
  }
  for (const [name, child] of schemaChildEntries(schema)) {
    assertSupportedSchema(child, `${at}.${name}`);
  }
}

function resolveLocalReference(rootSchema, reference) {
  let current = rootSchema;
  for (const rawSegment of reference.slice(2).split("/")) {
    const segment = rawSegment.replaceAll("~1", "/").replaceAll("~0", "~");
    if (!isObject(current) || !Object.hasOwn(current, segment)) {
      throw new Error(`Unresolvable local JSON Schema reference: ${reference}`);
    }
    current = current[segment];
  }
  if (!isObject(current)) throw new Error(`JSON Schema reference is not an object: ${reference}`);
  return current;
}

function valueHasType(value, type) {
  if (type === "null") return value === null;
  if (type === "array") return Array.isArray(value);
  if (type === "object") return isObject(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "number") return typeof value === "number" && Number.isFinite(value);
  return typeof value === type;
}

function sameJsonValue(left, right) {
  if (Object.is(left, right)) return true;
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => sameJsonValue(value, right[index]));
  }
  if (isObject(left) && isObject(right)) {
    const leftKeys = Object.keys(left).sort();
    const rightKeys = Object.keys(right).sort();
    return sameJsonValue(leftKeys, rightKeys) && leftKeys.every((key) => sameJsonValue(left[key], right[key]));
  }
  return false;
}

function containsLowConfidence(value) {
  if (Array.isArray(value)) return value.some(containsLowConfidence);
  if (!isObject(value)) return false;
  if (value.confidence === "low") return true;
  return Object.values(value).some(containsLowConfidence);
}

function evaluateSchema(value, schema, rootSchema, at, errors) {
  if (schema.$ref !== undefined) {
    evaluateSchema(value, resolveLocalReference(rootSchema, schema.$ref), rootSchema, at, errors);
  }
  if (schema.type !== undefined) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((type) => valueHasType(value, type))) {
      errors.push(`${at} must have type ${types.join(" or ")}`);
      return;
    }
  }
  if (Object.hasOwn(schema, "const") && !sameJsonValue(value, schema.const)) {
    errors.push(`${at} must equal the schema constant`);
  }
  if (schema.enum !== undefined && !schema.enum.some((entry) => sameJsonValue(value, entry))) {
    errors.push(`${at} must match one of the enumerated values`);
  }
  if (schema.oneOf !== undefined) {
    const branchResults = schema.oneOf.map((branch) => {
      const branchErrors = [];
      evaluateSchema(value, branch, rootSchema, at, branchErrors);
      return branchErrors;
    });
    const matches = branchResults.filter((branchErrors) => branchErrors.length === 0).length;
    if (matches !== 1) {
      errors.push(`${at} must match exactly one oneOf branch; matched ${matches}`);
      if (matches === 0 && branchResults.length > 0) {
        errors.push(...branchResults.flat());
      }
    }
  }
  if (schema.allOf !== undefined) {
    schema.allOf.forEach((branch) => evaluateSchema(value, branch, rootSchema, at, errors));
  }
  if (schema.if !== undefined) {
    const conditionErrors = [];
    evaluateSchema(value, schema.if, rootSchema, at, conditionErrors);
    const selected = conditionErrors.length === 0 ? schema.then : schema.else;
    if (selected !== undefined) evaluateSchema(value, selected, rootSchema, at, errors);
  }
  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) {
      errors.push(`${at} must contain at least ${schema.minLength} characters`);
    }
    if (schema.pattern !== undefined && !new RegExp(schema.pattern).test(value)) {
      errors.push(`${at} does not match pattern ${schema.pattern}`);
    }
    if (schema.format === "uri") {
      try {
        const parsed = new URL(value);
        if (!parsed.protocol) errors.push(`${at} is not an absolute URI`);
      } catch {
        errors.push(`${at} is not a valid URI`);
      }
    }
  }
  if (typeof value === "number" && schema.minimum !== undefined && value < schema.minimum) {
    errors.push(`${at} must be at least ${schema.minimum}`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) {
      errors.push(`${at} must contain at least ${schema.minItems} items`);
    }
    if (schema.maxItems !== undefined && value.length > schema.maxItems) {
      errors.push(`${at} must contain at most ${schema.maxItems} items`);
    }
    if (schema.items !== undefined) {
      value.forEach((entry, index) => evaluateSchema(entry, schema.items, rootSchema, `${at}[${index}]`, errors));
    }
  }
  if (isObject(value)) {
    if (schema.required !== undefined) {
      for (const key of schema.required) {
        if (!Object.hasOwn(value, key)) errors.push(`${at}.${key} is required`);
      }
    }
    if (schema.properties !== undefined) {
      for (const [key, childSchema] of Object.entries(schema.properties)) {
        if (Object.hasOwn(value, key)) evaluateSchema(value[key], childSchema, rootSchema, `${at}.${key}`, errors);
      }
      for (const key of Object.keys(value)) {
        if (Object.hasOwn(schema.properties, key)) continue;
        if (schema.additionalProperties === false) errors.push(`${at}.${key} is not allowed by the schema`);
        else if (isObject(schema.additionalProperties)) {
          evaluateSchema(value[key], schema.additionalProperties, rootSchema, `${at}.${key}`, errors);
        }
      }
    }
  }
}

export function validateJsonSchema(value, schema) {
  assertSupportedSchema(schema);
  const errors = [];
  evaluateSchema(value, schema, schema, "candidate", errors);
  return errors;
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
  if (candidate.status === "ready") {
    if (candidate.warnings?.some((warning) => warning.severity === "warning" || warning.severity === "error")) {
      errors.push("ready candidates cannot contain warning or error severity warnings");
    }
    if ((candidate.competingCandidates?.length ?? 0) > 0) {
      errors.push("ready candidates cannot contain competing candidates");
    }
    if (containsLowConfidence(candidate.fields)) {
      errors.push("ready candidates cannot contain low-confidence recovered values");
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
    const sourceUrl = fixture.input.sourceUrl;
    if (html === null && plainText === null) throw new Error(`Fixture ${fixture.id} has no input`);
    if (sourceUrl !== null) {
      const sourceUrlErrors = [];
      validateSourceUrl(sourceUrl, `Fixture ${fixture.id} input.sourceUrl`, sourceUrlErrors);
      if (sourceUrlErrors.length > 0) throw new Error(sourceUrlErrors.join("\n"));
    }
    if (html !== null && Buffer.byteLength(html) >= 20_000) throw new Error(`Fixture ${fixture.id} HTML is not compact`);
    if (plainText !== null && Buffer.byteLength(plainText) >= 10_000) throw new Error(`Fixture ${fixture.id} text is not compact`);
    const expected = JSON.parse(await fs.readFile(resolveInside(root, fixture.expected), "utf8"));
    const schemaErrors = validateJsonSchema(expected, schema);
    if (schemaErrors.length > 0) {
      throw new Error(`Fixture ${fixture.id} does not match the import candidate JSON Schema:\n- ${schemaErrors.join("\n- ")}`);
    }
    const errors = validateImportCandidate(expected);
    if (errors.length > 0) {
      throw new Error(`Fixture ${fixture.id} expected candidate is invalid:\n- ${errors.join("\n- ")}`);
    }
    fixtures.push({ fixture, input: { html, plainText, sourceUrl }, expected });
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
