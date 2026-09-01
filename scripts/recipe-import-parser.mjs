const INGREDIENT_HEADINGS = new Set(["ingredient", "ingredients"]);
const INSTRUCTION_HEADINGS = new Set([
  "direction",
  "directions",
  "instruction",
  "instructions",
  "method",
  "preparation",
]);
const UNIT_PATTERN = [
  "cups?",
  "tablespoons?",
  "tbsp",
  "teaspoons?",
  "tsp",
  "grams?",
  "g",
  "kilograms?",
  "kg",
  "millilit(?:er|re)s?",
  "ml",
  "lit(?:er|re)s?",
  "l",
  "ounces?",
  "oz",
  "pounds?",
  "lbs?",
  "pinches?",
  "cloves?",
  "cans?",
  "packages?",
  "slices?",
  "pieces?",
].join("|");
const AMOUNT_PATTERN = "(?:\\d+(?:[.,]\\d+)?(?:\\s+\\d+/\\d+)?|\\d+/\\d+|[\\u00bc-\\u00be\\u2150-\\u215e])";

function normalizeWhitespace(value) {
  return value.replace(/\s+/gu, " ").trim();
}

function decodeHtml(value) {
  const named = { amp: "&", apos: "'", gt: ">", lt: "<", nbsp: " ", quot: '"' };
  return value.replace(/&(#(?:x[0-9a-f]+|\d+)|[a-z]+);/giu, (entity, body) => {
    if (!body.startsWith("#")) return named[body.toLowerCase()] ?? entity;
    const hexadecimal = body[1]?.toLowerCase() === "x";
    const codePoint = Number.parseInt(body.slice(hexadecimal ? 2 : 1), hexadecimal ? 16 : 10);
    if (!Number.isInteger(codePoint) || codePoint < 0 || codePoint > 0x10ffff) return entity;
    try {
      return String.fromCodePoint(codePoint);
    } catch {
      return entity;
    }
  });
}

function htmlText(fragment) {
  return normalizeWhitespace(decodeHtml(fragment.replace(/<[^>]*>/gu, " ")));
}

function provenance(representation, locator) {
  return [{ representation, locator }];
}

function scalar(raw, representation, locator) {
  return { raw, value: raw, confidence: "high", provenance: provenance(representation, locator) };
}

function ingredient(raw, representation, locator) {
  const amountAndUnit = raw.match(
    new RegExp(
      `^(${AMOUNT_PATTERN})\\s+(${UNIT_PATTERN})[.,]?\\s+(.+)$`,
      "iu",
    ),
  );
  const amountOnly = amountAndUnit ? null : raw.match(new RegExp(`^(${AMOUNT_PATTERN})\\s+(.+)$`, "iu"));
  return {
    raw,
    amount: amountAndUnit?.[1] ?? amountOnly?.[1] ?? null,
    unit: amountAndUnit?.[2] ?? null,
    name: amountAndUnit?.[3] ?? amountOnly?.[2] ?? raw,
    confidence: "high",
    provenance: provenance(representation, locator),
  };
}

function instruction(raw, representation, locator) {
  return {
    raw,
    text: raw,
    confidence: "high",
    provenance: provenance(representation, locator),
    substeps: [],
  };
}

function sourceMetadata(sourceUrl) {
  const empty = {
    suppliedUrl: null,
    normalizedUrl: null,
    provider: null,
    author: null,
    attribution: null,
    provenance: { suppliedUrl: [], normalizedUrl: [], provider: [], author: [], attribution: [] },
  };
  if (typeof sourceUrl !== "string" || sourceUrl.trim() === "") return empty;

  const suppliedUrl = sourceUrl.trim();
  try {
    const parsed = new URL(suppliedUrl);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") return empty;
    parsed.hash = "";
    return {
      ...empty,
      suppliedUrl,
      normalizedUrl: parsed.href,
      provenance: {
        ...empty.provenance,
        suppliedUrl: provenance("source_url", "import source URL field"),
        normalizedUrl: provenance("source_url", "normalized from supplied URL"),
      },
    };
  } catch {
    return empty;
  }
}

function candidate(parser, sourceUrl) {
  return {
    schemaVersion: 1,
    status: "ready",
    parser,
    source: sourceMetadata(sourceUrl),
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
    unclassifiedLines: [],
    competingCandidates: [],
    warnings: [],
    discardedContent: [],
  };
}

function headingKind(value) {
  const heading = value.toLowerCase().replace(/:\s*$/u, "");
  if (INGREDIENT_HEADINGS.has(heading)) return "ingredients";
  if (INSTRUCTION_HEADINGS.has(heading)) return "instructions";
  return null;
}

function htmlHeadings(html) {
  const headings = [];
  const pattern = /<(h([1-6]))\b[^>]*>([\s\S]*?)<\/\1\s*>/giu;
  for (const match of html.matchAll(pattern)) {
    headings.push({
      kind: headingKind(htmlText(match[3])),
      text: htmlText(match[3]),
      tag: match[1].toLowerCase(),
      level: Number.parseInt(match[2], 10),
      index: match.index,
      end: match.index + match[0].length,
    });
  }
  return headings;
}

function listItems(sectionHtml) {
  const list = sectionHtml.match(/<(ul|ol)\b[^>]*>([\s\S]*?)<\/\1\s*>/iu);
  if (!list) return null;
  const items = [...list[2].matchAll(/<li\b[^>]*>([\s\S]*?)<\/li\s*>/giu)]
    .map((match) => htmlText(match[1]))
    .filter(Boolean);
  return items.length > 0 ? { tag: list[1].toLowerCase(), items } : null;
}

function rootRanges(html) {
  const ranges = [];
  const stack = [];
  const pattern = /<\/?(main|article)\b[^>]*>/giu;
  for (const match of html.matchAll(pattern)) {
    const tag = match[1].toLowerCase();
    if (!match[0].startsWith("</")) {
      stack.push({ tag, start: match.index, contentStart: match.index + match[0].length });
      continue;
    }
    const openingIndex = stack.findLastIndex((opening) => opening.tag === tag);
    if (openingIndex < 0) continue;
    const [opening] = stack.splice(openingIndex, 1);
    ranges.push({ ...opening, end: match.index + match[0].length, contentEnd: match.index });
  }
  return ranges;
}

function closestRoot(roots, index) {
  return roots
    .filter(({ start, end }) => start < index && index < end)
    .sort((left, right) => right.start - left.start)[0] ?? null;
}

function sameRoot(left, right) {
  if (left === null || right === null) return left === right;
  return left.start === right.start && left.end === right.end;
}

function recipeRegion(headings, roots) {
  for (const ingredientHeading of headings.filter(({ kind }) => kind === "ingredients")) {
    const root = closestRoot(roots, ingredientHeading.index);
    const instructionHeading = headings.find(
      ({ kind, index }) =>
        kind === "instructions" &&
        index > ingredientHeading.index &&
        sameRoot(closestRoot(roots, index), root),
    );
    if (instructionHeading) return { ingredientHeading, instructionHeading, root };
  }
  return null;
}

const VOID_TAGS = new Set([
  "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
  "source", "track", "wbr",
]);

function tagStackContext(html, index) {
  const stack = [];
  const pattern = /<\/?([a-z][a-z0-9:-]*)\b[^>]*>/giu;
  for (const match of html.matchAll(pattern)) {
    if ((match.index ?? 0) >= index) break;
    const tag = match[1].toLowerCase();
    if (match[0].startsWith("</")) {
      const openingIndex = stack.findLastIndex((opening) => opening.tag === tag);
      if (openingIndex >= 0) stack.splice(openingIndex);
      continue;
    }
    const parent = stack.at(-1);
    const ordinal = (parent?.childTagCounts.get(tag) ?? 0) + 1;
    if (parent) parent.childTagCounts.set(tag, ordinal);
    if (!VOID_TAGS.has(tag) && !match[0].endsWith("/>")) {
      stack.push({
        tag,
        start: match.index,
        ordinal,
        childTagCounts: new Map(),
      });
    }
  }
  return stack;
}

function elementLocator(html, root, element, { ordinal = false, omitDirectRoot = false } = {}) {
  const context = tagStackContext(html, element.index);
  const rootIndex = root
    ? context.findLastIndex(({ start }) => start === root.start)
    : -1;
  const parent = context.at(-1);
  const elementOrdinal = (parent?.childTagCounts.get(element.tag) ?? 0) + 1;
  const elementSelector = `${element.tag}${ordinal ? `:nth-of-type(${elementOrdinal})` : ""}`;
  const wrappers = rootIndex >= 0 ? context.slice(rootIndex + 1) : context;

  if (wrappers.length === 0) {
    if (omitDirectRoot || !root) return elementSelector;
    return `${root.tag} > ${elementSelector}`;
  }
  const wrapperSelector = wrappers
    .map(({ tag, ordinal: wrapperOrdinal }) => `${tag}:nth-of-type(${wrapperOrdinal})`)
    .join(" > ");
  return `${root ? `${root.tag} ` : ""}${wrapperSelector} > ${elementSelector}`;
}

function groupedLists(html, headings, roots, startHeading, endIndex, expectedListTag, root) {
  const componentHeadings = headings.filter(
    ({ level, index }) =>
      level === 3 &&
      index > startHeading.index &&
      index < endIndex &&
      sameRoot(closestRoot(roots, index), root),
  );
  if (componentHeadings.length === 0) return null;

  const sections = [];
  for (const componentHeading of componentHeadings) {
    const nextHeading = headings.find(({ index }) => index > componentHeading.index && index < endIndex);
    const componentHtml = html.slice(componentHeading.end, nextHeading?.index ?? endIndex);
    const list = listItems(componentHtml);
    if (!list || list.tag !== expectedListTag) continue;
    const headingLocator = elementLocator(html, root, componentHeading, {
      ordinal: true,
      omitDirectRoot: true,
    });
    const listLocator = `${headingLocator} + ${list.tag}`;
    sections.push({ name: componentHeading.text, list, listLocator });
  }
  return sections.length > 0 ? sections : null;
}

function parseHtml(html, sourceUrl) {
  // Remove active/raw-text regions before scanning. No markup is inserted into a DOM.
  const inertHtml = html.replace(
    /<(script|style|iframe|template|noscript)\b[^>]*>[\s\S]*?<\/\1\s*>/giu,
    "",
  );
  const headings = htmlHeadings(inertHtml);
  const roots = rootRanges(inertHtml);
  const region = recipeRegion(headings, roots);
  if (!region) return null;
  const { ingredientHeading, instructionHeading, root } = region;

  const nextHeadingIndex = (heading) =>
    headings.find((candidateHeading) => candidateHeading.index > heading.index)?.index ?? inertHtml.length;
  const followingSectionEnd = headings.find(
    ({ index, level }) => index > instructionHeading.index && level <= instructionHeading.level,
  )?.index ?? inertHtml.length;
  const instructionEnd = Math.min(followingSectionEnd, root?.contentEnd ?? inertHtml.length);
  const groupedIngredients = groupedLists(
    inertHtml,
    headings,
    roots,
    ingredientHeading,
    instructionHeading.index,
    "ul",
    root,
  );
  const groupedInstructions = groupedLists(
    inertHtml,
    headings,
    roots,
    instructionHeading,
    instructionEnd,
    "ol",
    root,
  );
  const ingredients = groupedIngredients ? null : listItems(
    inertHtml.slice(ingredientHeading.end, nextHeadingIndex(ingredientHeading)),
  );
  const instructions = groupedInstructions ? null : listItems(
    inertHtml.slice(instructionHeading.end, nextHeadingIndex(instructionHeading)),
  );
  if ((!groupedIngredients && !ingredients) || (!groupedInstructions && !instructions)) return null;

  const result = candidate(
    { id: "generic-headings", version: "1", tier: "generic_html", sourceAdapter: null },
    sourceUrl,
  );
  const beforeIngredients = inertHtml.slice(0, ingredientHeading.index);
  const titleMatch = [...beforeIngredients.matchAll(/<h1\b[^>]*>([\s\S]*?)<\/h1\s*>/giu)].at(-1);
  if (titleMatch) {
    const title = htmlText(titleMatch[1]);
    if (title) {
      const titleElement = { tag: "h1", index: titleMatch.index ?? 0 };
      result.fields.title = scalar(
        title,
        "text/html",
        elementLocator(inertHtml, root, titleElement),
      );
    }
  }
  const afterTitle = titleMatch
    ? beforeIngredients.slice((titleMatch.index ?? 0) + titleMatch[0].length)
    : beforeIngredients;
  const descriptionMatch = afterTitle.match(/<p\b[^>]*>([\s\S]*?)<\/p\s*>/iu);
  if (descriptionMatch) {
    const description = htmlText(descriptionMatch[1]);
    if (description) result.fields.description = scalar(description, "text/html", "article > p");
  }
  result.fields.ingredientSections = groupedIngredients
    ? groupedIngredients.map(({ name, list, listLocator }) => ({
      name,
      items: list.items.map((raw, index) =>
        ingredient(
          raw,
          "text/html",
          `${listLocator} > li${list.items.length > 1 ? `:nth-child(${index + 1})` : ""}`,
        )),
    }))
    : [{
      name: null,
      items: ingredients.items.map((raw, index) =>
        ingredient(raw, "text/html", `${ingredients.tag} > li:nth-child(${index + 1})`)),
    }];
  result.fields.instructionSections = groupedInstructions
    ? groupedInstructions.map(({ name, list, listLocator }) => ({
      name,
      steps: list.items.map((raw, index) =>
        instruction(
          raw,
          "text/html",
          `${listLocator} > li${list.items.length > 1 ? `:nth-child(${index + 1})` : ""}`,
        )),
    }))
    : [{
      name: null,
      steps: instructions.items.map((raw, index) =>
        instruction(raw, "text/html", `${instructions.tag} > li:nth-child(${index + 1})`)),
    }];
  if (groupedIngredients || groupedInstructions) {
    result.status = "needs_review";
    result.warnings.push(groupedIngredients
      ? {
        code: "SECTION_NAMES_NOT_PERSISTED",
        severity: "warning",
        path: "fields.ingredientSections",
        message: "Section names remain in preview but the current editor stores flat ingredients.",
      }
      : {
        code: "SECTION_NAMES_NOT_PERSISTED",
        severity: "warning",
        path: "fields.instructionSections",
        message: "Section names remain in preview but the current editor stores flat instructions.",
      });
  }
  return result;
}

function parsePlainText(plainText, sourceUrl) {
  const lines = plainText.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n")
    .map((raw, index) => ({ raw: raw.trim(), number: index + 1 }))
    .filter(({ raw }) => raw !== "");
  const firstHeading = lines.findIndex(({ raw }) => headingKind(raw));
  if (firstHeading < 1) return null;

  let section = null;
  const ingredients = [];
  const instructions = [];
  for (const line of lines.slice(firstHeading)) {
    const kind = headingKind(line.raw);
    if (kind) {
      section = kind;
      continue;
    }
    const prefix = section === "ingredients" ? /^[-*\u2022]\s+/u : /^(?:\d+[.)]|[-*\u2022])\s+/u;
    const raw = line.raw.replace(prefix, "");
    if (raw === line.raw) continue;
    (section === "ingredients" ? ingredients : instructions).push({ ...line, raw });
  }
  if (ingredients.length === 0 || instructions.length === 0) return null;

  const result = candidate(
    { id: "generic-text-headings", version: "1", tier: "generic_text", sourceAdapter: null },
    sourceUrl,
  );
  result.fields.title = scalar(lines[0].raw, "text/plain", `line ${lines[0].number}`);
  result.fields.ingredientSections = [{
    name: null,
    items: ingredients.map((line) => ingredient(line.raw, "text/plain", `line ${line.number}`)),
  }];
  result.fields.instructionSections = [{
    name: null,
    steps: instructions.map((line) => instruction(line.raw, "text/plain", `line ${line.number}`)),
  }];
  if (result.source.suppliedUrl === null) {
    result.warnings.push({
      code: "SOURCE_URL_MISSING",
      severity: "info",
      path: "source.suppliedUrl",
      message: "No source URL accompanied the pasted text.",
    });
  }
  return result;
}

/** Convert inert clipboard strings into a deterministic preview candidate. */
export function parseRecipeClipboard(input) {
  const html = typeof input?.html === "string" ? input.html.trim() : "";
  const plainText = typeof input?.plainText === "string" ? input.plainText : "";
  const sourceUrl = input?.sourceUrl ?? null;
  if (html) {
    const htmlCandidate = parseHtml(html, sourceUrl);
    if (htmlCandidate) return htmlCandidate;
  }
  if (plainText.trim()) {
    const textCandidate = parsePlainText(plainText, sourceUrl);
    if (textCandidate) return textCandidate;
  }
  throw new Error("Clipboard input does not contain a recognizable recipe.");
}
