const INGREDIENT_HEADINGS = new Set(["ingredient", "ingredients"]);
const INSTRUCTION_HEADINGS = new Set([
  "direction",
  "directions",
  "instruction",
  "instructions",
  "method",
  "preparation",
]);
const EQUIPMENT_HEADINGS = new Set(["equipment"]);
const NOTE_HEADINGS = new Set(["note", "notes"]);
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
const TASTY_AMOUNT_PATTERN = "(?:\\d+(?:[.,]\\d+)?(?:\\s+(?:\\d+/\\d+|[\\u00bc-\\u00be\\u2150-\\u215e]))?|\\d+/\\d+|[\\u00bc-\\u00be\\u2150-\\u215e])";

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

function ingredient(raw, representation, locator, amountPattern = AMOUNT_PATTERN) {
  const amountAndUnit = raw.match(
    new RegExp(
      `^(${amountPattern})\\s+(${UNIT_PATTERN})[.,]?\\s+(.+)$`,
      "iu",
    ),
  );
  const amountOnly = amountAndUnit
    ? null
    : raw.match(new RegExp(`^(${amountPattern})\\s+(.+)$`, "iu"));
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

function supplementaryHeadingKind(value) {
  const heading = value.toLowerCase().replace(/:\s*$/u, "");
  if (EQUIPMENT_HEADINGS.has(heading)) return "equipment";
  if (NOTE_HEADINGS.has(heading)) return "notes";
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

function listItems(sectionHtml, absoluteOffset = 0) {
  const list = sectionHtml.match(/<(ul|ol)\b([^>]*)>([\s\S]*?)<\/\1\s*>/iu);
  if (!list) return null;
  const items = [...list[3].matchAll(/<li\b[^>]*>([\s\S]*?)<\/li\s*>/giu)]
    .map((match) => htmlText(match[1]))
    .filter(Boolean);
  return items.length > 0
    ? {
      tag: list[1].toLowerCase(),
      attributes: htmlAttributes(list[2]),
      items,
      index: absoluteOffset + (list.index ?? 0),
    }
    : null;
}

function htmlAttributes(rawAttributes) {
  const attributes = new Map();
  const pattern = /([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/gu;
  for (const match of rawAttributes.matchAll(pattern)) {
    attributes.set(match[1].toLowerCase(), decodeHtml(match[2] ?? match[3] ?? match[4] ?? ""));
  }
  return attributes;
}

function listsWithinRoot(html, start, end, roots, root, expectedTag = null) {
  const sectionHtml = html.slice(start, end);
  const lists = [];
  const pattern = /<(ul|ol)\b([^>]*)>([\s\S]*?)<\/\1\s*>/giu;
  for (const match of sectionHtml.matchAll(pattern)) {
    const tag = match[1].toLowerCase();
    if (expectedTag && tag !== expectedTag) continue;
    const index = start + (match.index ?? 0);
    if (!sameRoot(closestRoot(roots, index), root)) continue;
    const items = [...match[3].matchAll(/<li\b[^>]*>([\s\S]*?)<\/li\s*>/giu)]
      .map((itemMatch) => htmlText(itemMatch[1]))
      .filter(Boolean);
    if (items.length === 0) continue;
    lists.push({ tag, attributes: htmlAttributes(match[2]), items, index });
  }
  return lists;
}

function listIsHidden(list) {
  if (list.attributes.has("hidden")) return true;
  if (list.attributes.get("aria-hidden")?.toLowerCase() === "true") return true;
  return /(?:^|;)\s*(?:display\s*:\s*none|visibility\s*:\s*hidden)\s*(?:!important\s*)?(?:;|$)/iu.test(
    list.attributes.get("style") ?? "",
  );
}

function ingredientListSelection(html, start, end, roots, root) {
  const lists = listsWithinRoot(html, start, end, roots, root, "ul");
  if (lists.length === 0) return null;

  const measurementLists = lists.filter((list) => list.attributes.has("data-unit"));
  if (measurementLists.length > 1) {
    const visibleMeasurementLists = measurementLists.filter((list) => !listIsHidden(list));
    const selected = visibleMeasurementLists.length > 0
      ? visibleMeasurementLists[0]
      : measurementLists[0];
    return {
      selected,
      competingMeasurementVariants: measurementLists
        .filter((list) => list !== selected)
        .map((list) => ({
          list,
          state: listIsHidden(list) ? "inactive" : "ambiguous",
        })),
    };
  }
  return { selected: lists[0], competingMeasurementVariants: [] };
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

function recipeRegions(headings, roots) {
  const regions = [];
  const ingredientHeadings = headings.filter(({ kind }) => kind === "ingredients");
  for (const ingredientHeading of ingredientHeadings) {
    const root = closestRoot(roots, ingredientHeading.index);
    const nextIngredientIndex = ingredientHeadings.find(
      ({ index }) => index > ingredientHeading.index,
    )?.index ?? Number.POSITIVE_INFINITY;
    const instructionHeading = headings.find(
      ({ kind, index }) =>
        kind === "instructions" &&
        index > ingredientHeading.index &&
        index < nextIngredientIndex &&
        sameRoot(closestRoot(roots, index), root),
    );
    if (instructionHeading) regions.push({ ingredientHeading, instructionHeading, root });
  }
  return regions;
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

function elementNeedsOrdinal(html, root, element) {
  const parentContext = tagStackContext(html, element.index);
  const start = root?.contentStart ?? 0;
  const end = root?.contentEnd ?? html.length;
  let count = 0;
  const pattern = new RegExp(`<${element.tag}\\b`, "giu");
  for (const match of html.slice(start, end).matchAll(pattern)) {
    const index = start + (match.index ?? 0);
    if (!sameRoot(closestRoot(root ? [root] : [], index), root)) continue;
    if (!sameTagStack(tagStackContext(html, index), parentContext)) continue;
    count += 1;
    if (count > 1) return true;
  }
  return false;
}

function rootedElementLocator(html, root, element) {
  return elementLocator(html, root, element, {
    ordinal: elementNeedsOrdinal(html, root, element),
  });
}

function sameTagStack(left, right) {
  return left.length === right.length && left.every(({ start }, index) => start === right[index].start);
}

function headingsMatchExactly(left, right) {
  return left.text.toLocaleLowerCase("en-US") === right.text.toLocaleLowerCase("en-US");
}

function adjacentListAfterHeading(html, heading, list) {
  const sameParent = sameTagStack(
    tagStackContext(html, heading.index),
    tagStackContext(html, list.index),
  );
  if (!sameParent) return false;
  const interveningHtml = html.slice(heading.end, list.index).replace(/<!--[\s\S]*?-->/gu, "");
  return interveningHtml.trim() === "";
}

function listItemsWithinRoot(html, start, end, roots, root) {
  const list = listItems(html.slice(start, end), start);
  if (!list || !sameRoot(closestRoot(roots, list.index), root)) return null;
  return list;
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
    const componentEnd = nextHeading?.index ?? endIndex;
    const list = listItemsWithinRoot(
      html,
      componentHeading.end,
      componentEnd,
      roots,
      root,
    );
    if (
      !list ||
      list.tag !== expectedListTag ||
      !adjacentListAfterHeading(html, componentHeading, list)
    ) continue;
    const headingLocator = elementLocator(html, root, componentHeading, {
      ordinal: true,
      omitDirectRoot: true,
    });
    const listLocator = `${headingLocator} + ${list.tag}`;
    sections.push({ name: componentHeading.text, list, listLocator });
  }
  return sections.length > 0 ? sections : null;
}

function regionTitle(headings, roots, region) {
  const { ingredientHeading, root } = region;
  const contentStart = root?.contentStart ?? 0;
  const precedingHeadings = headings.filter(
    ({ index, kind }) =>
      kind === null &&
      index >= contentStart &&
      index < ingredientHeading.index &&
      sameRoot(closestRoot(roots, index), root),
  );
  const pageTitle = precedingHeadings.filter(({ tag }) => tag === "h1").at(-1) ?? null;

  // Recipe cards commonly put an h2 title above h3 metadata sections whose
  // wrappers separate the title from Ingredients. Same-level metadata headings
  // are not sufficient evidence to displace the page h1.
  const embeddedTitle = precedingHeadings.filter(
    ({ tag, level }) => tag !== "h1" && level < ingredientHeading.level,
  ).at(-1) ?? null;
  return {
    title: embeddedTitle ?? pageTitle,
    collectionTitle: embeddedTitle && pageTitle && pageTitle.index < embeddedTitle.index &&
      !headingsMatchExactly(embeddedTitle, pageTitle)
      ? pageTitle
      : null,
  };
}

function extractRegionLists(html, headings, roots, region) {
  const { ingredientHeading, instructionHeading, root } = region;
  const nextHeadingIndex = (heading) =>
    headings.find((candidateHeading) => candidateHeading.index > heading.index)?.index ?? html.length;
  const followingSectionEnd = headings.find(
    ({ index, level }) => index > instructionHeading.index && level <= instructionHeading.level,
  )?.index ?? html.length;
  const rootContentEnd = root?.contentEnd ?? html.length;
  const instructionEnd = Math.min(followingSectionEnd, rootContentEnd);
  const groupedIngredients = groupedLists(
    html,
    headings,
    roots,
    ingredientHeading,
    instructionHeading.index,
    "ul",
    root,
  );
  const groupedInstructions = groupedLists(
    html,
    headings,
    roots,
    instructionHeading,
    instructionEnd,
    "ol",
    root,
  );
  const ingredientSelection = groupedIngredients ? null : ingredientListSelection(
    html,
    ingredientHeading.end,
    Math.min(nextHeadingIndex(ingredientHeading), rootContentEnd),
    roots,
    root,
  );
  const ingredients = ingredientSelection?.selected ?? null;
  const instructions = groupedInstructions ? null : listItemsWithinRoot(
    html,
    instructionHeading.end,
    Math.min(nextHeadingIndex(instructionHeading), rootContentEnd),
    roots,
    root,
  );
  if ((!groupedIngredients && !ingredients) || (!groupedInstructions && !instructions)) return null;
  return {
    groupedIngredients,
    groupedInstructions,
    ingredients,
    instructions,
    competingMeasurementVariants: ingredientSelection?.competingMeasurementVariants ?? [],
  };
}

function listItemCount({ groupedIngredients, groupedInstructions, ingredients, instructions }) {
  const ingredientCount = groupedIngredients
    ? groupedIngredients.reduce((count, section) => count + section.list.items.length, 0)
    : ingredients.items.length;
  const instructionCount = groupedInstructions
    ? groupedInstructions.reduce((count, section) => count + section.list.items.length, 0)
    : instructions.items.length;
  return ingredientCount + instructionCount;
}

function hasAncestorTag(html, index, tag) {
  return tagStackContext(html, index).some((element) => element.tag === tag);
}

function scoreRegion(region) {
  let score = region.title ? 100 : 0;
  score += Math.min(listItemCount(region.lists), 20);
  if (region.root?.tag === "article") score += 20;
  if (region.title && region.title.tag !== "h1") score += 10;
  if (region.navigationIngredientList) score -= 1_000;
  return score;
}

function editorialIngredientRegions(html, headings, roots, selectedRegion) {
  return headings.flatMap((heading) => {
    if (
      heading.index >= selectedRegion.ingredientHeading.index ||
      !/^ingredients?\s+for\b/iu.test(heading.text)
    ) return [];
    const root = closestRoot(roots, heading.index);
    const nextHeadingIndex = headings.find(({ index }) => index > heading.index)?.index ?? html.length;
    const list = listItemsWithinRoot(html, heading.end, nextHeadingIndex, roots, root);
    if (!list || !adjacentListAfterHeading(html, heading, list)) return [];

    const context = tagStackContext(html, heading.index);
    const rootIndex = root
      ? context.findLastIndex(({ start }) => start === root.start)
      : -1;
    const wrappers = rootIndex >= 0 ? context.slice(rootIndex + 1) : context;
    const locator = root && wrappers.length > 0
      ? `${root.tag} > ${wrappers.map(({ tag }) => tag).join(" > ")}`
      : elementLocator(html, root, heading);
    return [{
      reason: "Earlier ingredient-like editorial region",
      title: heading.text,
      provenance: provenance("text/html", locator),
    }];
  });
}

function listLocator(list) {
  const dataUnit = list.attributes.get("data-unit");
  if (dataUnit && /^[a-z0-9_-]+$/iu.test(dataUnit)) {
    return `${list.tag}[data-unit=${dataUnit}]`;
  }
  return list.tag;
}

function measurementVariantReason(list, state) {
  const unitSystem = normalizeWhitespace(list.attributes.get("data-unit") ?? "").toLowerCase();
  const stateLabel = state === "ambiguous" ? "Ambiguous" : "Inactive";
  return unitSystem
    ? `${stateLabel} ${unitSystem} measurement variant`
    : `${stateLabel} measurement variant`;
}

function durationMinutes(value) {
  const tokens = [];
  const pattern = /(\d+(?:[.,]\d+)?)\s*(days?|d|hours?|hrs?|hr|h|minutes?|mins?|min|m)\b/giu;
  for (const match of value.matchAll(pattern)) {
    const amount = Number.parseFloat(match[1].replace(",", "."));
    if (!Number.isFinite(amount)) continue;
    const unit = match[2].toLowerCase();
    const canonicalUnit = unit.startsWith("d")
      ? "day"
      : unit.startsWith("h")
        ? "hour"
        : "minute";
    tokens.push({ amount, canonicalUnit });
  }
  if (tokens.length === 0) return null;

  // Clipboard HTML can expose an accessible duration immediately after its
  // visible equivalent. Collapse only a complete repeated token sequence so
  // legitimate repeated units in a longer duration remain additive.
  let normalizedTokens = tokens;
  for (let sequenceLength = 1; sequenceLength <= Math.floor(tokens.length / 2); sequenceLength += 1) {
    if (tokens.length % sequenceLength !== 0) continue;
    const repeatsExactly = tokens.every((token, index) => {
      const expected = tokens[index % sequenceLength];
      return token.amount === expected.amount && token.canonicalUnit === expected.canonicalUnit;
    });
    if (repeatsExactly) {
      normalizedTokens = tokens.slice(0, sequenceLength);
      break;
    }
  }

  let minutes = 0;
  for (const { amount, canonicalUnit } of normalizedTokens) {
    if (canonicalUnit === "day") minutes += amount * 24 * 60;
    else if (canonicalUnit === "hour") minutes += amount * 60;
    else minutes += amount;
  }
  return Number.isSafeInteger(minutes) && minutes >= 0 ? minutes : null;
}

function labeledDuration(raw) {
  const match = raw.match(
    /^(prep(?:aration)?(?:\s+time)?|active(?:\s+time)?|cook(?:ing)?(?:\s+time)?|total(?:\s+time)?)\s*:?\s*(.+)$/iu,
  );
  if (!match) return null;
  const value = durationMinutes(match[2]);
  if (value === null) return null;
  const label = match[1].toLowerCase();
  const field = label.startsWith("prep")
    ? "prepTime"
    : label.startsWith("total")
      ? "totalTime"
      : "cookTime";
  return { field, value };
}

function regionDurations(html, start, end, root) {
  const result = new Map();
  const sectionHtml = html.slice(start, end);
  const pattern = /<(div|p|span|li|time)\b[^>]*>([\s\S]*?)<\/\1\s*>/giu;
  for (const match of sectionHtml.matchAll(pattern)) {
    const raw = htmlText(match[2]);
    const parsed = labeledDuration(raw);
    if (!parsed || result.has(parsed.field)) continue;
    const element = { tag: match[1].toLowerCase(), index: start + (match.index ?? 0) };
    result.set(parsed.field, {
      raw,
      value: parsed.value,
      confidence: "high",
      provenance: provenance(
        "text/html",
        elementLocator(html, root, element, { ordinal: true }),
      ),
    });
  }
  return result;
}

function selectedYield(html, start, end) {
  const sectionHtml = html.slice(start, end);
  const labels = [...sectionHtml.matchAll(/<label\b([^>]*)>([\s\S]*?)<\/label\s*>/giu)]
    .map((label) => ({
      attributes: htmlAttributes(label[1]),
      text: htmlText(label[2].replace(/<select\b[^>]*>[\s\S]*?<\/select\s*>/giu, " ")),
      start: label.index ?? 0,
      end: (label.index ?? 0) + label[0].length,
    }));
  for (const select of sectionHtml.matchAll(/<select\b([^>]*)>([\s\S]*?)<\/select\s*>/giu)) {
    const selectAttributes = htmlAttributes(select[1]);
    const selectIndex = select.index ?? 0;
    const selectId = selectAttributes.get("id") ?? "";
    const associatedLabelText = labels
      .filter((label) =>
        (label.start < selectIndex && selectIndex < label.end) ||
        (selectId !== "" && label.attributes.get("for") === selectId))
      .map((label) => label.text);
    const describedAsYield = [
      ...associatedLabelText,
      selectAttributes.get("aria-label") ?? "",
      selectAttributes.get("name") ?? "",
      selectId,
    ].some((value) => /(?:yield|servings?)/iu.test(value));
    if (!describedAsYield) continue;

    const selectedOption = [...select[2].matchAll(/<option\b([^>]*)>([\s\S]*?)<\/option\s*>/giu)]
      .find((option) => htmlAttributes(option[1]).has("selected"));
    if (!selectedOption) continue;
    const raw = htmlText(selectedOption[2]);
    if (!raw) continue;
    const servingsMatch = raw.match(/^(\d+(?:[.,]\d+)?)(?:\s|$)/u);
    const servings = servingsMatch
      ? Number.parseFloat(servingsMatch[1].replace(",", "."))
      : null;
    return {
      raw,
      value: {
        display: raw,
        servings: Number.isSafeInteger(servings) && servings > 0 ? servings : null,
      },
      confidence: "high",
      provenance: provenance("text/html", "select > option[selected]"),
    };
  }
  return null;
}

const DISCARDED_RAW_CONTENT_TAGS = new Set(["script", "style", "iframe", "template", "noscript"]);
const TEXT_CONTENT_TAGS = new Set(["script", "style", "textarea", "title", "iframe", "noscript"]);
const STRUCTURED_OPAQUE_TAGS = new Set(["template"]);
const REMOTE_RESOURCE_ATTRIBUTES = new Set(["action", "href", "poster", "src", "srcset"]);

function htmlTagEnd(html, start) {
  let quote = null;
  for (let index = start; index < html.length; index += 1) {
    const character = html[index];
    if (quote !== null) {
      if (character === quote) quote = null;
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
    } else if (character === ">") {
      return index + 1;
    }
  }
  return null;
}

function textContentEnd(html, start, tag) {
  const closingPattern = new RegExp(`<\\/${tag}\\s*>`, "giu");
  closingPattern.lastIndex = start;
  const closingMatch = closingPattern.exec(html);
  return closingMatch
    ? (closingMatch.index ?? 0) + closingMatch[0].length
    : html.length;
}

function rawContentEnd(html, start, tag) {
  if (TEXT_CONTENT_TAGS.has(tag)) return textContentEnd(html, start, tag);

  const openContainers = [tag];
  let scanIndex = start;
  while (scanIndex < html.length) {
    const tagIndex = html.indexOf("<", scanIndex);
    if (tagIndex < 0) return html.length;
    if (html.startsWith("<!--", tagIndex)) {
      const commentEnd = html.indexOf("-->", tagIndex + 4);
      if (commentEnd < 0) return html.length;
      scanIndex = commentEnd + 3;
      continue;
    }
    if (html.startsWith("<!", tagIndex) || html.startsWith("<?", tagIndex)) {
      const declarationEnd = html.indexOf(">", tagIndex + 2);
      if (declarationEnd < 0) return html.length;
      scanIndex = declarationEnd + 1;
      continue;
    }

    const closingMatch = html.slice(tagIndex).match(/^<\/\s*([a-z][a-z0-9:-]*)\s*>/iu);
    if (closingMatch) {
      if (closingMatch[1].toLowerCase() === openContainers.at(-1)) {
        openContainers.pop();
        if (openContainers.length === 0) return tagIndex + closingMatch[0].length;
      }
      scanIndex = tagIndex + closingMatch[0].length;
      continue;
    }

    const openingMatch = html.slice(tagIndex + 1).match(/^([a-z][a-z0-9:-]*)\b/iu);
    if (!openingMatch) {
      scanIndex = tagIndex + 1;
      continue;
    }
    const openingEnd = htmlTagEnd(html, tagIndex + 1 + openingMatch[0].length);
    if (openingEnd === null) return html.length;
    const openingTag = openingMatch[1].toLowerCase();
    const selfClosing = /\/\s*>$/u.test(html.slice(tagIndex, openingEnd));
    if (!selfClosing && TEXT_CONTENT_TAGS.has(openingTag)) {
      scanIndex = textContentEnd(html, openingEnd, openingTag);
      continue;
    }
    if (!selfClosing && STRUCTURED_OPAQUE_TAGS.has(openingTag)) {
      openContainers.push(openingTag);
    }
    scanIndex = openingEnd;
  }
  return html.length;
}

function discardRawContent(html, preserveOpeningTags = false) {
  const output = [];
  let retainedStart = 0;
  let scanIndex = 0;

  while (scanIndex < html.length) {
    const openingIndex = html.indexOf("<", scanIndex);
    if (openingIndex < 0) break;
    if (html.startsWith("<!--", openingIndex)) {
      const commentEnd = html.indexOf("-->", openingIndex + 4);
      output.push(html.slice(retainedStart, openingIndex));
      retainedStart = commentEnd < 0 ? html.length : commentEnd + 3;
      scanIndex = retainedStart;
      continue;
    }

    const tagMatch = html.slice(openingIndex + 1).match(/^([a-z][a-z0-9:-]*)\b/iu);
    if (!tagMatch) {
      scanIndex = openingIndex + 1;
      continue;
    }
    const tag = tagMatch[1].toLowerCase();
    const tagNameEnd = openingIndex + 1 + tagMatch[0].length;
    const openingEnd = htmlTagEnd(html, tagNameEnd);
    if (!DISCARDED_RAW_CONTENT_TAGS.has(tag)) {
      scanIndex = openingEnd ?? html.length;
      continue;
    }

    output.push(html.slice(retainedStart, openingIndex));
    if (openingEnd === null) {
      if (preserveOpeningTags) output.push(`<${tag}>`);
      retainedStart = html.length;
      scanIndex = html.length;
      break;
    }

    if (preserveOpeningTags) output.push(html.slice(openingIndex, openingEnd));
    const discardedEnd = rawContentEnd(html, openingEnd, tag);
    retainedStart = discardedEnd;
    scanIndex = discardedEnd;
  }

  output.push(html.slice(retainedStart));
  return output.join("");
}

function openingHtmlElements(html) {
  // Raw element bodies are intentionally opaque so markup-looking script text is
  // not mistaken for page controls or resources during the safety audit.
  const auditableHtml = discardRawContent(html, true);
  const elements = [];
  const pattern = /<([a-z][a-z0-9:-]*)\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu;
  for (const match of auditableHtml.matchAll(pattern)) {
    elements.push({
      tag: match[1].toLowerCase(),
      attributes: htmlAttributes(match[2]),
    });
  }
  return elements;
}

function isRemoteHttpResource(value) {
  if (typeof value !== "string") return false;
  return /(?:^|[\s,])https?:\/\/[^\s,]+/iu.test(value.trim());
}

function clipboardHtmlAudit(html) {
  const elements = openingHtmlElements(html);
  const remoteResources = [];
  for (const element of elements) {
    for (const [attribute, value] of element.attributes) {
      if (REMOTE_RESOURCE_ATTRIBUTES.has(attribute) && isRemoteHttpResource(value)) {
        remoteResources.push({ tag: element.tag, attribute, value: normalizeWhitespace(value) });
      }
    }
  }
  return {
    scriptCount: elements.filter(({ tag }) => tag === "script").length,
    iframeCount: elements.filter(({ tag }) => tag === "iframe").length,
    formControlCount: elements.filter(({ tag }) =>
      tag === "button" || tag === "input" || tag === "select" || tag === "textarea").length,
    remoteResources,
  };
}

function discardedRemoteResourceCount(audit, retainedImages) {
  const retainedImageSources = new Map();
  for (const image of retainedImages) {
    retainedImageSources.set(image.sourceUrl, (retainedImageSources.get(image.sourceUrl) ?? 0) + 1);
  }

  let count = 0;
  for (const resource of audit.remoteResources) {
    const retainedCount = resource.tag === "img" && resource.attribute === "src"
      ? retainedImageSources.get(resource.value) ?? 0
      : 0;
    if (retainedCount > 0) {
      retainedImageSources.set(resource.value, retainedCount - 1);
    } else {
      count += 1;
    }
  }
  return count;
}

function appendHtmlAuditEvidence(result, audit, remoteResourceCount) {
  const activeOrRemoteContentDiscarded =
    audit.scriptCount > 0 || audit.iframeCount > 0 || remoteResourceCount > 0;
  if (audit.scriptCount > 0) {
    result.discardedContent.push({
      kind: "script",
      count: audit.scriptCount,
      reason: "Executable content is never parsed or rendered.",
    });
  }
  if (audit.iframeCount > 0) {
    result.discardedContent.push({
      kind: "iframe",
      count: audit.iframeCount,
      reason: "Frames are never loaded.",
    });
  }
  if (audit.formControlCount > 0) {
    result.discardedContent.push({
      kind: "form_control",
      count: audit.formControlCount,
      reason: activeOrRemoteContentDiscarded
        ? "Interactive controls are not recipe content."
        : "Controls are evidence of active state, not recipe content.",
    });
  }
  if (remoteResourceCount > 0) {
    result.discardedContent.push({
      kind: "remote_resource",
      count: remoteResourceCount,
      reason: "Parsing performs no network requests.",
    });
  }
  if (activeOrRemoteContentDiscarded) {
    result.warnings.push({
      code: "ACTIVE_CONTENT_DISCARDED",
      severity: "info",
      path: "discardedContent",
      message: "Active and remote page elements were removed before parsing.",
    });
  }
}

function supplementarySections(html, headings, roots, root) {
  const rootEnd = root?.contentEnd ?? html.length;
  const sectionHeadings = headings.filter(
    (heading) =>
      supplementaryHeadingKind(heading.text) !== null &&
      sameRoot(closestRoot(roots, heading.index), root),
  );
  const equipment = [];
  const notes = [];

  for (const heading of sectionHeadings) {
    const kind = supplementaryHeadingKind(heading.text);
    const sectionEnd = headings.find(
      (candidateHeading) =>
        candidateHeading.index > heading.index &&
        candidateHeading.level <= heading.level &&
        sameRoot(closestRoot(roots, candidateHeading.index), root),
    )?.index ?? rootEnd;

    if (kind === "equipment") {
      for (const list of listsWithinRoot(html, heading.end, sectionEnd, roots, root)) {
        const locator = rootedElementLocator(html, root, list);
        equipment.push(...list.items.map((raw, index) =>
          scalar(
            raw,
            "text/html",
            `${locator} > li${list.items.length > 1 ? `:nth-child(${index + 1})` : ""}`,
          )));
      }
      continue;
    }

    const sectionHtml = html.slice(heading.end, sectionEnd);
    for (const match of sectionHtml.matchAll(/<p\b[^>]*>([\s\S]*?)<\/p\s*>/giu)) {
      const raw = htmlText(match[1]);
      if (!raw) continue;
      const paragraph = { tag: "p", index: heading.end + (match.index ?? 0) };
      if (!sameRoot(closestRoot(roots, paragraph.index), root)) continue;
      const adjacent = sameTagStack(
        tagStackContext(html, heading.index),
        tagStackContext(html, paragraph.index),
      ) && html.slice(heading.end, paragraph.index).replace(/<!--[\s\S]*?-->/gu, "").trim() === "";
      const locator = adjacent
        ? `${elementLocator(html, root, heading, { ordinal: true, omitDirectRoot: true })} + p`
        : rootedElementLocator(html, root, paragraph);
      notes.push(scalar(raw, "text/html", locator));
    }
  }
  return { equipment, notes };
}

function imageReferences(html, roots, root, fallbackSourceAttribute = null) {
  const start = root?.contentStart ?? 0;
  const end = root?.contentEnd ?? html.length;
  const images = [];
  let attribution = null;
  for (const match of html.slice(start, end).matchAll(/<figure\b[^>]*>([\s\S]*?)<\/figure\s*>/giu)) {
    const figure = { tag: "figure", index: start + (match.index ?? 0) };
    if (!sameRoot(closestRoot(roots, figure.index), root)) continue;
    const imageMatch = match[1].match(/<img\b([^>]*)>/iu);
    if (!imageMatch) continue;
    const attributes = htmlAttributes(imageMatch[1]);
    // Lazy-source attributes are opt-in for a verified provider shape. They are
    // references only; no image is ever fetched by this extractor.
    const sourceUrl = normalizeWhitespace(attributes.get("src") ??
      (fallbackSourceAttribute ? attributes.get(fallbackSourceAttribute) : null) ?? "");
    try {
      const parsed = new URL(sourceUrl);
      if (parsed.protocol !== "http:" && parsed.protocol !== "https:") continue;
    } catch {
      continue;
    }
    const captionMatch = match[1].match(/<figcaption\b[^>]*>([\s\S]*?)<\/figcaption\s*>/iu);
    const credit = captionMatch ? htmlText(captionMatch[1]) || null : null;
    const figureNeedsOrdinal = elementNeedsOrdinal(html, root, figure);
    if (credit !== null && attribution === null) {
      attribution = {
        value: credit,
        provenance: provenance(
          "text/html",
          `${elementLocator(html, root, figure, {
            ordinal: figureNeedsOrdinal,
            omitDirectRoot: true,
          })} > figcaption`,
        ),
      };
    }
    images.push({
      sourceUrl,
      alt: normalizeWhitespace(attributes.get("alt") ?? "") || null,
      credit,
      provenance: provenance(
        "text/html",
        elementLocator(html, root, figure, { ordinal: figureNeedsOrdinal }),
      ),
    });
  }
  return { images, attribution };
}

function unsupportedNavigationCandidate(html, sourceUrl, root, ingredients) {
  const result = candidate(
    { id: "generic-region-scorer", version: "1", tier: "generic_html", sourceAdapter: null },
    sourceUrl,
  );
  result.status = "unsupported";
  const listLocator = elementLocator(html, root, ingredients, { ordinal: true });
  result.unclassifiedLines = ingredients.items.map((raw, index) => ({
    raw,
    provenance: provenance("text/html", `${listLocator} > li:nth-child(${index + 1})`),
  }));
  result.warnings.push({
    code: "NAVIGATION_INGREDIENT_LIST",
    severity: "error",
    path: "fields.ingredientSections",
    message: "The ingredient-like list is inside navigation and is not safe to import as recipe content.",
  });
  result.discardedContent.push({
    kind: "navigation",
    count: 1,
    reason: "Navigation lists are not recipe ingredients.",
  });
  return result;
}

function unsupportedTitlelessCandidate(sourceUrl, root, lists) {
  const result = candidate(
    { id: "generic-region-scorer", version: "1", tier: "generic_html", sourceAdapter: null },
    sourceUrl,
  );
  result.status = "unsupported";
  const rootPrefix = root ? `${root.tag} ` : "";
  const ingredientLists = lists.groupedIngredients
    ? lists.groupedIngredients.map((section) => section.list)
    : [lists.ingredients];
  const instructionLists = lists.groupedInstructions
    ? lists.groupedInstructions.map((section) => section.list)
    : [lists.instructions];
  result.unclassifiedLines = [
    ...ingredientLists.flatMap((list) => list.items.map((raw, index) => ({
      raw,
      provenance: provenance("text/html", `${rootPrefix}${list.tag} > li:nth-child(${index + 1})`),
    }))),
    ...instructionLists.flatMap((list) => list.items.map((raw, index) => ({
      raw,
      provenance: provenance("text/html", `${rootPrefix}${list.tag} > li:nth-child(${index + 1})`),
    }))),
  ];
  result.warnings.push({
    code: "MISSING_RECIPE_TITLE",
    severity: "error",
    path: "fields.title",
    message: "An ingredient and instruction region was found without a coherent recipe title.",
  });
  return result;
}

function hasClass(attributes, className) {
  return (attributes.get("class") ?? "").split(/\s+/u).includes(className);
}

function matchingElementEnd(html, tag, openingEnd) {
  const pattern = new RegExp(`<\\/?${tag}\\b(?:"[^"]*"|'[^']*'|[^>"'])*>`, "giu");
  pattern.lastIndex = openingEnd;
  let depth = 1;
  for (const match of html.matchAll(pattern)) {
    if (match[0].startsWith("</")) {
      depth -= 1;
      if (depth === 0) return { contentEnd: match.index, end: match.index + match[0].length };
    } else if (!match[0].endsWith("/>")) {
      depth += 1;
    }
  }
  return null;
}

function elementsWithClass(html, className) {
  return elementsMatchingAttributes(html, (attributes) => hasClass(attributes, className));
}

// Class and provider data-field extraction share the same non-executing scanner.
function elementsMatchingAttributes(html, matchesAttributes) {
  const elements = [];
  const pattern = /<([a-z][a-z0-9:-]*)\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu;
  for (const match of html.matchAll(pattern)) {
    const attributes = htmlAttributes(match[2]);
    if (!matchesAttributes(attributes, match[1].toLowerCase())) continue;
    const openingEnd = (match.index ?? 0) + match[0].length;
    const range = VOID_TAGS.has(match[1].toLowerCase())
      ? { contentEnd: openingEnd, end: openingEnd }
      : matchingElementEnd(html, match[1], openingEnd);
    if (!range) continue;
    elements.push({
      tag: match[1].toLowerCase(),
      attributes,
      index: match.index ?? 0,
      openingEnd,
      ...range,
      innerHtml: html.slice(openingEnd, range.contentEnd),
    });
  }
  return elements;
}

function firstClassText(html, className) {
  const element = elementsWithClass(html, className)[0];
  if (!element) return null;
  const text = htmlText(element.innerHtml);
  return text ? { text, element } : null;
}

function tastyHostname(sourceUrl) {
  if (typeof sourceUrl !== "string") return null;
  try {
    const parsed = new URL(sourceUrl);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") return null;
    const hostname = parsed.hostname.toLowerCase();
    return hostname === "tasty.co" || hostname.endsWith(".tasty.co") ? hostname : null;
  } catch {
    return null;
  }
}

function tastyIngredient(raw, representation, locator) {
  const compoundAmount = raw.match(new RegExp(
    `^((${TASTY_AMOUNT_PATTERN})\\s+(${UNIT_PATTERN})[.,]?\\s+plus\\s+(${TASTY_AMOUNT_PATTERN})\\s+(${UNIT_PATTERN}))\\s+(.+)$`,
    "iu",
  ));
  if (!compoundAmount) return ingredient(raw, representation, locator, TASTY_AMOUNT_PATTERN);

  // A compound measurement has two units, so preserve it as one amount instead
  // of assigning either unit to the recipe editor's singular unit field.
  return {
    raw,
    amount: compoundAmount[1],
    unit: null,
    name: compoundAmount[6],
    confidence: "high",
    provenance: provenance(representation, locator),
  };
}

function tastyIngredientSections(containerHtml) {
  const sections = elementsWithClass(containerHtml, "ingredients__section");
  if (sections.length === 0) return null;
  const extractedSections = [];
  for (const [sectionIndex, section] of sections.entries()) {
    const itemElements = elementsWithClass(section.innerHtml, "ingredient");
    if (itemElements.length === 0) return null;
    const sectionName = firstClassText(section.innerHtml, "ingredient-section-name")?.text ?? null;
    const items = [];
    for (const [itemIndex, item] of itemElements.entries()) {
      const raw = htmlText(item.innerHtml);
      if (!raw) return null;
      items.push(tastyIngredient(
        raw,
        "text/html",
        `.ingredients__section:nth-child(${sectionIndex + 1}) .ingredient:nth-child(${itemIndex + 1})`,
      ));
    }
    extractedSections.push({
      name: sectionName,
      items,
    });
  }
  return extractedSections;
}

function tastyDownloadPromotion(stepHtml) {
  for (const anchor of stepHtml.matchAll(/<a\b([^>]*)>/giu)) {
    const href = htmlAttributes(anchor[1]).get("href");
    if (!href) continue;
    try {
      const url = new URL(href, "https://tasty.co");
      if (
        tastyHostname(url.href) &&
        url.pathname === "/download" &&
        url.searchParams.get("ct") === "prepsteps"
      ) return true;
    } catch {
      // A malformed link is ordinary visible step content, not a provider CTA.
    }
  }
  return false;
}

function tastyInstructionSections(preparationHtml) {
  const stepLists = elementsWithClass(preparationHtml, "prep-steps");
  if (stepLists.length !== 1) return null;
  const steps = [];
  let discardedPromotionCount = 0;
  for (const [itemIndex, item] of [...stepLists[0].innerHtml.matchAll(
    /<li\b[^>]*>([\s\S]*?)<\/li\s*>/giu,
  )].entries()) {
    if (tastyDownloadPromotion(item[1])) {
      discardedPromotionCount += 1;
      continue;
    }
    const text = htmlText(item[1]).replace(/\s+([,.;:!?])/gu, "$1");
    if (!text) return null;
    steps.push(instruction(
      text,
      "text/html",
      `.prep-steps > li:nth-child(${itemIndex + 1})`,
    ));
  }
  return steps.length > 0
    ? { sections: [{ name: null, steps }], discardedPromotionCount }
    : null;
}

function tastyDurations(html) {
  const durations = new Map();
  for (const [index, time] of elementsWithClass(html, "recipe-time").entries()) {
    const headingMatch = time.innerHtml.match(/<h[1-6]\b[^>]*>([\s\S]*?)<\/h[1-6]\s*>/iu);
    if (!headingMatch) continue;
    const label = htmlText(headingMatch[1]);
    const field = /^prep(?:aration)?\s*time$/iu.test(label)
      ? "prepTime"
      : /^cook(?:ing)?\s*time$/iu.test(label)
        ? "cookTime"
        : /^total\s*time$/iu.test(label)
          ? "totalTime"
          : null;
    if (!field || durations.has(field)) continue;
    const paragraphValues = [...time.innerHtml.matchAll(/<p\b[^>]*>([\s\S]*?)<\/p\s*>/giu)]
      .map((match) => htmlText(match[1]));
    const raw = paragraphValues.find((value) => durationMinutes(value) !== null) ?? null;
    const value = raw === null ? null : durationMinutes(raw);
    if (value === null) continue;
    durations.set(field, {
      raw,
      value,
      confidence: "high",
      provenance: provenance("text/html", `.recipe-time:nth-child(${index + 1})`),
    });
  }

  if (!durations.has("totalTime")) {
    const compactContainer = elementsWithClass(html, "recipe-time-container").find(
      (container) => elementsWithClass(container.innerHtml, "recipe-time").length === 0,
    );
    const raw = compactContainer
      ? [...compactContainer.innerHtml.matchAll(/<p\b[^>]*>([\s\S]*?)<\/p\s*>/giu)]
        .map((match) => htmlText(match[1]))
        .find((value) => durationMinutes(value) !== null) ?? null
      : null;
    const value = raw === null ? null : durationMinutes(raw);
    if (value !== null) {
      durations.set("totalTime", {
        raw,
        value,
        confidence: "high",
        provenance: provenance("text/html", ".recipe-time-container > p"),
      });
    }
  }
  return durations;
}

function tastyImageReferences(html) {
  const containers = [
    ...elementsWithClass(html, "video-thumbnail"),
    ...elementsWithClass(html, "main-image"),
  ].sort((left, right) => left.index - right.index);
  const images = [];
  const seenSourceUrls = new Set();
  for (const container of containers) {
    for (const match of container.innerHtml.matchAll(/<img\b([^>]*)>/giu)) {
      const attributes = htmlAttributes(match[1]);
      const sourceUrl = normalizeWhitespace(attributes.get("src") ?? "");
      if (seenSourceUrls.has(sourceUrl)) continue;
      try {
        const parsed = new URL(sourceUrl);
        if (parsed.protocol !== "http:" && parsed.protocol !== "https:") continue;
      } catch {
        continue;
      }
      seenSourceUrls.add(sourceUrl);
      images.push({
        sourceUrl,
        alt: normalizeWhitespace(attributes.get("alt") ?? "") || null,
        credit: null,
        provenance: provenance("text/html", `.${hasClass(container.attributes, "video-thumbnail") ? "video-thumbnail" : "main-image"} img`),
      });
    }
  }
  return images;
}

function parseTastyVisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  if (!tastyHostname(sourceUrl)) return null;
  const titles = elementsWithClass(inertHtml, "recipe-name");
  const ingredientContainers = elementsWithClass(inertHtml, "ingredients-prep");
  const preparationContainers = elementsWithClass(inertHtml, "preparation");
  if (titles.length !== 1 || ingredientContainers.length !== 1 || preparationContainers.length !== 1) {
    return null;
  }
  const title = htmlText(titles[0].innerHtml);
  const ingredientSections = tastyIngredientSections(ingredientContainers[0].innerHtml);
  const instructionExtraction = tastyInstructionSections(preparationContainers[0].innerHtml);
  if (!title || !ingredientSections || !instructionExtraction) return null;

  const result = candidate(
    {
      id: "tasty-visible-recipe",
      version: "1",
      tier: "source_adapter",
      sourceAdapter: "tasty",
    },
    sourceUrl,
  );
  result.source.provider = "Tasty";
  result.source.provenance.provider = provenance("source_url", "hostname tasty.co");
  const author = firstClassText(inertHtml, "byline");
  if (author) {
    result.source.author = author.text;
    result.source.provenance.author = provenance("text/html", ".byline");
  }
  result.fields.title = scalar(title, "text/html", ".recipe-name");
  const description = firstClassText(inertHtml, "description");
  if (description) result.fields.description = scalar(description.text, "text/html", ".description");
  const durations = tastyDurations(inertHtml);
  result.fields.prepTime = durations.get("prepTime") ?? null;
  result.fields.cookTime = durations.get("cookTime") ?? null;
  result.fields.totalTime = durations.get("totalTime") ?? null;

  const servings = firstClassText(ingredientContainers[0].innerHtml, "servings-display");
  if (servings) {
    const servingsMatch = servings.text.match(/\b(\d+(?:[.,]\d+)?)\s+servings?\b/iu);
    const numericServings = servingsMatch
      ? Number.parseFloat(servingsMatch[1].replace(",", "."))
      : null;
    result.fields.yield = {
      raw: servings.text,
      value: {
        display: servings.text,
        servings: Number.isSafeInteger(numericServings) && numericServings > 0
          ? numericServings
          : null,
      },
      confidence: "high",
      provenance: provenance("text/html", ".servings-display"),
    };
  }
  result.fields.ingredientSections = ingredientSections;
  result.fields.instructionSections = instructionExtraction.sections;

  if (ingredientSections.some(({ name }) => name !== null)) {
    result.status = "needs_review";
    result.warnings.push({
      code: "SECTION_NAMES_NOT_PERSISTED",
      severity: "warning",
      path: "fields.ingredientSections",
      message: "Section names remain in preview but the current editor stores flat recipe content.",
    });
  }
  if (instructionExtraction.discardedPromotionCount > 0) {
    result.discardedContent.push({
      kind: "provider_promotion",
      count: instructionExtraction.discardedPromotionCount,
      reason: "Tasty app download prompts are not recipe instructions.",
    });
  }

  result.fields.images = tastyImageReferences(inertHtml);
  if (result.fields.images.length > 0) {
    result.status = "needs_review";
    result.warnings.push(...result.fields.images.map((_, index) => ({
      code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION",
      severity: "info",
      path: `fields.images[${index}]`,
      message: "The image reference is not fetched and requires confirmation before mapping.",
    })));
  }
  appendHtmlAuditEvidence(
    result,
    htmlAudit,
    discardedRemoteResourceCount(htmlAudit, result.fields.images),
  );
  return result;
}

function recipeTinEatsHostname(sourceUrl) {
  if (typeof sourceUrl !== "string") return null;
  try {
    const parsed = new URL(sourceUrl);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") return null;
    const hostname = parsed.hostname.toLowerCase();
    return hostname === "recipetineats.com" || hostname.endsWith(".recipetineats.com")
      ? hostname
      : null;
  } catch {
    return null;
  }
}

function wprmDuration(cardHtml, className) {
  const components = elementsWithClass(cardHtml, className).filter(
    (component, componentIndex, matches) => !matches.some(
      (possibleParent, parentIndex) =>
        parentIndex !== componentIndex &&
        possibleParent.index < component.index &&
        component.end <= possibleParent.contentEnd,
    ),
  );
  const raw = normalizeWhitespace(
    components.map(({ innerHtml }) => htmlText(innerHtml)).filter(Boolean).join(" "),
  );
  if (!raw) return null;
  const value = durationMinutes(raw);
  if (value === null) return null;
  return {
    raw,
    value,
    confidence: "high",
    provenance: provenance("text/html", `.${className}`),
  };
}

function wprmIngredient(itemHtml, locator) {
  const amount = firstClassText(itemHtml, "wprm-recipe-ingredient-amount")?.text ?? null;
  const sourceUnit = firstClassText(itemHtml, "wprm-recipe-ingredient-unit")?.text ?? null;
  const ingredientName = firstClassText(itemHtml, "wprm-recipe-ingredient-name")?.text ?? null;
  const notes = firstClassText(itemHtml, "wprm-recipe-ingredient-notes")?.text ?? "";
  if (!ingredientName) return null;

  // WPRM can repeat the final unit of a dual-system amount in its unit span.
  // Suppress only that repeated suffix; a separate package unit still matters.
  const unitIsAmountSuffix = amount && sourceUnit &&
    amount.toLocaleLowerCase("en-US").endsWith(sourceUnit.toLocaleLowerCase("en-US"));
  const unit = unitIsAmountSuffix ? null : sourceUnit;
  const notesSeparator = notes && !/^[,.;:!?]/u.test(notes) ? " " : "";
  const name = normalizeWhitespace(`${ingredientName}${notesSeparator}${notes}`);
  const raw = normalizeWhitespace([amount, unit, name].filter(Boolean).join(" "));
  return {
    raw,
    amount,
    unit,
    name,
    confidence: "high",
    provenance: provenance("text/html", locator),
  };
}

function wprmIngredientSections(containerHtml, parseIngredient = wprmIngredient) {
  const groups = elementsWithClass(containerHtml, "wprm-recipe-ingredient-group");
  const sectionSources = groups.length > 0
    ? groups.map((group) => group.innerHtml)
    : [containerHtml];
  const sections = [];
  for (const [sectionIndex, sectionHtml] of sectionSources.entries()) {
    const name = firstClassText(sectionHtml, "wprm-recipe-ingredient-group-name")?.text ?? null;
    const itemElements = elementsWithClass(sectionHtml, "wprm-recipe-ingredient");
    const items = itemElements.map((item, itemIndex) => parseIngredient(
      item.innerHtml,
      `.wprm-recipe-ingredient-group:nth-child(${sectionIndex + 1}) .wprm-recipe-ingredient:nth-child(${itemIndex + 1})`,
    ));
    if (items.length === 0 || items.some((item) => item === null)) return null;
    sections.push({ name, items });
  }
  return sections;
}

function wprmInstructionSections(containerHtml) {
  const groups = elementsWithClass(containerHtml, "wprm-recipe-instruction-group");
  const sectionSources = groups.length > 0
    ? groups.map((group) => group.innerHtml)
    : [containerHtml];
  const sections = [];
  let ignoredEmptySections = 0;
  for (const [sectionIndex, sectionHtml] of sectionSources.entries()) {
    const name = firstClassText(sectionHtml, "wprm-recipe-instruction-group-name")?.text ?? null;
    const itemElements = elementsWithClass(sectionHtml, "wprm-recipe-instruction");
    if (itemElements.length === 0) {
      const instructionLists = elementsWithClass(sectionHtml, "wprm-recipe-instructions");
      const recognizedEmptyStructure = [
        ...elementsWithClass(sectionHtml, "wprm-recipe-instruction-group-name"),
        ...instructionLists,
      ].sort((left, right) => right.index - left.index);
      let remainingHtml = sectionHtml;
      for (const element of recognizedEmptyStructure) {
        remainingHtml = remainingHtml.slice(0, element.index) + remainingHtml.slice(element.end);
      }
      const residualMarkup = remainingHtml
        .replace(/<!--[\s\S]*?-->/gu, "")
        .replace(/<\/?(?:div|section)\b[^>]*>/giu, "")
        .trim();
      const structurallyEmpty = instructionLists.length > 0 &&
        instructionLists.every(({ innerHtml }) => htmlText(innerHtml) === "") &&
        residualMarkup === "";
      if (!structurallyEmpty) return null;
      ignoredEmptySections += 1;
      continue;
    }
    const steps = itemElements.map((item, itemIndex) => {
      const raw = firstClassText(item.innerHtml, "wprm-recipe-instruction-text")?.text ?? null;
      return raw
        ? instruction(
          raw,
          "text/html",
          `.wprm-recipe-instruction-group:nth-child(${sectionIndex + 1}) .wprm-recipe-instruction:nth-child(${itemIndex + 1})`,
        )
        : null;
    });
    if (steps.length === 0 || steps.some((step) => step === null)) return null;
    sections.push({ name, steps });
  }
  return sections.length > 0 ? { sections, ignoredEmptySections } : null;
}

function parseRecipeTinEatsWprm(inertHtml, sourceUrl, htmlAudit) {
  if (!recipeTinEatsHostname(sourceUrl)) return null;
  const completeCards = elementsWithClass(inertHtml, "wprm-recipe-container").flatMap((card) => {
    const title = firstClassText(card.innerHtml, "wprm-recipe-name");
    const ingredientContainer = elementsWithClass(
      card.innerHtml,
      "wprm-recipe-ingredients-container",
    )[0];
    const instructionContainer = elementsWithClass(
      card.innerHtml,
      "wprm-recipe-instructions-container",
    )[0];
    if (!title || !ingredientContainer || !instructionContainer) return [];
    const ingredientSections = wprmIngredientSections(ingredientContainer.innerHtml);
    const instructionExtraction = wprmInstructionSections(instructionContainer.innerHtml);
    return ingredientSections && instructionExtraction
      ? [{
        card,
        title,
        ingredientSections,
        instructionSections: instructionExtraction.sections,
        ignoredEmptySections: instructionExtraction.ignoredEmptySections,
      }]
      : [];
  });
  // Generic region scoring retains competing-card evidence; an adapter must not
  // silently erase that ambiguity by selecting the first structurally valid card.
  if (completeCards.length !== 1) return null;
  const [{
    card,
    title,
    ingredientSections,
    instructionSections,
    ignoredEmptySections,
  }] = completeCards;

  const result = candidate(
    {
      id: "recipetineats-wprm",
      version: "1",
      tier: "source_adapter",
      sourceAdapter: "recipetineats",
    },
    sourceUrl,
  );
  result.source.provider = "RecipeTinEats";
  result.source.provenance.provider = provenance("source_url", "hostname recipetineats.com");
  const author = firstClassText(card.innerHtml, "wprm-recipe-author");
  if (author) {
    result.source.author = author.text;
    result.source.provenance.author = provenance("text/html", ".wprm-recipe-author");
  }
  result.fields.title = scalar(title.text, "text/html", ".wprm-recipe-name");
  const summary = firstClassText(card.innerHtml, "wprm-recipe-summary");
  if (summary) result.fields.description = scalar(summary.text, "text/html", ".wprm-recipe-summary");
  result.fields.prepTime = wprmDuration(card.innerHtml, "wprm-recipe-prep_time");
  result.fields.cookTime = wprmDuration(card.innerHtml, "wprm-recipe-cook_time");
  result.fields.totalTime = wprmDuration(card.innerHtml, "wprm-recipe-total_time");

  const servings = firstClassText(card.innerHtml, "wprm-recipe-servings");
  const servingsUnit = firstClassText(card.innerHtml, "wprm-recipe-servings-unit");
  if (servings) {
    const display = normalizeWhitespace([servings.text, servingsUnit?.text].filter(Boolean).join(" "));
    const numericServings = Number.parseFloat(servings.text.replace(",", "."));
    result.fields.yield = {
      raw: display,
      value: {
        display,
        servings: Number.isSafeInteger(numericServings) && numericServings > 0
          ? numericServings
          : null,
      },
      confidence: "high",
      provenance: provenance("text/html", ".wprm-recipe-servings"),
    };
  }
  result.fields.ingredientSections = ingredientSections;
  result.fields.instructionSections = instructionSections;

  const hasNamedSections = ingredientSections.some(({ name }) => name !== null) ||
    instructionSections.some(({ name }) => name !== null);
  if (hasNamedSections) {
    result.status = "needs_review";
    result.warnings.push({
      code: "SECTION_NAMES_NOT_PERSISTED",
      severity: "warning",
      path: "fields.ingredientSections",
      message: "Section names remain in preview but the current editor stores flat recipe content.",
    });
  }
  if (ignoredEmptySections > 0) {
    result.status = "needs_review";
    result.warnings.push({
      code: "EMPTY_PROVIDER_SECTION_IGNORED",
      severity: "warning",
      path: "fields.instructionSections",
      message: "Empty provider instruction sections were ignored while complete sections were retained.",
    });
  }
  const roots = rootRanges(inertHtml);
  const root = closestRoot(roots, card.index);
  const imageExtraction = imageReferences(inertHtml, roots, root);
  result.fields.images = imageExtraction.images;
  if (result.fields.images.length > 0) {
    result.status = "needs_review";
    if (imageExtraction.attribution) {
      result.source.attribution = imageExtraction.attribution.value;
      result.source.provenance.attribution = imageExtraction.attribution.provenance;
    }
    result.warnings.push(...result.fields.images.map((_, index) => ({
      code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION",
      severity: "info",
      path: `fields.images[${index}]`,
      message: "The image reference is not fetched and requires confirmation before mapping.",
    })));
  }
  appendHtmlAuditEvidence(
    result,
    htmlAudit,
    discardedRemoteResourceCount(htmlAudit, result.fields.images),
  );
  return result;
}

function budgetBytesIngredient(itemHtml, locator) {
  const systems = elementsWithClass(itemHtml, "wprm-recipe-ingredient-unit-system")
    .filter(({ innerHtml }) => htmlText(innerHtml));
  // Parenthesized conversions are one source expression, not competing choices.
  // Preserve both measurements without inferring numerical equivalence.
  const hasConversion = systems.length === 2 && /^\([^()]+\)$/u.test(htmlText(systems[1].innerHtml));
  if (systems.length > 1 && !hasConversion) return null;
  for (const field of ["amount", "unit", "name", "notes"]) {
    const fields = elementsWithClass(itemHtml, `wprm-recipe-ingredient-${field}`)
      .filter(({ innerHtml }) => htmlText(innerHtml));
    if (hasConversion && ["amount", "unit"].includes(field)) {
      if (fields.length !== 2 || systems.some(({ innerHtml }) =>
        elementsWithClass(innerHtml, `wprm-recipe-ingredient-${field}`).length !== 1)) return null;
    } else if (fields.length > 1) return null;
  }
  const parsed = wprmIngredient(itemHtml, locator);
  if (!parsed) return null;
  // Strip only a terminal decimal dollar cost in a note. Keep footnote markers,
  // preparation text and the untouched source row available in the preview.
  const notes = firstClassText(itemHtml, "wprm-recipe-ingredient-notes")?.text ?? "";
  const withoutCost = notes.replace(/\$\d+\.\d{2}(?=[*¹²³⁴⁵⁶⁷⁸⁹⁰†‡]*\s*\)+$)/u, "");
  const cleanedNotes = withoutCost === notes ? notes : normalizeWhitespace(withoutCost
    .replace(/\(\s*\)/gu, "").replace(/,?\s+\)/gu, ")"));
  const ingredientName = firstClassText(itemHtml, "wprm-recipe-ingredient-name").text;
  parsed.name = normalizeWhitespace(`${ingredientName}${cleanedNotes && !/^[,.;:!?]/u.test(cleanedNotes) ? " " : ""}${cleanedNotes}`);
  if (hasConversion) {
    parsed.amount = normalizeWhitespace(systems.map(({ innerHtml }) => htmlText(innerHtml)).join(" "))
      .replace(/\(\s+/gu, "(").replace(/\s+\)/gu, ")");
    parsed.unit = null;
    parsed.raw = normalizeWhitespace([parsed.amount, ingredientName, notes].filter(Boolean).join(" "));
  }
  return parsed;
}

function parseBudgetBytesWprm(inertHtml, sourceUrl, htmlAudit) {
  try {
    const url = new URL(sourceUrl);
    if (!["http:", "https:"].includes(url.protocol) ||
      !(url.hostname === "budgetbytes.com" || url.hostname.endsWith(".budgetbytes.com"))) return null;
  } catch { return null; }
  const cards = elementsWithClass(inertHtml, "wprm-recipe-container");
  if (cards.length !== 1) return null;
  const cardHtml = cards[0].innerHtml;
  const titles = elementsWithClass(cardHtml, "wprm-recipe-name");
  const ingredients = elementsWithClass(cardHtml, "wprm-recipe-ingredients-container");
  const instructions = elementsWithClass(cardHtml, "wprm-recipe-instructions-container");
  if (titles.length !== 1 || !htmlText(titles[0].innerHtml) || ingredients.length !== 1 || instructions.length !== 1) return null;
  // Each row must have one complete text block; selecting the first can lose steps.
  if (elementsWithClass(instructions[0].innerHtml, "wprm-recipe-instruction")
    .some(({ innerHtml }) => elementsWithClass(innerHtml, "wprm-recipe-instruction-text")
      .filter((text) => htmlText(text.innerHtml)).length !== 1)) return null;
  const ingredientSections = wprmIngredientSections(ingredients[0].innerHtml, budgetBytesIngredient);
  const instructionExtraction = wprmInstructionSections(instructions[0].innerHtml);
  if (!ingredientSections || !instructionExtraction) return null;
  // Every row must belong to the retained groups; never drop stray card content.
  if (ingredientSections.flatMap(({ items }) => items).length !== elementsWithClass(cardHtml, "wprm-recipe-ingredient").length ||
    instructionExtraction.sections.flatMap(({ steps }) => steps).length !== elementsWithClass(cardHtml, "wprm-recipe-instruction").length) return null;
  const result = candidate({ id: "budgetbytes-wprm", version: "1", tier: "source_adapter", sourceAdapter: "budgetbytes" }, sourceUrl);
  result.source.provider = "BudgetBytes";
  result.source.provenance.provider = provenance("source_url", "hostname budgetbytes.com");
  const author = firstClassText(cardHtml, "wprm-recipe-author");
  if (author) {
    result.source.author = author.text;
    result.source.provenance.author = provenance("text/html", ".wprm-recipe-author");
  }
  result.fields.title = scalar(htmlText(titles[0].innerHtml), "text/html", ".wprm-recipe-name");
  const summary = firstClassText(cardHtml, "wprm-recipe-summary");
  if (summary) result.fields.description = scalar(summary.text.replace(/\s+([,.;:!?])/gu, "$1"), "text/html", ".wprm-recipe-summary");
  result.fields.prepTime = wprmDuration(cardHtml, "wprm-recipe-prep_time");
  result.fields.cookTime = wprmDuration(cardHtml, "wprm-recipe-cook_time");
  result.fields.totalTime = wprmDuration(cardHtml, "wprm-recipe-total_time");
  const servings = elementsWithClass(cardHtml, "wprm-recipe-servings");
  if (servings.length === 1) {
    const rawServings = servings[0].tag === "input" ? servings[0].attributes.get("value") ?? "" : htmlText(servings[0].innerHtml);
    const display = normalizeWhitespace(`${rawServings} ${firstClassText(cardHtml, "wprm-recipe-servings-unit")?.text ?? ""}`);
    if (rawServings.trim()) {
      const numeric = Number(rawServings);
      result.fields.yield = { raw: display, value: { display, servings: Number.isSafeInteger(numeric) && numeric > 0 ? numeric : null }, confidence: "high", provenance: provenance("text/html", ".wprm-recipe-servings") };
    }
  }
  result.fields.ingredientSections = ingredientSections;
  result.fields.instructionSections = instructionExtraction.sections;
  result.fields.notes = elementsWithClass(cardHtml, "wprm-recipe-notes")
    .map(({ innerHtml }) => htmlText(innerHtml)).filter(Boolean)
    .map((text) => scalar(text, "text/html", ".wprm-recipe-notes"));
  for (const container of elementsWithClass(cardHtml, "wprm-recipe-image")) {
    for (const match of container.innerHtml.matchAll(/<img\b([^>]*)>/giu)) {
      const attributes = htmlAttributes(match[1]);
      const imageUrl = normalizeWhitespace(attributes.get("src") ?? "");
      try { if (!["http:", "https:"].includes(new URL(imageUrl).protocol)) continue; } catch { continue; }
      if (result.fields.images.some(({ sourceUrl: existing }) => existing === imageUrl)) continue;
      result.fields.images.push({ sourceUrl: imageUrl, alt: normalizeWhitespace(attributes.get("alt") ?? "") || null, credit: null, provenance: provenance("text/html", ".wprm-recipe-image img") });
    }
  }
  if (ingredientSections.some(({ name }) => name) || instructionExtraction.sections.some(({ name }) => name)) {
    result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields.ingredientSections", message: "Section names remain in preview but the current editor stores flat recipe content." });
  }
  if (instructionExtraction.ignoredEmptySections) {
    result.warnings.push({ code: "EMPTY_PROVIDER_SECTION_IGNORED", severity: "warning", path: "fields.instructionSections", message: "Empty provider instruction sections were ignored while complete sections were retained." });
  }
  if (result.fields.notes.length) result.warnings.push({ code: "NOTES_REQUIRE_MAPPING", severity: "info", path: "fields.notes", message: "Notes remain separate until the user chooses where to place them." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "The image reference is not fetched and requires confirmation before mapping." })));
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

function parseAllRecipesVisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  try {
    const url = new URL(sourceUrl);
    if (!["http:", "https:"].includes(url.protocol) ||
      !["allrecipes.com", "www.allrecipes.com"].includes(url.hostname)) return null;
  } catch { return null; }

  const titles = elementsWithClass(inertHtml, "article-heading");
  const ingredientContainers = elementsWithClass(inertHtml, "mm-recipes-structured-ingredients");
  const instructionContainers = elementsWithClass(inertHtml, "mm-recipes-steps__content");
  if (titles.length !== 1 || titles[0].tag !== "h1" || !htmlText(titles[0].innerHtml) ||
    ingredientContainers.length !== 1 || instructionContainers.length !== 1) return null;
  const title = titles[0];
  const ingredientContainer = ingredientContainers[0];
  const instructionContainer = instructionContainers[0];
  // Clipboard fragments can retain explicitly hidden nodes. Check the node and
  // every enclosing hidden range rather than treating a class as visibility.
  const hiddenRanges = elementsMatchingAttributes(inertHtml, attributes => listIsHidden({ attributes }));
  const isHidden = (index, end = index + 1) => hiddenRanges.some(range =>
    (range.index <= index && index < range.end) ||
    (index <= range.index && range.index < end && htmlText(range.innerHtml)));
  if (isHidden(title.index, title.end) ||
    [ingredientContainer, instructionContainer].some(({ index }) => isHidden(index))) return null;
  const roots = rootRanges(inertHtml);
  const root = closestRoot(roots, title.index);
  // A whole-page copy must describe one coherent recipe, not disconnected cards.
  if (!(title.end <= ingredientContainer.index && ingredientContainer.end <= instructionContainer.index) ||
    !sameRoot(root, closestRoot(roots, ingredientContainer.index)) ||
    !sameRoot(root, closestRoot(roots, instructionContainer.index)) ||
    [title, ingredientContainer, instructionContainer].some(({ index }) => hasAncestorTag(inertHtml, index, "nav"))) return null;

  const ingredientHtml = ingredientContainer.innerHtml;
  const lists = elementsWithClass(ingredientHtml, "mm-recipes-structured-ingredients__list");
  const groupHeadings = elementsWithClass(ingredientHtml, "mm-recipes-structured-ingredients__list-heading");
  if (!lists.length) return null;
  const ingredientSections = [];
  for (const [sectionIndex, list] of lists.entries()) {
    const listOffset = ingredientContainer.openingEnd + list.openingEnd;
    if (list.tag !== "ul" || isHidden(ingredientContainer.openingEnd + list.index)) return null;
    const previousEnd = lists[sectionIndex - 1]?.end ?? 0;
    const headings = groupHeadings.filter(({ index, end }) => index >= previousEnd && end <= list.index);
    if (headings.length > 1) return null;
    const rows = elementsWithClass(list.innerHtml, "mm-recipes-structured-ingredients__list-item");
    if (!rows.length || rows.some(({ tag }) => tag !== "li") || rows.length !==
      elementsMatchingAttributes(list.innerHtml, (_, tag) => tag === "li").length) return null;
    const items = [];
    for (const [itemIndex, row] of rows.entries()) {
      const rowOffset = listOffset + row.openingEnd;
      if (isHidden(listOffset + row.index)) return null;
      const structured = {};
      let firstNamedFieldIndex = null;
      for (const [field, attribute] of [["amount", "quantity"], ["unit", "unit"], ["name", "name"]]) {
        const fields = elementsMatchingAttributes(row.innerHtml, (attributes) => attributes.get(`data-ingredient-${attribute}`) === "true");
        if (fields.length > 1) return null;
        if (fields.some(({ index, end }) => isHidden(rowOffset + index, rowOffset + end))) return null;
        if (field !== "amount" && fields.length) firstNamedFieldIndex = Math.min(firstNamedFieldIndex ?? fields[0].index, fields[0].index);
        structured[field] = fields.length ? htmlText(fields[0].innerHtml) || null : null;
      }
      if (!structured.name) return null;
      // Some clipboard rows lose only the quantity span, leaving its literal
      // fraction before the still-marked unit/name. Recover only that bounded
      // numeric prefix, never infer an amount from descriptive text.
      if (!structured.amount && firstNamedFieldIndex !== null) {
        const prefix = htmlText(row.innerHtml.slice(0, firstNamedFieldIndex));
        if (prefix) {
          if (!new RegExp(`^${TASTY_AMOUNT_PATTERN}$`, "u").test(prefix)) return null;
          structured.amount = prefix;
        }
      }
      // A changed row with unmarked preparation text must not silently lose it.
      if (normalizeWhitespace([structured.amount, structured.unit, structured.name].filter(Boolean).join(" ")) !== htmlText(row.innerHtml)) return null;
      items.push({
        raw: htmlText(row.innerHtml), ...structured, confidence: "high",
        provenance: provenance("text/html", `.mm-recipes-structured-ingredients__list:nth-of-type(${sectionIndex + 1}) .mm-recipes-structured-ingredients__list-item:nth-child(${itemIndex + 1})`),
      });
    }
    ingredientSections.push({ name: headings.length ? htmlText(headings[0].innerHtml).replace(/:\s*$/u, "") || null : null, items });
  }
  if (ingredientSections.flatMap(({ items }) => items).length !==
    elementsWithClass(ingredientHtml, "mm-recipes-structured-ingredients__list-item").length ||
    groupHeadings.some(({ index }) => !lists.some((list, i) => index >= (lists[i - 1]?.end ?? 0) && index < list.index))) return null;

  const instructionHtml = instructionContainer.innerHtml;
  const orderedLists = elementsWithClass(instructionHtml, "mntl-sc-block-group--OL");
  if (orderedLists.length !== 1 || orderedLists[0].tag !== "ol") return null;
  const instructionListOffset = instructionContainer.openingEnd + orderedLists[0].openingEnd;
  if (isHidden(instructionContainer.openingEnd + orderedLists[0].index)) return null;
  const stepRows = elementsWithClass(orderedLists[0].innerHtml, "mntl-sc-block-group--LI");
  if (!stepRows.length || stepRows.some(({ tag }) => tag !== "li") || stepRows.length !==
    elementsMatchingAttributes(orderedLists[0].innerHtml, (_, tag) => tag === "li")
      .filter(({ index }) => tagStackContext(orderedLists[0].innerHtml, index).length === 0).length) return null;
  const steps = [];
  let flattenedSingleSteps = 0;
  for (const [stepIndex, row] of stepRows.entries()) {
    const rowOffset = instructionListOffset + row.openingEnd;
    if (isHidden(instructionListOffset + row.index)) return null;
    // Photos, captions and ads may sit inside a step. Retain only direct recipe
    // paragraphs, joining multiple paragraphs without changing step numbering.
    const blocks = elementsMatchingAttributes(row.innerHtml, () => true)
      .filter(({ index }) => tagStackContext(row.innerHtml, index).length === 0);
    const texts = [];
    let previousEnd = 0;
    for (const block of blocks) {
      // Unknown paragraphs or loose text could be essential procedure. Refuse
      // this adapter instead of silently returning a shortened instruction.
      if (htmlText(row.innerHtml.slice(previousEnd, block.index))) return null;
      previousEnd = block.end;
      if (block.tag === "figure" || hasClass(block.attributes, "mntl-sc-block-adslot")) continue;
      if (block.tag === "div" && hasClass(block.attributes, "allrecipes-sc-block-featuredlink") &&
        hasClass(block.attributes, "mntl-sc-block-universal-featured-link--related")) continue;
      if (!hasClass(block.attributes, "mntl-sc-block-html")) {
        if (htmlText(block.innerHtml)) return null;
        continue;
      }
      if (isHidden(rowOffset + block.index, rowOffset + block.end)) return null;
      if (block.tag === "p") {
        const text = htmlText(block.innerHtml);
        if (text) texts.push(text);
        continue;
      }
      // One captured layout nests a single instruction in an OL followed by an
      // empty spacer. Retain that text without inventing a parent instruction.
      const nestedItems = elementsMatchingAttributes(block.innerHtml, (_, tag) => tag === "li");
      if (block.tag !== "ol" || nestedItems.length !== 1 ||
        tagStackContext(block.innerHtml, nestedItems[0].index).length !== 0 || !htmlText(nestedItems[0].innerHtml)) return null;
      texts.push(htmlText(nestedItems[0].innerHtml));
      flattenedSingleSteps += 1;
    }
    if (!texts.length || htmlText(row.innerHtml.slice(previousEnd))) return null;
    const text = texts.map(text => text.replace(/\s+([,.;:!?])/gu, "$1")).join(" ");
    steps.push(instruction(text, "text/html", `.mm-recipes-steps__content ol > li:nth-child(${stepIndex + 1}) > .mntl-sc-block-html`));
  }
  if (stepRows.length !== elementsWithClass(instructionHtml, "mntl-sc-block-group--LI").length) return null;

  const result = candidate({ id: "allrecipes-visible", version: "1", tier: "source_adapter", sourceAdapter: "allrecipes" }, sourceUrl);
  result.source.provider = "AllRecipes";
  result.source.provenance.provider = provenance("source_url", "hostname allrecipes.com");
  result.fields.title = scalar(htmlText(title.innerHtml), "text/html", "h1.article-heading");
  const headerHtml = inertHtml.slice(title.end, ingredientContainer.index);
  const descriptions = elementsWithClass(headerHtml, "article-subheading");
  if (descriptions.length === 1 && htmlText(descriptions[0].innerHtml)) {
    result.fields.description = scalar(htmlText(descriptions[0].innerHtml), "text/html", ".article-subheading");
  }
  // Submitted/author bylines are distinct from testers and image owners.
  const authorGroups = [...elementsWithClass(headerHtml, "mntl-bylines__group--ugc_recipe"), ...elementsWithClass(headerHtml, "mntl-bylines__group--author")];
  const authors = [...new Set(authorGroups.flatMap(({ innerHtml }) => elementsWithClass(innerHtml, "mntl-attribution__item-name").map(({ innerHtml }) => htmlText(innerHtml))).filter(Boolean))];
  if (authors.length === 1) {
    result.source.author = authors[0];
    result.source.provenance.author = provenance("text/html", ".mntl-bylines__group--ugc_recipe .mntl-attribution__item-name, .mntl-bylines__group--author .mntl-attribution__item-name");
  }
  const details = elementsWithClass(headerHtml, "mm-recipes-details");
  if (details.length > 1) return null;
  const metadata = new Map();
  for (const item of elementsWithClass(details[0]?.innerHtml ?? "", "mm-recipes-details__item")) {
    const labels = elementsWithClass(item.innerHtml, "mm-recipes-details__label");
    const values = elementsWithClass(item.innerHtml, "mm-recipes-details__value");
    if (labels.length !== 1 || values.length !== 1) return null;
    const label = htmlText(labels[0].innerHtml).replace(/:\s*$/u, "");
    const value = htmlText(values[0].innerHtml);
    if (metadata.has(label)) return null;
    metadata.set(label, value);
    const raw = `${label}: ${value}`;
    const locator = `.mm-recipes-details__item (${label})`;
    const duration = labeledDuration(raw);
    if (duration) result.fields[duration.field] = { ...scalar(raw, "text/html", locator), value: duration.value };
    else if (!["Servings", "Yield"].includes(label)) result.unclassifiedLines.push({ raw, provenance: provenance("text/html", locator) });
  }
  const servingText = metadata.get("Servings");
  const yieldText = metadata.get("Yield");
  if (servingText || yieldText) {
    const display = [servingText ? `${servingText} servings` : null, yieldText].filter(Boolean).join("; ");
    const numeric = servingText && /^\d+$/u.test(servingText) ? Number(servingText) : null;
    result.fields.yield = { ...scalar(display, "text/html", ".mm-recipes-details__item (Servings, Yield)"), value: { display, servings: Number.isSafeInteger(numeric) && numeric > 0 ? numeric : null } };
  }
  const selectedScales = elementsWithClass(ingredientHtml, "mm-recipes-serving-size-adjuster__input").filter(({ attributes }) => attributes.has("checked"));
  if (selectedScales.length > 1 || selectedScales.some(({ attributes }) => attributes.get("value") !== "1")) {
    if (result.fields.yield) result.fields.yield.value.servings = null;
    result.warnings.push({ code: "SCALED_INGREDIENTS_YIELD_UNCERTAIN", severity: "warning", path: "fields.yield", message: "Visible ingredient amounts are retained, but scaled servings cannot be inferred from the original recipe metadata. Review the yield." });
  }
  result.fields.ingredientSections = ingredientSections;
  result.fields.instructionSections = [{ name: null, steps }];
  if (flattenedSingleSteps) result.warnings.push({ code: "NESTED_SINGLE_STEP_FLATTENED", severity: "warning", path: "fields.instructionSections", message: "A single-item nested procedure was retained at its enclosing step position. Review its presentation." });
  result.fields.notes = elementsMatchingAttributes(instructionHtml, (attributes) =>
    hasClass(attributes, "allrecipes-sc-block-callout--cooksnote") ||
    (hasClass(attributes, "allrecipes-sc-block-callout") && hasClass(attributes, "theme-recipetip")))
    .flatMap(({ innerHtml }) => elementsWithClass(innerHtml, "mntl-sc-block-universal-callout__body"))
    .map(({ innerHtml }) => htmlText(innerHtml)).filter(Boolean)
    .map((text) => scalar(text, "text/html", ".allrecipes-sc-block-callout--cooksnote .mntl-sc-block-universal-callout__body, .allrecipes-sc-block-callout.theme-recipetip .mntl-sc-block-universal-callout__body"));
  // Only figures preceding the end of the recipe procedure are eligible. Never
  // retain review galleries or recommended-recipe images from the page footer.
  const recipeImageHtml = inertHtml.slice(title.end, instructionContainer.end);
  const imageExtraction = imageReferences(recipeImageHtml, rootRanges(recipeImageHtml), null, "data-src");
  result.fields.images = imageExtraction.images.filter((image, index, images) => images.findIndex(({ sourceUrl: url }) => url === image.sourceUrl) === index);
  if (ingredientSections.some(({ name }) => name)) result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields.ingredientSections", message: "Section names remain in preview but the current editor stores flat recipe content." });
  if (result.fields.notes.length) result.warnings.push({ code: "NOTES_REQUIRE_MAPPING", severity: "info", path: "fields.notes", message: "Notes remain separate until the user chooses where to place them." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "The image reference is not fetched and requires confirmation before mapping." })));
  if (result.unclassifiedLines.length) result.warnings.push({ code: "UNCLASSIFIED_PROVIDER_METADATA", severity: "warning", path: "unclassifiedLines", message: "Additional or unrecognized recipe metadata is retained for review, not silently mapped." });
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

function parseFood52VisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  try {
    const url = new URL(sourceUrl);
    if (!["http:", "https:"].includes(url.protocol) ||
      !["food52.com", "www.food52.com"].includes(url.hostname)) return null;
  } catch { return null; }

  const mains = elementsMatchingAttributes(inertHtml, (_, tag) => tag === "main");
  if (mains.length !== 1 || hasAncestorTag(inertHtml, mains[0].index, "nav")) return null;
  const articles = elementsMatchingAttributes(mains[0].innerHtml, (_, tag) => tag === "article");
  if (articles.length !== 1) return null;
  const article = articles[0];
  const articleOffset = mains[0].openingEnd + article.openingEnd;
  const articleHtml = article.innerHtml;
  const hiddenRanges = elementsMatchingAttributes(inertHtml, attributes =>
    listIsHidden({ attributes }) || hasClass(attributes, "hidden"));
  const isHidden = (element, offset = articleOffset, includeText = false) => hiddenRanges.some(range =>
    (range.index <= offset + element.index && offset + element.index < range.end) ||
    (includeText && offset + element.index <= range.index && range.index < offset + element.end && htmlText(range.innerHtml)));
  if (isHidden(mains[0], 0) || isHidden(article, mains[0].openingEnd)) return null;
  const headers = elementsMatchingAttributes(inertHtml, attributes => attributes.get("id") === "contentHeader");
  if (headers.length !== 1 || headers[0].tag !== "header" || headers[0].index < articleOffset ||
    headers[0].end > articleOffset + articleHtml.length || isHidden(headers[0], 0) ||
    hasAncestorTag(inertHtml, headers[0].index, "nav")) return null;
  const header = headers[0];
  const titles = elementsMatchingAttributes(articleHtml, (_, tag) => tag === "h1");
  if (titles.length !== 1 || !htmlText(titles[0].innerHtml) || isHidden(titles[0], articleOffset, true) ||
    articleOffset + titles[0].index < header.openingEnd || articleOffset + titles[0].end > header.contentEnd) return null;

  const directElements = html => elementsMatchingAttributes(html, () => true)
    .filter(({ index }) => tagStackContext(html, index).length === 0);
  // Account for all direct text between recognized nodes: a new paragraph or
  // loose procedure must not vanish just because existing selectors still match.
  const hasLooseText = (html, elements) => {
    let end = 0;
    for (const element of elements) {
      if (htmlText(html.slice(end, element.index))) return true;
      end = element.end;
    }
    return Boolean(htmlText(html.slice(end)));
  };
  const headings = elementsMatchingAttributes(articleHtml, (_, tag) => tag === "h2");
  const ingredientHeadings = headings.filter(h => htmlText(h.innerHtml) === "Ingredients");
  const directionHeadings = headings.filter(h => htmlText(h.innerHtml) === "Directions");
  if (ingredientHeadings.length !== 1 || directionHeadings.length !== 1) return null;
  const allArticleElements = elementsMatchingAttributes(articleHtml, () => true);
  const sections = [];
  let emptySectionNames = 0;
  for (const heading of [ingredientHeadings[0], directionHeadings[0]]) {
    const parentStart = tagStackContext(articleHtml, heading.index).at(-1)?.start;
    const container = allArticleElements.find(element => element.index === parentStart);
    if (!container || isHidden(container) || isHidden(heading, articleOffset, true) ||
      hasAncestorTag(articleHtml, container.index, "nav")) return null;
    const children = directElements(container.innerHtml);
    if (hasLooseText(container.innerHtml, children) || children.length !== 2 || children[0].tag !== "h2" ||
      children[1].tag !== "div" || !hasClass(children[1].attributes, "space-y-8")) return null;
    const wrapper = children[1];
    const wrapperOffset = articleOffset + container.openingEnd + wrapper.openingEnd;
    if (isHidden(wrapper, articleOffset + container.openingEnd)) return null;
    const groups = directElements(wrapper.innerHtml);
    if (!groups.length || hasLooseText(wrapper.innerHtml, groups)) return null;
    const extractedGroups = [];
    for (const [groupIndex, group] of groups.entries()) {
      const groupOffset = wrapperOffset + group.openingEnd;
      if (group.tag !== "div" || isHidden(group, wrapperOffset)) return null;
      const children = directElements(group.innerHtml);
      if (hasLooseText(group.innerHtml, children) || ![1, 2].includes(children.length) ||
        (children.length === 2 && children[0].tag !== "h3")) return null;
      const list = children.at(-1);
      const name = children.length === 2 ? htmlText(children[0].innerHtml) || null : null;
      if (children.length === 2 && !name) emptySectionNames += 1;
      if ((children.length === 2 && isHidden(children[0], groupOffset, true)) || list.tag !== "ul" ||
        !hasClass(list.attributes, "space-y-4") || isHidden(list, groupOffset)) return null;
      const listOffset = groupOffset + list.openingEnd;
      const rows = directElements(list.innerHtml);
      if (!rows.length || hasLooseText(list.innerHtml, rows) || rows.some(row => row.tag !== "li" || isHidden(row, listOffset, true))) return null;
      extractedGroups.push({ name, rows, listOffset, groupIndex });
    }
    sections.push({ container, groups: extractedGroups });
  }
  if (!(header.end <= articleOffset + sections[0].container.index &&
    sections[0].container.end <= sections[1].container.index)) return null;
  // Recognizing the usual containers must not hide an additional recipe group
  // elsewhere in a changed article layout. All marked recipe lists and step
  // labels must belong to the completely accounted-for core sections.
  if (allArticleElements.some(element =>
    ((element.tag === "ul" && hasClass(element.attributes, "space-y-4")) ||
      (element.tag === "span" && /^Step \d+$/u.test(htmlText(element.innerHtml)))) &&
    !sections.some(({ container }) => container.index <= element.index && element.end <= container.end))) return null;

  const result = candidate({ id: "food52-visible", version: "1", tier: "source_adapter", sourceAdapter: "food52" }, sourceUrl);
  result.source.provider = "Food52";
  result.source.provenance.provider = provenance("source_url", "hostname food52.com");
  result.fields.title = scalar(htmlText(titles[0].innerHtml), "text/html", "main article #contentHeader h1");
  const rangePattern = `${TASTY_AMOUNT_PATTERN}(?:\\s*(?:to|[-–—])\\s*${TASTY_AMOUNT_PATTERN})?`;
  const measurementPattern = new RegExp(`^((?:[Aa]bout\\s+)?${rangePattern})(?:\\s+(.+))?$`, "u");
  let unstructuredRows = 0;
  let uncertainMeasurements = 0;
  result.fields.ingredientSections = sections[0].groups.map(({ name, rows, groupIndex }) => ({
    name,
    items: rows.map((row, itemIndex) => {
      const locator = `main article Ingredients group ${groupIndex + 1} li:nth-child(${itemIndex + 1})`;
      const raw = htmlText(row.innerHtml);
      const children = directElements(row.innerHtml);
      const measurements = children.filter(e => e.tag === "span" && hasClass(e.attributes, "font-bold"));
      if (measurements.length !== 1 || children.length !== 1 || htmlText(row.innerHtml.slice(0, measurements[0].index))) {
        unstructuredRows += 1;
        return { ...tastyIngredient(raw, "text/html", locator), confidence: "low" };
      }
      const marked = htmlText(measurements[0].innerHtml);
      const parsed = marked.match(measurementPattern);
      const ingredientName = htmlText(row.innerHtml.slice(measurements[0].end));
      if (!ingredientName || !marked) return null;
      if (!parsed) uncertainMeasurements += 1;
      return { raw, amount: parsed?.[1] ?? marked, unit: parsed?.[2] ?? null, name: ingredientName,
        confidence: parsed ? "high" : "low", provenance: provenance("text/html", locator) };
    }),
  }));
  if (result.fields.ingredientSections.some(({ items }) => items.some(item => !item || !item.raw))) return null;
  for (const { name, rows, groupIndex } of sections[1].groups) {
    const steps = [];
    for (const [index, row] of rows.entries()) {
      const children = directElements(row.innerHtml);
      if (hasLooseText(row.innerHtml, children) || children.length !== 2 || children[0].tag !== "span" ||
        htmlText(children[0].innerHtml) !== `Step ${index + 1}` || children[1].tag !== "div" ||
        !hasClass(children[1].attributes, "RichText") || children[1].attributes.get("role") !== "document") return null;
      const body = children[1];
      const paragraphs = directElements(body.innerHtml);
      if (!paragraphs.length || hasLooseText(body.innerHtml, paragraphs) || paragraphs.some(p => p.tag !== "p") ||
        !htmlText(body.innerHtml)) return null;
      steps.push(instruction(htmlText(body.innerHtml), "text/html", `main article Directions group ${groupIndex + 1} li:nth-child(${index + 1}) .RichText`));
    }
    result.fields.instructionSections.push({ name, steps });
  }

  const headerHtml = articleHtml.slice(header.end - articleOffset, sections[0].container.index);
  const headerHtmlOffset = header.end;
  const descriptions = elementsWithClass(headerHtml, "RichText").filter(e => e.attributes.get("role") === "document");
  if (descriptions.length > 1) return null;
  if (descriptions.some(e => isHidden(e, headerHtmlOffset, true))) return null;
  if (descriptions.length && htmlText(descriptions[0].innerHtml)) {
    result.fields.description = scalar(htmlText(descriptions[0].innerHtml).replace(/\s+([,.;:!?])/gu, "$1"), "text/html", "main article .RichText before Ingredients");
  }
  const bylines = elementsMatchingAttributes(header.innerHtml, (_, tag) => tag === "p")
    .filter(e => /^By\s+/u.test(htmlText(e.innerHtml)));
  if (bylines.length > 1) return null;
  if (bylines.some(e => isHidden(e, header.openingEnd, true))) return null;
  if (bylines.length) {
    result.source.author = htmlText(bylines[0].innerHtml).replace(/^By\s+/u, "");
    result.source.provenance.author = provenance("text/html", "#contentHeader p (By)");
  }
  const labels = new Set();
  for (const item of elementsMatchingAttributes(headerHtml, (_, tag) => tag === "dl")) {
    if (isHidden(item, headerHtmlOffset, true)) return null;
    const parts = directElements(item.innerHtml);
    if (hasLooseText(item.innerHtml, parts) || parts.length !== 2 || parts[0].tag !== "dt" || parts[1].tag !== "dd") return null;
    const label = htmlText(parts[0].innerHtml);
    const value = htmlText(parts[1].innerHtml);
    if (!label || !value || labels.has(label)) return null;
    labels.add(label);
    const raw = `${label}: ${value}`;
    const locator = `main article dl (${label})`;
    if (label === "Serves") {
      const numeric = /^\d+$/u.test(value) ? Number(value) : null;
      result.fields.yield = { ...scalar(raw, "text/html", locator), value: { display: raw, servings: Number.isSafeInteger(numeric) && numeric > 0 ? numeric : null } };
    } else {
      const duration = labeledDuration(raw);
      if (duration) {
        if (result.fields[duration.field]) return null;
        result.fields[duration.field] = { ...scalar(raw, "text/html", locator), value: duration.value };
      }
      else result.unclassifiedLines.push({ raw, provenance: provenance("text/html", locator) });
    }
  }
  // Food52's hero is an IMG with a sibling photo-credit paragraph, not a FIGURE.
  // Keep only the pre-ingredient hero reference; never fetch image bytes.
  const images = elementsMatchingAttributes(headerHtml, (_, tag) => tag === "img");
  if (images.length === 1 && !isHidden(images[0], headerHtmlOffset)) {
    const url = normalizeWhitespace(images[0].attributes.get("src") ?? "");
    try {
      if (["http:", "https:"].includes(new URL(url).protocol)) {
        const credits = elementsMatchingAttributes(headerHtml, (_, tag) => tag === "p")
          .filter(e => !isHidden(e, headerHtmlOffset, true))
          .map(e => htmlText(e.innerHtml)).filter(text => /^Photo by\s+/u.test(text));
        result.fields.images.push({ sourceUrl: url, alt: normalizeWhitespace(images[0].attributes.get("alt") ?? "") || null,
          credit: credits.length === 1 ? credits[0].replace(/^Photo by\s+/u, "") : null,
          provenance: provenance("text/html", "main article img before Ingredients") });
      }
    } catch { /* Invalid image references are not eligible for import. */ }
  }
  if (unstructuredRows) result.warnings.push({ code: "UNSTRUCTURED_PROVIDER_INGREDIENT", severity: "warning", path: "fields.ingredientSections", message: "Unmarked ingredient rows are retained with low confidence. Some may be section labels or equipment; review them before saving." });
  if (uncertainMeasurements) result.warnings.push({ code: "UNRECOGNIZED_PROVIDER_MEASUREMENT", severity: "warning", path: "fields.ingredientSections", message: "An unfamiliar marked measurement is preserved literally without guessing its amount/unit split." });
  if (emptySectionNames) result.warnings.push({ code: "EMPTY_PROVIDER_SECTION_NAME", severity: "warning", path: "fields.ingredientSections", message: "A complete recipe group has an empty source heading. Its content is retained without inventing a label; review the grouping." });
  if (result.fields.ingredientSections.some(s => s.name) || result.fields.instructionSections.some(s => s.name)) result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields.ingredientSections", message: "Section names remain in preview but the current editor stores flat recipe content." });
  if (result.unclassifiedLines.length) result.warnings.push({ code: "UNCLASSIFIED_PROVIDER_METADATA", severity: "warning", path: "unclassifiedLines", message: "Unrecognized recipe metadata is retained for review rather than silently mapped." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "The image reference is not fetched and requires confirmation before mapping." })));
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

function parseFoodComVisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  try {
    const url = new URL(sourceUrl);
    if (!["http:", "https:"].includes(url.protocol) || !["food.com", "www.food.com"].includes(url.hostname)) return null;
  } catch { return null; }
  const mains = elementsMatchingAttributes(inertHtml, (_, tag) => tag === "main");
  const primaryMains = mains.filter(element => element.attributes.get("id") === "recipe");
  // Food.com can append another recipe while the URL still names the first.
  // Its unique #recipe marker identifies the primary; never choose by length
  // or borrow fields from unmarked sibling recipes. Unknown roots stay ambiguous.
  if (primaryMains.length !== 1 || mains.some(element => !hasClass(element.attributes, "recipe") ||
    (element !== primaryMains[0] && element.attributes.has("id")))) return null;
  const main = primaryMains[0];
  if (mains.some((element, index) => mains.slice(index + 1).some(other =>
    element.index < other.end && other.index < element.end))) return null;
  const hiddenRanges = elementsMatchingAttributes(inertHtml, attributes => listIsHidden({ attributes }) || hasClass(attributes, "hidden"));
  // A malformed hidden wrapper still hides its descendants in the browser.
  // The complete-element scanner skips it, so retain its conservative range.
  let mainOpeningCount = 0;
  for (const match of inertHtml.matchAll(/<([a-z][a-z0-9:-]*)\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu)) {
    if (match[1].toLowerCase() === "main") mainOpeningCount += 1;
    const attributes = htmlAttributes(match[2]);
    if ((listIsHidden({ attributes }) || hasClass(attributes, "hidden")) && !hiddenRanges.some(e => e.index === match.index)) {
      const openingEnd = match.index + match[0].length;
      hiddenRanges.push({ index: match.index, end: inertHtml.length, innerHtml: inertHtml.slice(openingEnd) });
    }
  }
  if (mainOpeningCount !== mains.length) return null;
  const isHidden = (element, offset = main.openingEnd, includeText = false) => hiddenRanges.some(range =>
    (range.index <= offset + element.index && offset + element.index < range.end) ||
    (includeText && offset + element.index <= range.index && range.index < offset + element.end && htmlText(range.innerHtml)));
  if (isHidden(main, 0) || hasAncestorTag(inertHtml, main.index, "nav")) return null;
  const mainHtml = main.innerHtml;
  const layout = elementsWithClass(mainHtml, "layout__item");
  const containers = name => layout.filter(e => hasClass(e.attributes, name));
  const directElements = html => elementsMatchingAttributes(html, () => true).filter(e => tagStackContext(html, e.index).length === 0);
  const hasLooseText = (html, elements) => {
    let end = 0;
    for (const element of elements) { if (htmlText(html.slice(end, element.index))) return true; end = element.end; }
    return Boolean(htmlText(html.slice(end)));
  };
  // Sup/sub clipboard fractions introduce spaces around the fraction slash.
  // Normalize their typography without evaluating quantities or conversions.
  const text = html => htmlText(html).replace(/\s*⁄\s*/gu, "/").replace(/\s+([,.;:!?])/gu, "$1").replace(/\s+\)/gu, ")");
  const titleContainers = containers("title");
  const titles = elementsMatchingAttributes(mainHtml, (_, tag) => tag === "h1");
  if (titleContainers.length !== 1 || titles.length !== 1 || !text(titles[0].innerHtml) || isHidden(titles[0], main.openingEnd, true) ||
    titles[0].index < titleContainers[0].openingEnd || titles[0].end > titleContainers[0].contentEnd || hasAncestorTag(mainHtml, titles[0].index, "nav")) return null;
  const core = [];
  for (const kind of ["ingredient", "direction"]) {
    const section = containers(`${kind}s`);
    const lists = elementsWithClass(mainHtml, `${kind}-list`);
    if (section.length !== 1 || lists.length !== 1 || lists[0].tag !== "ul" || lists[0].index < section[0].openingEnd || lists[0].end > section[0].contentEnd ||
      isHidden(section[0]) || hasAncestorTag(mainHtml, section[0].index, "nav")) return null;
    const children = directElements(section[0].innerHtml);
    const headings = elementsMatchingAttributes(section[0].innerHtml, (_, tag) => tag === "h2");
    if (headings.length !== 1 || text(headings[0].innerHtml).toLowerCase() !== `${kind}s` ||
      !hasClass(headings[0].attributes, `${kind}s__title`) || isHidden(headings[0], main.openingEnd + section[0].openingEnd, true) ||
      children.length < 2 || children[0].tag !== "div" || !hasClass(children[0].attributes, "heading") ||
      children[1].index + section[0].openingEnd !== lists[0].index || hasLooseText(section[0].innerHtml, children) ||
      children.slice(2).some(e => htmlText(e.innerHtml))) return null;
    const headingChildren = directElements(children[0].innerHtml);
    if (hasLooseText(children[0].innerHtml, headingChildren) || headingChildren[0]?.tag !== "h2" ||
      headingChildren.slice(1).some(e => kind !== "ingredient" || e.tag !== "button" || !hasClass(e.attributes, "button--units") || !/^Units:\s*(US|Metric)$/u.test(text(e.innerHtml)))) return null;
    const rows = directElements(lists[0].innerHtml);
    const listOffset = main.openingEnd + lists[0].openingEnd;
    if (!rows.length || isHidden(lists[0]) || hasLooseText(lists[0].innerHtml, rows) || rows.some(e => e.tag !== "li" || isHidden(e, listOffset, true))) return null;
    core.push({ section: section[0], list: lists[0], rows });
  }
  if (!(titles[0].end <= core[0].section.index && core[0].section.end <= core[1].section.index)) return null;
  // Account for marked rows outside the selected lists as well as duplicates.
  if (elementsMatchingAttributes(mainHtml, attributes => hasClass(attributes, "ingredient-quantity") || hasClass(attributes, "direction") ||
    hasClass(attributes, "ingredient-heading") || hasClass(attributes, "direction-heading")).some(e => !core.some(({ list }) => list.openingEnd <= e.index && e.end <= list.contentEnd))) return null;
  const result = candidate({ id: "foodcom-visible", version: "1", tier: "source_adapter", sourceAdapter: "foodcom" }, sourceUrl);
  result.source.provider = "Food.com";
  result.source.provenance.provider = provenance("source_url", "hostname food.com");
  result.fields.title = scalar(text(titles[0].innerHtml), "text/html", "main#recipe .title h1");
  const amountExpression = new RegExp(`^${TASTY_AMOUNT_PATTERN}(?:\\s*(?:to|[-–—])\\s*${TASTY_AMOUNT_PATTERN})?$`, "iu");
  const units = `${UNIT_PATTERN}|fluid ounces?|fl\\.? oz|cartons?|containers?|boxes?|bottles?|jars?|quarts?|pints?|gallons?|dash(?:es)?|bunch(?:es)?|heads?`;
  const packageExpression = `\\(${TASTY_AMOUNT_PATTERN}\\s+(?:ounces?|oz|grams?|g|pounds?|lbs?)\\)\\s+(?:packages?|cartons?|cans?|containers?|boxes?|bottles?|jars?)`;
  const unitExpression = new RegExp(`^(${packageExpression}|${units})\\s+(.+)$`, "iu");
  let uncertainMeasurements = 0;
  for (const [kindIndex, { rows }] of core.entries()) {
    const kind = kindIndex === 0 ? "ingredient" : "direction";
    const groups = [];
    let current = null;
    for (const [index, row] of rows.entries()) {
      const children = directElements(row.innerHtml);
      if (children.length === 1 && children[0].tag === "h4" && hasClass(children[0].attributes, `${kind}-heading`) && !hasLooseText(row.innerHtml, children)) {
        const name = text(children[0].innerHtml);
        if (!name || (current && !current.items.length)) return null;
        current = { name, items: [] }; groups.push(current); continue;
      }
      if (!current) { current = { name: null, items: [] }; groups.push(current); }
      const locator = `main#recipe .${kind}-list li:nth-child(${index + 1})`;
      if (kindIndex === 0) {
        if (children.length !== 2 || children.some(e => e.tag !== "span") || !hasClass(children[0].attributes, "ingredient-quantity") ||
          !hasClass(children[1].attributes, "ingredient-text") || hasLooseText(row.innerHtml, children)) return null;
        const amount = text(children[0].innerHtml) || null;
        const body = text(children[1].innerHtml);
        if (!body || /<(?:ul|ol|h[1-6]|div|p)\b/iu.test(row.innerHtml)) return null;
        const measured = body.match(unitExpression);
        const uncertain = amount !== null && !amountExpression.test(amount);
        if (uncertain) uncertainMeasurements += 1;
        current.items.push({ raw: normalizeWhitespace(`${amount ?? ""} ${body}`), amount, unit: measured?.[1] ?? null, name: measured?.[2] ?? body,
          confidence: uncertain ? "low" : "high", provenance: provenance("text/html", locator) });
      } else {
        if (!hasClass(row.attributes, "direction") || !text(row.innerHtml) || /<(?:ul|ol|h[1-6]|div|p)\b/iu.test(row.innerHtml)) return null;
        current.items.push(instruction(text(row.innerHtml), "text/html", locator));
      }
    }
    if (!groups.length || groups.some(g => !g.items.length)) return null;
    if (kindIndex === 0) result.fields.ingredientSections = groups;
    else result.fields.instructionSections = groups.map(({ name, items }) => ({ name, steps: items }));
  }
  const authorContainers = containers("author-description");
  if (authorContainers.length > 1) return null;
  if (authorContainers.length) {
    const container = authorContainers[0];
    const offset = main.openingEnd + container.openingEnd;
    const bylines = elementsWithClass(container.innerHtml, "byline");
    const descriptions = elementsWithClass(container.innerHtml, "recipe-description");
    if (bylines.length > 1 || descriptions.length > 1 || [...bylines, ...descriptions].some(e => isHidden(e, offset, true))) return null;
    if (bylines.length) {
      const byline = text(bylines[0].innerHtml).match(/^Submitted by\s+(.+)$/u);
      if (!byline) return null;
      result.source.author = byline[1]; result.source.provenance.author = provenance("text/html", "main#recipe .byline");
    }
    if (descriptions.length) {
      let descriptionHtml = descriptions[0].innerHtml;
      // Keep the visible recipe copy, not the provider's checkbox label for
      // expanding it. Do not strip similarly worded prose or unknown controls.
      for (const control of elementsWithClass(descriptionHtml, "toggle").reverse()) {
        if (control.tag === "label" && /^(?:See More|Less)$/u.test(text(control.innerHtml)) &&
          elementsMatchingAttributes(control.innerHtml, (attributes, tag) => tag === "input" && attributes.get("type") === "checkbox").length === 1) {
          descriptionHtml = descriptionHtml.slice(0, control.index) + descriptionHtml.slice(control.end);
        }
      }
      if (text(descriptionHtml)) result.fields.description = scalar(text(descriptionHtml), "text/html", "main#recipe .recipe-description");
    }
  }
  const details = containers("details");
  if (details.length > 1) return null;
  const facts = new Map();
  if (details.length) for (const item of elementsWithClass(details[0].innerHtml, "facts__item")) {
    const labels = elementsWithClass(item.innerHtml, "facts__label");
    const values = elementsWithClass(item.innerHtml, "facts__value");
    if (labels.length !== 1 || values.length !== 1 || labels[0].tag !== "dt" || values[0].tag !== "dd" || isHidden(item, main.openingEnd + details[0].openingEnd, true)) return null;
    const label = text(labels[0].innerHtml).replace(/:$/u, "");
    if (!label || facts.has(label)) return null;
    let valueHtml = values[0].innerHtml;
    for (const controls of elementsWithClass(valueHtml, "adjust__controls").reverse()) valueHtml = valueHtml.slice(0, controls.index) + valueHtml.slice(controls.end);
    const value = text(valueHtml);
    if (!value) return null;
    facts.set(label, value);
    const raw = `${label}: ${value}`;
    if (label === "Ready In" && /^(?:\d+(?:[.,]\d+)?\s*(?:days?|d|hours?|hrs?|hr|h|minutes?|mins?|min|m)\s*)+$/iu.test(value)) {
      // Unlike the shared word-boundary parser, this anchored provider grammar
      // also supports compact adjacent tokens such as 1hr15mins.
      let minutes = 0;
      for (const part of value.matchAll(/(\d+(?:[.,]\d+)?)\s*(days?|d|hours?|hrs?|hr|h|minutes?|mins?|min|m)/giu)) {
        const multiplier = part[2].toLowerCase().startsWith("d") ? 1440 : part[2].toLowerCase().startsWith("h") ? 60 : 1;
        minutes += Number(part[1].replace(",", ".")) * multiplier;
      }
      if (Number.isSafeInteger(minutes) && minutes >= 0) result.fields.totalTime = { ...scalar(raw, "text/html", "main#recipe .facts (Ready In)"), value: minutes };
      else result.unclassifiedLines.push({ raw, provenance: provenance("text/html", "main#recipe .facts (Ready In)") });
    } else if (!["Yields", "Serves", "Ingredients"].includes(label)) result.unclassifiedLines.push({ raw, provenance: provenance("text/html", "main#recipe .facts") });
  }
  if (facts.has("Yields") || facts.has("Serves")) {
    const display = ["Yields", "Serves"].filter(label => facts.has(label)).map(label => `${label}: ${facts.get(label)}`).join("; ");
    const count = facts.get("Serves");
    const numeric = /^\d+$/u.test(count ?? "") ? Number(count) : null;
    result.fields.yield = { ...scalar(display, "text/html", "main#recipe .facts (Yields/Serves)"), value: { display, servings: Number.isSafeInteger(numeric) && numeric > 0 ? numeric : null } };
  }
  for (const container of containers("media")) for (const primary of elementsWithClass(container.innerHtml, "primary-image")) {
    const offset = main.openingEnd + container.openingEnd + primary.openingEnd;
    const credits = elementsWithClass(primary.innerHtml, "attribution").filter(e => !isHidden(e, offset, true));
    const credit = credits.length === 1 ? text(credits[0].innerHtml).replace(/^photo by\s+/iu, "") || null : null;
    for (const img of directElements(primary.innerHtml).filter(e => e.tag === "img" && !isHidden(e, offset))) {
      const url = normalizeWhitespace(img.attributes.get("src") ?? "");
      try { if (!["http:", "https:"].includes(new URL(url).protocol)) continue; } catch { continue; }
      if (!result.fields.images.some(image => image.sourceUrl === url)) result.fields.images.push({ sourceUrl: url, alt: normalizeWhitespace(img.attributes.get("alt") ?? "") || null, credit, provenance: provenance("text/html", "main#recipe .primary-image > img") });
    }
  }
  if (mains.length > 1) result.warnings.push({ code: "APPENDED_PROVIDER_RECIPES_IGNORED", severity: "warning", path: "fields.title", message: "Copied content contains appended recipes. The uniquely marked primary recipe was selected; verify it matches the source URL before saving." });
  if (uncertainMeasurements) result.warnings.push({ code: "UNRECOGNIZED_PROVIDER_MEASUREMENT", severity: "warning", path: "fields.ingredientSections", message: "An unfamiliar marked quantity is retained literally with low confidence; review before saving." });
  if (result.fields.ingredientSections.some(s => s.name) || result.fields.instructionSections.some(s => s.name)) result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields.ingredientSections", message: "Explicit group names remain in preview and optional commentary; the current editor stores flat content." });
  if (result.unclassifiedLines.length) result.warnings.push({ code: "UNCLASSIFIED_PROVIDER_METADATA", severity: "warning", path: "unclassifiedLines", message: "Unrecognized metadata is retained for review rather than guessed." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "Image references are unfetched and require confirmation before mapping." })));
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

function parseSallysVisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  try {
    const url = new URL(sourceUrl);
    if (!["http:", "https:"].includes(url.protocol) || !["sallysbakingaddiction.com", "www.sallysbakingaddiction.com"].includes(url.hostname)) return null;
  } catch { return null; }
  const coreClasses = ["tasty-recipes", "tasty-recipes-entry-header", "tasty-recipes-title", "tasty-recipes-ingredients", "tasty-recipes-ingredients-header", "tasty-recipes-ingredients-body", "tasty-recipes-instructions", "tasty-recipes-instructions-header", "tasty-recipes-instructions-body"];
  const optionalClasses = ["tasty-recipes-description-body", "tasty-recipes-details", "tasty-recipes-author-name", "tasty-recipes-prep-time", "tasty-recipes-cook-time", "tasty-recipes-total-time", "tasty-recipes-yield", "tasty-recipes-notes-body", "tasty-recipes-image"];
  const markedCore = attributes => coreClasses.some(name => hasClass(attributes, name)) || attributes.has("data-tr-ingredient-checkbox") || /^instruction-step-/u.test(attributes.get("id") ?? "");
  const coreElements = elementsMatchingAttributes(inertHtml, markedCore), complete = new Map();
  for (const name of [...coreClasses, ...optionalClasses]) complete.set(name, elementsWithClass(inertHtml, name));
  if (coreClasses.some(name => complete.get(name).length !== 1) || optionalClasses.some(name => complete.get(name).length > 1)) return null;
  const mains = elementsMatchingAttributes(inertHtml, (_, tag) => tag === "main");
  if (mains.length !== 1) return null;
  const [main] = mains, [card] = complete.get("tasty-recipes"), [header] = complete.get("tasty-recipes-entry-header"), [title] = complete.get("tasty-recipes-title");
  const inside = (element, container) => container.openingEnd <= element.index && element.end <= container.contentEnd;
  if (!inside(card, main) || !inside(header, card) || !inside(title, header) || title.tag !== "h2" || hasAncestorTag(inertHtml, card.index, "nav")) return null;
  const openingCounts = new Map([...complete.keys()].map(name => [name, 0]));
  const hiddenRanges = elementsMatchingAttributes(inertHtml, attributes => listIsHidden({ attributes }) || hasClass(attributes, "hidden") || hasClass(attributes, "is-hidden"));
  let coreOpenings = 0, mainOpenings = 0;
  for (const match of inertHtml.matchAll(/<([a-z][a-z0-9:-]*)\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu)) {
    const attributes = htmlAttributes(match[2]);
    if (match[1].toLowerCase() === "main") mainOpenings += 1;
    if (markedCore(attributes)) coreOpenings += 1;
    for (const name of openingCounts.keys()) if (hasClass(attributes, name)) openingCounts.set(name, openingCounts.get(name) + 1);
    if ((listIsHidden({ attributes }) || hasClass(attributes, "hidden") || hasClass(attributes, "is-hidden")) && !hiddenRanges.some(element => element.index === match.index)) hiddenRanges.push({ index: match.index, end: inertHtml.length, innerHtml: inertHtml.slice(match.index + match[0].length) });
  }
  if (mainOpenings !== 1 || coreOpenings !== coreElements.length || [...openingCounts].some(([name, count]) => count !== complete.get(name).length)) return null;
  const isHidden = (element, offset = 0, includeText = false) => hiddenRanges.some(range =>
    (range.index <= offset + element.index && offset + element.index < range.end) ||
    (includeText && offset + element.index <= range.index && range.index < offset + element.end && htmlText(range.innerHtml)));
  if (isHidden(main) || isHidden(card) || isHidden(header) || isHidden(title, 0, true)) return null;
  const text = html => htmlText(html).replace(/\s+([,.;:!?])/gu, "$1").replace(/\s+\)/gu, ")").replace(/\(\s+/gu, "(");
  if (!text(title.innerHtml)) return null;
  const directElements = html => elementsMatchingAttributes(html, () => true).filter(element => tagStackContext(html, element.index).length === 0);
  const hasLooseText = (html, elements) => {
    let end = 0;
    for (const element of elements) { if (text(html.slice(end, element.index))) return true; end = element.end; }
    return Boolean(text(html.slice(end)));
  };
  const consumedCore = new Set([card.index, header.index, title.index]);
  const result = candidate({ id: "sallys-visible", version: "1", tier: "source_adapter", sourceAdapter: "sallys-baking-addiction" }, sourceUrl);
  result.source.provider = "Sally's Baking Addiction";
  result.source.provenance.provider = provenance("source_url", "hostname sallysbakingaddiction.com");
  result.fields.title = scalar(text(title.innerHtml), "text/html", ".tasty-recipes-title");
  const bodies = new Map();
  for (const [name, label] of [["ingredients", "Ingredients"], ["instructions", "Instructions"]]) {
    const [container] = complete.get(`tasty-recipes-${name}`), [body] = complete.get(`tasty-recipes-${name}-body`), [headingContainer] = complete.get(`tasty-recipes-${name}-header`);
    const headings = elementsMatchingAttributes(headingContainer.innerHtml, (_, tag) => tag === "h3");
    if (!inside(container, card) || !inside(body, container) || !inside(headingContainer, container) || headingContainer.end > body.index || headings.length !== 1 || text(headings[0].innerHtml) !== label || [container, body, headingContainer].some(element => isHidden(element)) || isHidden(headings[0], headingContainer.openingEnd, true)) return null;
    const children = directElements(container.innerHtml), headerChildren = directElements(headingContainer.innerHtml);
    if (hasLooseText(container.innerHtml, children) || hasLooseText(headingContainer.innerHtml, headerChildren)) return null;
    for (const child of children) {
      if (container.openingEnd + child.index === headingContainer.index || container.openingEnd + child.index === body.index) continue;
      if (name === "ingredients" && child.tag === "div" && hasClass(child.attributes, "tasty-recipes-cook-mode") && text(child.innerHtml) === "Cook Mode Prevent your screen from going dark" && !/<(?:p|ul|ol|li|h[1-6])\b/iu.test(child.innerHtml)) continue;
      return null;
    }
    for (const child of headerChildren) {
      if (child.tag === "h3" && child.index === headings[0].index) continue;
      if (name === "ingredients" && child.tag === "div" && hasClass(child.attributes, "tasty-recipes-ingredients-clipboard-container") && text(child.innerHtml) === "Ingredients" && elementsMatchingAttributes(child.innerHtml, (_, tag) => tag === "h3").length === 1) continue;
      if (name === "ingredients" && child.tag === "div" && hasClass(child.attributes, "tasty-recipes-units-scale-container") && !text(child.innerHtml)) continue;
      return null;
    }
    for (const element of [container, body, headingContainer]) consumedCore.add(element.index);
    bodies.set(name, body);
  }
  if (header.end > complete.get("tasty-recipes-ingredients")[0].index || complete.get("tasty-recipes-ingredients")[0].end > complete.get("tasty-recipes-instructions")[0].index) return null;
  const isEmptyAd = element => element.tag === "div" && hasClass(element.attributes, "adthrive-ad") && !text(element.innerHtml);
  if (elementsWithClass(card.innerHtml, "adthrive-ad").some(element => !isEmptyAd(element))) return null;
  const amount = `(?:\\d+\\s+and\\s+\\d+/\\d+|\\d+[\\u00bc-\\u00be\\u2150-\\u215e]|${TASTY_AMOUNT_PATTERN})`;
  const range = `${amount}(?:\\s*(?:to|[-–—])\\s*${amount})?`, units = `${UNIT_PATTERN}|sticks?|quarts?|pints?`;
  const compound = new RegExp(`^(${range}\\s+(?:${units})[.]?\\s+(?:plus|minus)\\s+${range}\\s+(?:${units})[.]?)\\s+(.+)$`, "iu");
  const packageAmount = new RegExp(`^(${range})\\s+(\\(${amount}[- ](?:ounces?|oz|grams?|g|pounds?|lbs?)\\)\\s+(?:cans?|packages?|boxes?))\\s+(.+)$`, "iu");
  const measured = new RegExp(`^(${range})\\s+(${units})[.]?\\s+(.+)$`, "iu");
  const ingredientBody = bodies.get("ingredients"), ingredientBlocks = [];
  const collectIngredientBlocks = (html, offset, inLegacyWrapper = false) => {
    const blocks = directElements(html);
    if (hasLooseText(html, blocks)) return false;
    for (const block of blocks) {
      if (isHidden(block, offset)) return false;
      // One legacy card wraps a UL and the following H4 in ERSIngredients.
      // Flatten that explicit transparent wrapper, never arbitrary containers.
      if (block.tag === "div" && hasClass(block.attributes, "ERSIngredients") && !inLegacyWrapper) {
        if (!collectIngredientBlocks(block.innerHtml, offset + block.openingEnd, true)) return false;
      } else ingredientBlocks.push({ ...block, offset });
    }
    return true;
  };
  if (!collectIngredientBlocks(ingredientBody.innerHtml, ingredientBody.openingEnd)) return null;
  let pendingName = null, awaitingList = false, emptyPlaceholders = 0;
  for (const [blockIndex, block] of ingredientBlocks.entries()) {
    if ((block.tag === "p" && !text(block.innerHtml)) || isEmptyAd(block)) continue;
    // Gutenberg leaves empty H4/UL placeholders between complete groups. They
    // carry no content or name; never skip a named group's only empty list.
    const emptyEditorList = element => element?.tag === "ul" && hasClass(element.attributes, "wp-block-list") && !element.innerHtml.trim();
    const nextBlock = ingredientBlocks[blockIndex + 1];
    if (!awaitingList && (emptyEditorList(block) || (block.tag === "h4" && !text(block.innerHtml) && result.fields.ingredientSections.length && (emptyEditorList(nextBlock) || nextBlock?.tag === "h4")))) { emptyPlaceholders += 1; continue; }
    if (block.tag === "h4") {
      if (awaitingList || !text(block.innerHtml) || isHidden(block, block.offset, true)) return null;
      pendingName = text(block.innerHtml).replace(/:\s*$/u, "");
      if (!pendingName) return null;
      awaitingList = true; continue;
    }
    if (block.tag !== "ul" || (result.fields.ingredientSections.length && !awaitingList)) return null;
    const rows = directElements(block.innerHtml), offset = block.offset + block.openingEnd, items = [];
    if (!rows.length || hasLooseText(block.innerHtml, rows)) return null;
    for (const row of rows) {
      if (isEmptyAd(row)) continue;
      if (row.tag !== "li" || !row.attributes.has("data-tr-ingredient-checkbox") || isHidden(row, offset, true) || /<(?:ul|ol|li|div|h[1-6]|p|figure|table)\b/iu.test(row.innerHtml)) return null;
      const raw = text(row.innerHtml);
      if (!raw) return null;
      consumedCore.add(offset + row.index);
      // Scaling attributes and checkbox labels are not the clipboard's visible
      // quantity. Keep written mixed fractions and conversions without guesses.
      const expression = raw.match(compound), packaged = raw.match(packageAmount), measure = raw.match(measured);
      const locator = `.tasty-recipes-ingredients-body group ${result.fields.ingredientSections.length + 1} li:nth-child(${items.length + 1})`;
      items.push(expression ? { raw, amount: expression[1], unit: null, name: expression[2], confidence: "high", provenance: provenance("text/html", locator) }
        : packaged ? { raw, amount: packaged[1], unit: packaged[2], name: packaged[3], confidence: "high", provenance: provenance("text/html", locator) }
          : measure ? { raw, amount: measure[1], unit: measure[2], name: measure[3], confidence: "high", provenance: provenance("text/html", locator) }
            : ingredient(raw, "text/html", locator, range));
    }
    if (!items.length) return null;
    result.fields.ingredientSections.push({ name: pendingName, items }); pendingName = null; awaitingList = false;
  }
  if (!result.fields.ingredientSections.length || awaitingList) return null;
  const procedure = bodies.get("instructions"), blocks = directElements(procedure.innerHtml);
  if (hasLooseText(procedure.innerHtml, blocks)) return null;
  const lists = blocks.filter(block => block.tag === "ol");
  if (lists.length !== 1 || blocks.some(block => block !== lists[0] && !(block.tag === "p" && !text(block.innerHtml)) && !isEmptyAd(block))) return null;
  const list = lists[0], rows = directElements(list.innerHtml), offset = procedure.openingEnd + list.openingEnd, steps = [], stepIds = new Set();
  if (isHidden(list, procedure.openingEnd) || hasLooseText(list.innerHtml, rows)) return null;
  for (const row of rows) {
    if (isEmptyAd(row)) continue;
    const id = row.attributes.get("id");
    if (row.tag !== "li" || !/^instruction-step-\d+$/u.test(id ?? "") || stepIds.has(id) || isHidden(row, offset, true) || /<(?:ul|ol|li|div|h[1-6]|figure|table)\b/iu.test(row.innerHtml) || !text(row.innerHtml)) return null;
    consumedCore.add(offset + row.index); stepIds.add(id);
    steps.push(instruction(text(row.innerHtml), "text/html", `.tasty-recipes-instructions-body ol > li:nth-child(${steps.length + 1})`));
  }
  if (!steps.length || consumedCore.size !== coreElements.length || coreElements.some(element => !consumedCore.has(element.index))) return null;
  result.fields.instructionSections.push({ name: null, steps });
  const [description] = complete.get("tasty-recipes-description-body");
  if (description) {
    const paragraphs = directElements(description.innerHtml);
    if (!inside(description, card) || isHidden(description, 0, true) || paragraphs.some(element => !["p", "div"].includes(element.tag) || /<(?:p|div|ul|ol|li|h[1-6]|figure|table)\b/iu.test(element.innerHtml)) || hasLooseText(description.innerHtml, paragraphs)) return null;
    if (text(description.innerHtml)) result.fields.description = scalar(text(description.innerHtml), "text/html", ".tasty-recipes-description-body");
  }
  const [details] = complete.get("tasty-recipes-details");
  if (details) {
    const children = directElements(details.innerHtml);
    if (!inside(details, header) || isHidden(details) || children.length !== 1 || children[0].tag !== "ul" || hasLooseText(details.innerHtml, children)) return null;
    const detailList = children[0], items = directElements(detailList.innerHtml), offset = details.openingEnd + detailList.openingEnd, seenLabels = new Set();
    if (hasLooseText(detailList.innerHtml, items)) return null;
    for (const item of items) {
      const labels = elementsWithClass(item.innerHtml, "tasty-recipes-label");
      if (item.tag !== "li" || labels.length !== 1 || isHidden(item, offset, true)) return null;
      const label = text(labels[0].innerHtml).replace(/:+\s*$/u, "");
      if (!label || seenLabels.has(label)) return null;
      seenLabels.add(label);
      const mapping = { Author: ["author", "tasty-recipes-author-name"], "Prep Time": ["prepTime", "tasty-recipes-prep-time"], "Cook Time": ["cookTime", "tasty-recipes-cook-time"], "Total Time": ["totalTime", "tasty-recipes-total-time"], Yield: ["yield", "tasty-recipes-yield"] }[label];
      if (!mapping) { if (text(item.innerHtml)) result.unclassifiedLines.push({ raw: text(item.innerHtml), provenance: provenance("text/html", ".tasty-recipes-details li") }); continue; }
      const [field, className] = mapping, values = elementsWithClass(item.innerHtml, className), value = values.length === 1 ? text(values[0].innerHtml) : "";
      if (!value || complete.get(className).length !== 1 || offset + item.openingEnd + values[0].index !== complete.get(className)[0].index || text(item.innerHtml) !== `${text(labels[0].innerHtml)} ${value}`) return null;
      if (field === "author") { result.source.author = value; result.source.provenance.author = provenance("text/html", ".tasty-recipes-author-name"); }
      else if (field === "yield") {
        const count = value.match(/^(?:serves\s+(\d+)|(\d+)\s+servings?)$/iu), number = count ? Number(count[1] ?? count[2]) : null;
        const servings = Number.isSafeInteger(number) && number > 0 ? number : null;
        result.fields.yield = { ...scalar(value, "text/html", ".tasty-recipes-yield"), value: { display: value, servings } };
      } else {
        const token = "\\d+(?:\\.\\d+)?\\s*(?:days?|hours?|hrs?|hr|minutes?|mins?|min)";
        const exact = new RegExp(`^${token}(?:(?:\\s+|\\s+and\\s+|,\\s*(?:and\\s+)?)${token})*$`, "iu").test(value), duration = labeledDuration(`${label}: ${value}`);
        const minutes = exact && duration && Number.isSafeInteger(duration.value) && duration.value >= 0 ? duration.value : null;
        result.fields[field] = { ...scalar(value, "text/html", `.${className}`), value: minutes, confidence: minutes === null ? "low" : "high" };
        if (minutes === null) result.warnings.push({ code: "UNRESOLVED_PROVIDER_DURATION", severity: "warning", path: `fields.${field}`, message: "Qualified or unsupported duration text remains literal; exact minutes are not inferred." });
      }
    }
  }
  // Marked optional values outside their explicit card detail rows are not
  // missing values. Refuse instead of silently omitting unmatched metadata.
  for (const [name, field] of [["tasty-recipes-author-name", "author"], ["tasty-recipes-prep-time", "prepTime"], ["tasty-recipes-cook-time", "cookTime"], ["tasty-recipes-total-time", "totalTime"], ["tasty-recipes-yield", "yield"]]) {
    if (complete.get(name).length && !(field === "author" ? result.source.author : result.fields[field])) return null;
  }
  const [notes] = complete.get("tasty-recipes-notes-body");
  if (notes) {
    const blocks = directElements(notes.innerHtml);
    if (!inside(notes, card) || isHidden(notes, 0, true) || hasLooseText(notes.innerHtml, blocks)) return null;
    for (const block of blocks) {
      if (isEmptyAd(block) || (block.tag === "p" && !text(block.innerHtml))) continue;
      if (block.tag === "p" && !/<(?:ul|ol|li|h[1-6])\b/iu.test(block.innerHtml)) result.fields.notes.push(scalar(text(block.innerHtml), "text/html", ".tasty-recipes-notes-body p"));
      else if (["ol", "ul"].includes(block.tag)) {
        const rows = directElements(block.innerHtml);
        if (hasLooseText(block.innerHtml, rows)) return null;
        for (const row of rows) {
          if (isEmptyAd(row)) continue;
          if (row.tag !== "li" || !text(row.innerHtml) || /<(?:ul|ol|li|h[1-6]|figure|table)\b/iu.test(row.innerHtml)) return null;
          result.fields.notes.push(scalar(text(row.innerHtml), "text/html", ".tasty-recipes-notes-body li"));
        }
      } else return null;
    }
  }
  const [imageContainer] = complete.get("tasty-recipes-image");
  if (imageContainer && inside(imageContainer, header) && !isHidden(imageContainer)) {
    for (const image of elementsMatchingAttributes(imageContainer.innerHtml, (_, tag) => tag === "img")) {
      if (isHidden(image, imageContainer.openingEnd)) continue;
      // Lazy card images use a data: placeholder; retain the first safe src or
      // data-src URL as an unfetched reference, not the placeholder itself.
      const url = [image.attributes.get("src"), image.attributes.get("data-src")].find(value => { try { return ["http:", "https:"].includes(new URL(value).protocol); } catch { return false; } });
      if (url && !result.fields.images.some(retained => retained.sourceUrl === url)) result.fields.images.push({ sourceUrl: url, alt: image.attributes.get("alt") || null, credit: null, provenance: provenance("text/html", ".tasty-recipes-entry-header .tasty-recipes-image img") });
    }
  }
  const pageTitles = elementsMatchingAttributes(main.innerHtml, (_, tag) => tag === "h1").filter(element => main.openingEnd + element.end <= card.index && !isHidden(element, main.openingEnd, true));
  if (pageTitles.length === 1 && text(pageTitles[0].innerHtml) && text(pageTitles[0].innerHtml) !== result.fields.title.value) {
    result.competingCandidates.push({ reason: "Page title differs from the selected complete recipe card", title: text(pageTitles[0].innerHtml), provenance: provenance("text/html", "main h1 before .tasty-recipes") });
    result.warnings.push({ code: "COMPETING_RECIPE_REGION", severity: "warning", path: "competingCandidates[0]", message: "Confirm the embedded recipe card title; it differs from the surrounding page title." });
  }
  if (result.fields.ingredientSections.some(section => section.name)) result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields.ingredientSections", message: "Explicit group names remain in preview and optional commentary; the editor stores flat ingredients." });
  if (emptyPlaceholders) result.warnings.push({ code: "EMPTY_PROVIDER_PLACEHOLDER", severity: "warning", path: "fields.ingredientSections", message: "Structurally empty editor placeholders were ignored between complete ingredient content. Review group presentation." });
  if (result.unclassifiedLines.length) result.warnings.push({ code: "UNCLASSIFIED_PROVIDER_METADATA", severity: "warning", path: "unclassifiedLines", message: "Additional recipe metadata remains available for review, not silently mapped." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "Image references are unfetched and require confirmation before mapping." })));
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

// These exact-host adapters share the measured Dotdash article/card shape.
// Simply Recipes extensions stay opt-in; Serious Eats output stays unchanged.
function parseDotdashVisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  let simplyRecipes = false;
  try {
    const url = new URL(sourceUrl);
    simplyRecipes = ["simplyrecipes.com", "www.simplyrecipes.com"].includes(url.hostname);
    if (!["http:", "https:"].includes(url.protocol) || (!simplyRecipes && !["seriouseats.com", "www.seriouseats.com"].includes(url.hostname))) return null;
  } catch { return null; }
  const mains = elementsMatchingAttributes(inertHtml, (_, tag) => tag === "main");
  const articles = elementsMatchingAttributes(inertHtml, (_, tag) => tag === "article");
  const headers = elementsWithClass(inertHtml, "mm-recipes-article-header");
  const titles = elementsWithClass(inertHtml, "article-heading");
  const ingredients = elementsWithClass(inertHtml, "mm-recipes-structured-ingredients");
  const procedures = elementsWithClass(inertHtml, "mm-recipes-steps");
  const contents = elementsWithClass(inertHtml, "mm-recipes-steps__content");
  if ([mains, articles, headers, titles, ingredients, procedures, contents].some(elements => elements.length !== 1)) return null;
  const [main] = mains, [article] = articles, [header] = headers, [title] = titles, [ingredientContainer] = ingredients, [procedure] = procedures, [content] = contents;
  const inside = (element, container) => container.openingEnd <= element.index && element.end <= container.contentEnd;
  if (!inside(article, main) || !inside(header, article) || !inside(title, header) || !inside(ingredientContainer, article) || !inside(procedure, article) || !inside(content, procedure) ||
    title.tag !== "h1" || !htmlText(title.innerHtml) || header.end > ingredientContainer.index || ingredientContainer.end > procedure.index ||
    [main, article, header, ingredientContainer, procedure].some(element => hasAncestorTag(inertHtml, element.index, "nav"))) return null;
  const coreClasses = ["mm-recipes-article-header", "article-heading", "mm-recipes-structured-ingredients", "mm-recipes-structured-ingredients__heading", "mm-recipes-structured-ingredients__list", "mm-recipes-structured-ingredients__list-heading", "mm-recipes-structured-ingredients__list-item", "mm-recipes-steps", "mm-recipes-steps__heading", "mm-recipes-steps__content", "mntl-sc-block-group--OL", "mntl-sc-block-group--LI"];
  const metadataClasses = ["article-subheading", "mntl-bylines__group--author", "mntl-attribution__item-name", "mm-recipes-details", "mm-recipes-details__item", "mm-recipes-details__label", "mm-recipes-details__value", "mm-recipes-details__note"];
  if (simplyRecipes) metadataClasses.push("mm-recipes-details__title", "mm-recipes-details__content", "mm-recipes-details__note-label");
  const metadataOpenings = new Map(metadataClasses.map(name => [name, 0]));
  const markedCore = elementsMatchingAttributes(inertHtml, attributes => coreClasses.some(name => hasClass(attributes, name)));
  const hiddenRanges = elementsMatchingAttributes(inertHtml, attributes => listIsHidden({ attributes }) || hasClass(attributes, "is-hidden") || hasClass(attributes, "hidden"));
  let mainOpenings = 0, articleOpenings = 0, coreOpenings = 0;
  for (const match of inertHtml.matchAll(/<([a-z][a-z0-9:-]*)\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu)) {
    const attributes = htmlAttributes(match[2]), tag = match[1].toLowerCase();
    if (tag === "main") mainOpenings += 1;
    if (tag === "article") articleOpenings += 1;
    if (coreClasses.some(name => hasClass(attributes, name))) coreOpenings += 1;
    for (const name of metadataClasses) if (hasClass(attributes, name)) metadataOpenings.set(name, metadataOpenings.get(name) + 1);
    if ((listIsHidden({ attributes }) || hasClass(attributes, "is-hidden") || hasClass(attributes, "hidden")) && !hiddenRanges.some(element => element.index === match.index)) hiddenRanges.push({ index: match.index, end: inertHtml.length, innerHtml: inertHtml.slice(match.index + match[0].length) });
  }
  if (mainOpenings !== 1 || articleOpenings !== 1 || coreOpenings !== markedCore.length) return null;
  // Optional metadata may be absent, but an unmatched marked opening is not
  // evidence of absence: refuse rather than silently discarding its value.
  if (metadataClasses.some(name => metadataOpenings.get(name) !== elementsWithClass(inertHtml, name).length)) return null;
  const isHidden = (element, offset = 0, includeText = false) => hiddenRanges.some(range =>
    (range.index <= offset + element.index && offset + element.index < range.end) ||
    (includeText && offset + element.index <= range.index && range.index < offset + element.end && htmlText(range.innerHtml)));
  if ([main, article, header, ingredientContainer, procedure, content].some(element => isHidden(element)) || isHidden(title, 0, true)) return null;
  const directElements = html => elementsMatchingAttributes(html, () => true).filter(element => tagStackContext(html, element.index).length === 0);
  const hasLooseText = (html, elements) => {
    let end = 0;
    for (const element of elements) { if (htmlText(html.slice(end, element.index))) return true; end = element.end; }
    return Boolean(htmlText(html.slice(end)));
  };
  // Clipboard spans can split a word ("oi" + "l"). Preserve source adjacency
  // for inline markup instead of inserting spaces at every closing span.
  const text = html => htmlText(simplyRecipes ? html.replace(/<\/?(?:span|strong|em|b|i|a|sup|sub)\b(?:"[^"]*"|'[^']*'|[^>"'])*>/giu, "") : html).replace(/\s+([,.;:!?])/gu, "$1").replace(/\s+\)/gu, ")").replace(/\(\s+/gu, "(");
  const consumedCore = new Set([header.index, title.index, ingredientContainer.index, procedure.index, content.index]);
  const result = candidate({ id: simplyRecipes ? "simply-recipes-visible" : "serious-eats-visible", version: "1", tier: "source_adapter", sourceAdapter: simplyRecipes ? "simply-recipes" : "serious-eats" }, sourceUrl);
  result.source.provider = simplyRecipes ? "Simply Recipes" : "Serious Eats";
  result.source.provenance.provider = provenance("source_url", simplyRecipes ? "hostname simplyrecipes.com" : "hostname seriouseats.com");
  result.fields.title = scalar(text(title.innerHtml), "text/html", "h1.article-heading");
  const amount = `(?:${simplyRecipes ? "\\d+\\s+and\\s+\\d+/\\d+|" : ""}\\d+[\\u00bc-\\u00be\\u2150-\\u215e]|${TASTY_AMOUNT_PATTERN})`;
  const range = `${amount}(?:\\s*(?:to|[-–—])\\s*${amount})?`;
  const units = `${UNIT_PATTERN}|quarts?|pints?|sticks?|bunch(?:es)?|heads?${simplyRecipes ? "|pinch" : ""}`;
  const compound = new RegExp(`^(${range}\\s+(?:${units})\\s+(?:plus|minus)\\s+${range}\\s+(?:${units}))\\s+(.+)$`, "iu");
  const packaged = new RegExp(`^(${range})\\s+(\\(${amount}[- ](?:ounces?|oz|grams?|g|pounds?|lbs?)\\)\\s+(?:cans?|packages?|boxes?${simplyRecipes ? "|jars?|blocks?" : ""}))\\s+(.+)$`, "iu");
  const measured = new RegExp(`^(${range})\\s+(${units})\\s+(.+)$`, "iu");
  const ingredientChildren = directElements(ingredientContainer.innerHtml);
  if (hasLooseText(ingredientContainer.innerHtml, ingredientChildren)) return null;
  let sawHeading = false, pendingName = null, awaitingList = false;
  for (const child of ingredientChildren) {
    const offset = ingredientContainer.openingEnd;
    if (isHidden(child, offset)) return null;
    if (hasClass(child.attributes, "mm-recipes-screen-wake") && !sawHeading) {
      if (simplyRecipes && (text(child.innerHtml) !== "Keep Screen Awake" || /<(?:p|ul|ol|li|h[1-6])\b/iu.test(child.innerHtml))) return null;
      continue;
    }
    const headings = elementsWithClass(child.innerHtml, "mm-recipes-structured-ingredients__heading");
    if (!sawHeading && headings.length === 1 && child.tag === "div") {
      const heading = headings[0];
      if (heading.tag !== "h2" || text(heading.innerHtml) !== "Ingredients" || isHidden(heading, offset + child.openingEnd, true) || text(child.innerHtml) !== "Ingredients") return null;
      consumedCore.add(offset + child.openingEnd + heading.index); sawHeading = true; continue;
    }
    if (!sawHeading) return null;
    if (hasClass(child.attributes, "mm-recipes-structured-ingredients__list-heading")) {
      if (awaitingList || child.tag !== "p" || !text(child.innerHtml) || isHidden(child, offset, true)) return null;
      consumedCore.add(offset + child.index); pendingName = text(child.innerHtml).replace(/:\s*$/u, ""); awaitingList = true; continue;
    }
    if (child.tag !== "ul" || !hasClass(child.attributes, "mm-recipes-structured-ingredients__list") || (result.fields.ingredientSections.length && !awaitingList)) return null;
    consumedCore.add(offset + child.index);
    const rows = directElements(child.innerHtml), rowOffset = offset + child.openingEnd;
    if (!rows.length || hasLooseText(child.innerHtml, rows)) return null;
    const items = [];
    for (const [index, row] of rows.entries()) {
      const paragraphs = directElements(row.innerHtml);
      if (row.tag !== "li" || !hasClass(row.attributes, "mm-recipes-structured-ingredients__list-item") || isHidden(row, rowOffset, true) || paragraphs.length !== 1 || paragraphs[0].tag !== "p" || !text(paragraphs[0].innerHtml) || hasLooseText(row.innerHtml, paragraphs) || /<(?:ul|ol|li|div|p|h[1-6])\b/iu.test(paragraphs[0].innerHtml)) return null;
      consumedCore.add(rowOffset + row.index);
      // Quantity/name annotations repeat inside conversions and alternatives.
      // Split the complete visible row only at its leading measurement; retain
      // all later conversion, preparation and alternative text literally.
      const raw = text(paragraphs[0].innerHtml), locator = `.mm-recipes-structured-ingredients group ${result.fields.ingredientSections.length + 1} li:nth-child(${index + 1})`;
      const expression = raw.match(compound), packageAmount = raw.match(packaged), measure = raw.match(measured);
      items.push(expression ? { raw, amount: expression[1], unit: null, name: expression[2], confidence: "high", provenance: provenance("text/html", locator) }
        : packageAmount ? { raw, amount: packageAmount[1], unit: packageAmount[2], name: packageAmount[3], confidence: "high", provenance: provenance("text/html", locator) }
          : measure ? { raw, amount: measure[1], unit: measure[2], name: measure[3], confidence: "high", provenance: provenance("text/html", locator) }
            : ingredient(raw, "text/html", locator, range));
    }
    result.fields.ingredientSections.push({ name: pendingName, items }); pendingName = null; awaitingList = false;
  }
  if (!sawHeading || !result.fields.ingredientSections.length || awaitingList) return null;
  const procedureChildren = directElements(procedure.innerHtml);
  if (procedureChildren.length !== 2 || procedureChildren[0].tag !== "h2" || !hasClass(procedureChildren[0].attributes, "mm-recipes-steps__heading") || text(procedureChildren[0].innerHtml) !== "Directions" || isHidden(procedureChildren[0], procedure.openingEnd, true) || procedure.openingEnd + procedureChildren[1].index !== content.index || hasLooseText(procedure.innerHtml, procedureChildren)) return null;
  consumedCore.add(procedure.openingEnd + procedureChildren[0].index);
  const contentChildren = directElements(content.innerHtml), list = contentChildren[0];
  if (!list || list.tag !== "ol" || !hasClass(list.attributes, "mntl-sc-block-group--OL") || hasLooseText(content.innerHtml, contentChildren) || isHidden(list, content.openingEnd)) return null;
  consumedCore.add(content.openingEnd + list.index);
  const rows = directElements(list.innerHtml), rowOffset = content.openingEnd + list.openingEnd;
  if (!rows.length || hasLooseText(list.innerHtml, rows)) return null;
  const supplementMode = label => /^(?:Special Equipment|Equipment)$/u.test(label) ? "equipment" : label === "Notes" || /^(?:Make[-– ]Ahead|Makes-Ahead)(?: and | & )Storage$/u.test(label) ? "notes" : label === "Read More" ? "related" : null;
  let currentPhase = null, flattenedNested = 0;
  const procedureExtraClasses = ["mntl-sc-block-subheading", "mntl-sc-block-universal-callout", "mntl-sc-block-universal-callout__heading", "mntl-sc-block-universal-callout__body"];
  const procedureExtras = simplyRecipes ? elementsMatchingAttributes(procedure.innerHtml, attributes => procedureExtraClasses.some(name => hasClass(attributes, name))) : [];
  const consumedExtras = new Set();
  if (simplyRecipes && procedureExtraClasses.some(name => openingHtmlElements(procedure.innerHtml).filter(element => hasClass(element.attributes, name)).length !== elementsWithClass(procedure.innerHtml, name).length)) return null;
  const consumeExtra = (element, offset) => consumedExtras.add(offset + element.index - procedure.openingEnd);
  const readCallout = (block, offset, isStep) => {
    const children = directElements(block.innerHtml), headings = children.filter(child => hasClass(child.attributes, "mntl-sc-block-universal-callout__heading")), bodies = children.filter(child => hasClass(child.attributes, "mntl-sc-block-universal-callout__body"));
    if (block.tag !== "div" || headings.length > 1 || bodies.length !== 1 || children.length !== headings.length + 1 || hasLooseText(block.innerHtml, children) || isHidden(block, offset, true)) return null;
    const label = headings.length ? text(headings[0].innerHtml) : null;
    if (headings.some(heading => heading.tag !== "h3") || (isStep ? label && !["Tip", "Tips", "Simply Recipes / Tip"].includes(label) : label !== "Recipe Notes")) return null;
    const body = bodies[0], parts = directElements(body.innerHtml), texts = [];
    if (body.tag !== "div" || hasLooseText(body.innerHtml, parts) || !parts.length) return null;
    for (const part of parts) {
      if (part.tag === "p" && !/<(?:p|div|ul|ol|li|h[1-6]|figure|table)\b/iu.test(part.innerHtml) && text(part.innerHtml)) texts.push(text(part.innerHtml));
      else if (["ul", "ol"].includes(part.tag)) {
        const entries = directElements(part.innerHtml);
        if (!entries.length || hasLooseText(part.innerHtml, entries)) return null;
        for (const entry of entries) {
          if (entry.tag !== "li" || /<(?:p|div|ul|ol|li|h[1-6]|figure|table)\b/iu.test(entry.innerHtml) || !text(entry.innerHtml)) return null;
          texts.push(text(entry.innerHtml));
        }
      } else return null;
    }
    consumeExtra(block, offset);
    for (const child of children) consumeExtra(child, offset + block.openingEnd);
    return { label, texts };
  };
  // The same explicit supplement headings can appear after the OL or, in one
  // copied layout, within the last LI. Handle both without adding them to steps.
  const readBlocks = (html, offset, isStep) => {
    const blocks = directElements(html), texts = []; let mode = null, headingLabel = null, phase = null, stepLabel = null;
    if (hasLooseText(html, blocks)) return null;
    for (const block of blocks) {
      if (hasClass(block.attributes, "heading-toc") && !text(block.innerHtml)) continue;
      if (hasClass(block.attributes, "mntl-sc-block-adslot") || block.tag === "figure" || hasClass(block.attributes, "mntl-sc-block-universal-callout--originalpublishdate")) {
        if (simplyRecipes && hasClass(block.attributes, "mntl-sc-block-adslot") && text(block.innerHtml)) return null;
        for (const match of block.innerHtml.matchAll(/<[a-z][a-z0-9:-]*\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu)) {
          const attributes = htmlAttributes(match[1]);
          if (hasClass(attributes, "mntl-sc-block-html") || hasClass(attributes, "mntl-sc-block-heading") || (simplyRecipes && procedureExtraClasses.some(name => hasClass(attributes, name)))) return null;
        }
        continue;
      }
      if (isHidden(block, offset, true)) return null;
      if (simplyRecipes && hasClass(block.attributes, "mntl-sc-block-subheading")) {
        if (!isStep || mode || stepLabel || texts.length || block.tag !== "h3" || /<(?:p|div|ul|ol|li|h[1-6])\b/iu.test(block.innerHtml) || !text(block.innerHtml).replace(/:\s*$/u, "")) return null;
        stepLabel = text(block.innerHtml).replace(/:\s*$/u, ""); consumeExtra(block, offset); continue;
      }
      if (simplyRecipes && hasClass(block.attributes, "mntl-sc-block-universal-callout")) {
        const callout = readCallout(block, offset, isStep && !mode);
        if (!callout) return null;
        if (isStep && !mode) texts.push(`${callout.label ? `${callout.label}: ` : ""}${callout.texts.join(" ")}`);
        else for (const value of callout.texts) result.fields.notes.push(scalar(`${callout.label}: ${value}`, "text/html", ".mm-recipes-steps__content Recipe Notes"));
        continue;
      }
      if (block.tag === "h2" && hasClass(block.attributes, "mntl-sc-block-heading")) {
        const label = text(block.innerHtml), selectedMode = supplementMode(label);
        if (!label) return null;
        if (selectedMode) { mode = selectedMode; headingLabel = label; continue; }
        if (!isStep || texts.length || mode || phase) return null;
        phase = label; continue;
      }
      if (!hasClass(block.attributes, "mntl-sc-block-html")) return null;
      if (mode === "related" && block.tag === "ul") continue;
      if (["equipment", "notes"].includes(mode) && block.tag === "ul") {
        const entries = directElements(block.innerHtml);
        if (!entries.length || hasLooseText(block.innerHtml, entries)) return null;
        for (const entry of entries) {
          if (entry.tag !== "li" || isHidden(entry, offset + block.openingEnd, true) || /<(?:ul|ol|li|div|p|h[1-6])\b/iu.test(entry.innerHtml) || !text(entry.innerHtml)) return null;
          const value = text(entry.innerHtml), locator = `.mm-recipes-steps__content ${headingLabel} li`;
          if (mode === "equipment") result.fields.equipment.push(scalar(value, "text/html", locator));
          else result.fields.notes.push(scalar(`${headingLabel}: ${value}`, "text/html", locator));
        }
        continue;
      }
      let retained;
      if (block.tag === "p") {
        if (/<(?:ul|ol|li|div|p|h[1-6])\b/iu.test(block.innerHtml)) return null;
        retained = text(block.innerHtml);
      } else if (isStep && !mode && block.tag === "ol") {
        const children = directElements(block.innerHtml);
        if (children.length !== 1 || children[0].tag !== "li" || hasLooseText(block.innerHtml, children) || /<(?:ul|ol|li|div|p|h[1-6])\b/iu.test(children[0].innerHtml)) return null;
        retained = text(children[0].innerHtml); flattenedNested += 1;
      } else return null;
      if (!retained) return null;
      if (simplyRecipes && isStep && !mode && retained === "Love the recipe? Leave us stars and a comment below!") continue;
      const locator = `.mm-recipes-steps__content ${headingLabel ?? "procedure"}`;
      if (mode === "equipment") result.fields.equipment.push(scalar(retained, "text/html", locator));
      else if (mode === "notes") result.fields.notes.push(scalar(`${headingLabel}: ${retained}`, "text/html", locator));
      else if (mode) return null;
      else if (isStep) texts.push(retained);
      else return null;
    }
    if (stepLabel) {
      if (texts.length) texts[0] = `${stepLabel}${/[.!?]$/u.test(stepLabel) ? "" : ":"} ${texts[0]}`;
      else texts.push(stepLabel);
    }
    return { texts, phase };
  };
  for (const [index, row] of rows.entries()) {
    if (row.tag !== "li" || !hasClass(row.attributes, "mntl-sc-block-group--LI") || isHidden(row, rowOffset)) return null;
    consumedCore.add(rowOffset + row.index);
    const extracted = readBlocks(row.innerHtml, rowOffset + row.openingEnd, true);
    if (!extracted?.texts.length) return null;
    if (extracted.phase) currentPhase = extracted.phase;
    if (!result.fields.instructionSections.length || extracted.phase) result.fields.instructionSections.push({ name: currentPhase, steps: [] });
    result.fields.instructionSections.at(-1).steps.push(instruction(extracted.texts.join(" "), "text/html", `.mm-recipes-steps__content ol > li:nth-child(${index + 1})`));
  }
  const tail = content.innerHtml.slice(list.end);
  if (!readBlocks(tail, content.openingEnd + list.end, false)) return null;
  // Required markers embedded in ignored ads, figures or controls must not
  // disappear merely because the usual containers still look recognizable.
  if (consumedCore.size !== markedCore.length || markedCore.some(element => !consumedCore.has(element.index))) return null;
  if (simplyRecipes && (consumedExtras.size !== procedureExtras.length || procedureExtras.some(element => !consumedExtras.has(element.index)))) return null;
  const descriptions = elementsWithClass(header.innerHtml, "article-subheading");
  if (descriptions.length > 1 || descriptions.some(element => isHidden(element, header.openingEnd, true))) return null;
  if (descriptions.length && text(descriptions[0].innerHtml)) result.fields.description = scalar(text(descriptions[0].innerHtml), "text/html", ".mm-recipes-article-header .article-subheading");
  const authorGroups = elementsWithClass(header.innerHtml, "mntl-bylines__group--author");
  const authors = [];
  for (const group of authorGroups) {
    if (isHidden(group, header.openingEnd)) return null;
    for (const author of elementsWithClass(group.innerHtml, "mntl-attribution__item-name")) {
      if (isHidden(author, header.openingEnd + group.openingEnd, true)) return null;
      if (text(author.innerHtml) && !authors.includes(text(author.innerHtml))) authors.push(text(author.innerHtml));
    }
  }
  if (authors.length) { result.source.author = authors.join("; "); result.source.provenance.author = provenance("text/html", ".mntl-bylines__group--author .mntl-attribution__item-name"); }
  const details = elementsWithClass(inertHtml, "mm-recipes-details");
  if (details.length > 1 || details.some(element => !inside(element, article) || element.index < header.end || element.end > ingredientContainer.index || isHidden(element))) return null;
  const metadata = new Map();
  if (simplyRecipes) {
    const cardTitles = elementsWithClass(inertHtml, "mm-recipes-details__title");
    if (cardTitles.length > 1 || cardTitles.some(cardTitle => !details.length || !inside(cardTitle, details[0]) || cardTitle.tag !== "h2" || /<(?:p|div|ul|ol|li|h[1-6]|figure|table)\b/iu.test(cardTitle.innerHtml) || !text(cardTitle.innerHtml) || isHidden(cardTitle, 0, true))) return null;
    if (cardTitles.length) {
      const cardTitle = cardTitles[0];
      if (text(cardTitle.innerHtml) !== result.fields.title.value) {
        result.competingCandidates.push({ reason: "Article title differs from the explicit recipe card title", title: result.fields.title.value, provenance: result.fields.title.provenance });
        result.warnings.push({ code: "COMPETING_RECIPE_REGION", severity: "warning", path: "competingCandidates[0]", message: "Confirm the explicit recipe card title; it differs from the surrounding article title." });
      }
      result.fields.title = scalar(text(cardTitle.innerHtml), "text/html", ".mm-recipes-details__title");
    }
    const detailContents = elementsWithClass(inertHtml, "mm-recipes-details__content");
    if (detailContents.length !== details.length || detailContents.some(body => !inside(body, details[0]))) return null;
    if (details.length) {
      const children = directElements(details[0].innerHtml), body = detailContents[0], entries = directElements(body.innerHtml);
      if (hasLooseText(details[0].innerHtml, children) || hasLooseText(body.innerHtml, entries) || entries.some(entry => !hasClass(entry.attributes, "mm-recipes-details__item")) || children.some(child => !["mm-recipes-details__title", "mm-recipes-details__content", "mm-recipes-details__note", "mm-recipes-details__nutrition-link-container"].some(name => hasClass(child.attributes, name)))) return null;
      // Complete markers are not enough: every card body, title and note must
      // be a direct owned child, and every item/label/value must be consumed.
      for (const name of ["mm-recipes-details__title", "mm-recipes-details__content", "mm-recipes-details__note", "mm-recipes-details__nutrition-link-container"]) {
        const owned = children.filter(child => hasClass(child.attributes, name));
        if (elementsWithClass(inertHtml, name).some(element => !owned.some(child => details[0].openingEnd + child.index === element.index))) return null;
      }
      for (const name of ["mm-recipes-details__item", "mm-recipes-details__label", "mm-recipes-details__value"]) {
        if (elementsWithClass(inertHtml, name).some(element => !entries.some(entry => name === "mm-recipes-details__item" ? body.openingEnd + entry.index === element.index : inside(element, { ...entry, openingEnd: body.openingEnd + entry.openingEnd, contentEnd: body.openingEnd + entry.contentEnd })))) return null;
      }
      const notes = elementsWithClass(inertHtml, "mm-recipes-details__note");
      if (elementsWithClass(inertHtml, "mm-recipes-details__note-label").some(label => !notes.some(note => inside(label, note)))) return null;
      for (const control of children.filter(child => hasClass(child.attributes, "mm-recipes-details__nutrition-link-container"))) {
        const controls = directElements(control.innerHtml);
        if (controls.length !== 1 || controls[0].tag !== "button" || text(control.innerHtml) !== "Jump to Nutrition Facts" || /<(?:p|ul|ol|li|h[1-6])\b/iu.test(controls[0].innerHtml) || hasLooseText(control.innerHtml, controls)) return null;
      }
    } else if (["mm-recipes-details__item", "mm-recipes-details__label", "mm-recipes-details__value", "mm-recipes-details__note", "mm-recipes-details__note-label"].some(name => elementsWithClass(inertHtml, name).length)) {
      return null;
    }
  }
  if (details.length) {
    const items = elementsWithClass(details[0].innerHtml, "mm-recipes-details__item");
    for (const item of items) {
      const labels = elementsWithClass(item.innerHtml, "mm-recipes-details__label"), values = elementsWithClass(item.innerHtml, "mm-recipes-details__value");
      const itemOffset = details[0].openingEnd + item.openingEnd;
      if (labels.length !== 1 || values.length !== 1 || isHidden(item, details[0].openingEnd) || [...labels, ...values].some(element => isHidden(element, itemOffset, true))) return null;
      const label = text(labels[0].innerHtml).replace(/:+\s*$/u, ""), value = text(values[0].innerHtml), raw = `${label}: ${value}`;
      if (!label || !value || metadata.has(label) || text(item.innerHtml) !== text(labels[0].innerHtml) + " " + value) return null;
      metadata.set(label, value);
      const field = { "Prep Time": "prepTime", "Cook Time": "cookTime", "Total Time": "totalTime", "Total": "totalTime" }[label];
      if (field) {
        if (result.fields[field]) {
          // Distinct total labels can disagree in a copied card. Keep both as
          // literal evidence without arbitrarily selecting one numeric value.
          result.fields[field] = { ...result.fields[field], raw: `${result.fields[field].raw}; ${raw}`, value: null, confidence: "low" };
          result.warnings.push({ code: "AMBIGUOUS_PROVIDER_DURATION", severity: "warning", path: `fields.${field}`, message: "Multiple duration labels map to this field; review the retained literal values." });
          continue;
        }
        const token = "\\d+(?:\\.\\d+)?\\s*(?:days?|hours?|hrs?|hr|minutes?|mins?|min)";
        const exactGrammar = new RegExp(`^${token}(?:(?:\\s+|\\s+and\\s+|,\\s*(?:and\\s+)?)${token})*$`, "iu").test(value);
        const duration = labeledDuration(`${field === "totalTime" ? "Total Time" : label}: ${value}`);
        const minutes = exactGrammar && duration && Number.isSafeInteger(duration.value) && duration.value >= 0 ? duration.value : null;
        result.fields[field] = { ...scalar(raw, "text/html", `.mm-recipes-details (${label})`), value: minutes, confidence: minutes === null ? "low" : "high" };
        if (minutes === null) result.warnings.push({ code: "UNRESOLVED_PROVIDER_DURATION", severity: "warning", path: `fields.${field}`, message: "Qualified or unsupported duration text remains literal; exact minutes are not inferred." });
      } else if (!["Servings", "Yield"].includes(label)) result.unclassifiedLines.push({ raw, provenance: provenance("text/html", `.mm-recipes-details (${label})`) });
    }
    for (const note of elementsWithClass(details[0].innerHtml, "mm-recipes-details__note")) {
      if (isHidden(note, details[0].openingEnd, true)) return null;
      if (simplyRecipes) {
        const children = directElements(note.innerHtml), labels = children.filter(child => hasClass(child.attributes, "mm-recipes-details__note-label")), paragraphs = children.filter(child => child.tag === "p");
        if (labels.length !== 1 || text(labels[0].innerHtml) !== "Note:" || children.length !== paragraphs.length + 1 || hasLooseText(note.innerHtml, children) || paragraphs.some(paragraph => /<(?:p|div|ul|ol|li|h[1-6])\b/iu.test(paragraph.innerHtml))) return null;
        for (const paragraph of paragraphs) if (text(paragraph.innerHtml)) result.fields.notes.push(scalar(`Note: ${text(paragraph.innerHtml)}`, "text/html", ".mm-recipes-details__note"));
      } else if (text(note.innerHtml)) result.unclassifiedLines.push({ raw: text(note.innerHtml), provenance: provenance("text/html", ".mm-recipes-details__note") });
    }
  }
  if (metadata.has("Servings") || metadata.has("Yield")) {
    const display = [metadata.has("Servings") ? `Servings: ${metadata.get("Servings")}` : null, metadata.has("Yield") ? `Yield: ${metadata.get("Yield")}` : null].filter(Boolean).join("; ");
    const count = metadata.get("Servings")?.match(/^(\d+)(?:\s+servings?)?$/iu);
    const servings = count && Number.isSafeInteger(Number(count[1])) && Number(count[1]) > 0 ? Number(count[1]) : null;
    result.fields.yield = { ...scalar(display, "text/html", ".mm-recipes-details (Servings, Yield)"), value: { display, servings } };
  }
  const introductions = elementsWithClass(article.innerHtml, "mm-recipes-intro__content");
  if (introductions.length === 1 && !isHidden(introductions[0], article.openingEnd)) {
    const introduction = introductions[0], offset = article.openingEnd + introduction.openingEnd;
    const figures = elementsMatchingAttributes(introduction.innerHtml, (_, tag) => tag === "figure");
    // The first visible intro figure is the recipe lead photo in saved copies;
    // later tutorial/review imagery is not an inferred primary image.
    const firstFigure = figures.find(figure => !isHidden(figure, offset, true));
    if (firstFigure) {
      let imageHtml = introduction.innerHtml.slice(firstFigure.index, firstFigure.end);
      // Void image nodes have no inner text, so the figure text guard cannot
      // detect them. Remove hidden images before collecting remote references.
      const imageOffset = offset + firstFigure.index;
      const hiddenImages = [...imageHtml.matchAll(/<img\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu)].filter(match => isHidden({ index: match.index, end: match.index + match[0].length }, imageOffset));
      for (const match of hiddenImages.reverse()) imageHtml = imageHtml.slice(0, match.index) + imageHtml.slice(match.index + match[0].length);
      result.fields.images = imageReferences(imageHtml, rootRanges(imageHtml), null, "data-src").images;
    }
  }
  if (flattenedNested) result.warnings.push({ code: "NESTED_SINGLE_STEP_FLATTENED", severity: "warning", path: "fields.instructionSections", message: "A single-item nested procedure remains at its enclosing step position. Review its presentation." });
  if ([...result.fields.ingredientSections, ...result.fields.instructionSections].some(section => section.name)) result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields", message: "Explicit group names remain in preview and optional commentary; the editor stores flat content." });
  if (result.unclassifiedLines.length) result.warnings.push({ code: "UNCLASSIFIED_PROVIDER_METADATA", severity: "warning", path: "unclassifiedLines", message: "Additional recipe metadata remains available for review, not silently mapped." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "Image references are unfetched and require confirmation before mapping." })));
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

// New visible-card adapters share ownership/completeness checks, not guessing
// rules. A marked node must be complete, visible, inside the unique root and
// explicitly consumed by the provider's measured structure.
function visibleProviderContext(html, rootClass, markerClasses) {
  const openings = openingHtmlElements(html);
  const selected = elementsMatchingAttributes(html, attributes => markerClasses.some(name => hasClass(attributes, name)));
  const roots = elementsWithClass(html, rootClass);
  if (roots.length !== 1 || openings.filter(e => hasClass(e.attributes, rootClass)).length !== 1 || openings.filter(e => markerClasses.some(name => hasClass(e.attributes, name))).length !== selected.length) return null;
  const root = roots[0], inside = (e, parent = root) => parent.openingEnd <= e.index && e.end <= parent.contentEnd;
  const hidden = elementsMatchingAttributes(html, a => listIsHidden({attributes:a}) || hasClass(a, "hidden") || hasClass(a, "is-hidden"));
  for (const e of openings.filter(e => listIsHidden(e) || hasClass(e.attributes, "hidden") || hasClass(e.attributes, "is-hidden"))) {
    if (!hidden.some(range => range.index === e.index)) hidden.push({...e, end:html.length, innerHtml:html.slice(e.openingEnd)});
  }
  const isHidden = (e, includeText = true) => hidden.some(range => (range.index <= e.index && e.index < range.end) || (includeText && e.index <= range.index && range.index < e.end && htmlText(range.innerHtml)));
  if (hasAncestorTag(html, root.index, "nav") || isHidden(root, false) || selected.some(e => e.index !== root.index && (!inside(e) || isHidden(e) || hasAncestorTag(html,e.index,"nav")))) return null;
  const consumed = new Set([root.index]);
  const all = name => selected.filter(e => hasClass(e.attributes, name));
  const children = e => elementsMatchingAttributes(e.innerHtml, () => true).filter(child => tagStackContext(e.innerHtml, child.index).length === 0).map(child => ({...child,index:e.openingEnd+child.index,openingEnd:e.openingEnd+child.openingEnd,contentEnd:e.openingEnd+child.contentEnd,end:e.openingEnd+child.end}));
  const loose = (e, parts = children(e)) => {let end=e.openingEnd;for(const part of parts){if(htmlText(html.slice(end,part.index)))return true;end=part.end;}return Boolean(htmlText(html.slice(end,e.contentEnd)));};
  const text = fragment => htmlText(fragment.replace(/<\/?(?:span|strong|em|b|i|a|sup|sub)\b(?:"[^"]*"|'[^']*'|[^>"'])*>/giu,""));
  const inline = e => !/<(?:p|div|section|ul|ol|li|h[1-6]|figure|table)\b/iu.test(e.innerHtml) && Boolean(text(e.innerHtml));
  return {root,all,inside,children,loose,text,inline,isHidden,consume:e=>consumed.add(e.index),complete:()=>selected.every(e=>consumed.has(e.index))};
}

function visibleProviderIngredient(raw, locator) {
  const amount = `(?:\\d+\\s+and\\s+\\d+/\\d+|\\d+[\\u00bc-\\u00be\\u2150-\\u215e]|${TASTY_AMOUNT_PATTERN})`;
  const range = `${amount}(?:\\s*(?:to|[-–—])\\s*${amount})?`;
  const units = `${UNIT_PATTERN}|pinch|quarts?|pints?|sticks?|bunch(?:es)?|heads?|jars?|blocks?`;
  const compound = raw.match(new RegExp(`^(${range}\\s+(?:${units})\\s+(?:plus|minus)\\s+${range}\\s+(?:${units}))\\s+(.+)$`,"iu"));
  const packaged = raw.match(new RegExp(`^(${range})\\s+(\\(${amount}[- ](?:ounces?|oz|grams?|g|pounds?|lbs?)\\)\\s+(?:cans?|packages?|boxes?|jars?|blocks?))\\s+(.+)$`,"iu"));
  const measured = raw.match(new RegExp(`^(${range})\\s+(${units})\\s+(.+)$`,"iu"));
  const counted = raw.match(new RegExp(`^(${range})\\s+(.+)$`,"iu"));
  return {...ingredient(raw,"text/html",locator), amount:compound?.[1]??packaged?.[1]??measured?.[1]??counted?.[1]??null,unit:compound?null:packaged?.[2]??measured?.[2]??null,name:compound?.[2]??packaged?.[3]??measured?.[3]??counted?.[2]??raw};
}

function visibleProviderDuration(result, label, value, locator) {
  const field = {Prep:"prepTime","Prep Time":"prepTime",Cook:"cookTime","Cook Time":"cookTime",Total:"totalTime","Total Time":"totalTime"}[label];
  const raw = `${label}: ${value}`;
  if (!field) {result.unclassifiedLines.push({raw,provenance:provenance("text/html",locator)});return;}
  const token = "\\d+(?:\\.\\d+)?\\s*(?:days?|hours?|hrs?|hr|minutes?|mins?|min)";
  const exact = new RegExp(`^${token}(?:(?:\\s+|\\s+and\\s+|,\\s*(?:and\\s+)?)${token})*$`,"iu").test(value);
  const minutes = exact ? labeledDuration(raw)?.value??null : null;
  result.fields[field] = {...scalar(raw,"text/html",locator),value:minutes,confidence:minutes===null?"low":"high"};
  if(minutes===null)result.warnings.push({code:"UNRESOLVED_PROVIDER_DURATION",severity:"warning",path:`fields.${field}`,message:"Qualified duration remains literal; exact minutes are not inferred."});
}

function finishVisibleProvider(result, htmlAudit) {
  if ([...result.fields.ingredientSections,...result.fields.instructionSections].some(s=>s.name))result.warnings.push({code:"SECTION_NAMES_NOT_PERSISTED",severity:"warning",path:"fields",message:"Group names remain in preview and optional commentary; the editor stores flat content."});
  if(result.unclassifiedLines.length)result.warnings.push({code:"UNCLASSIFIED_PROVIDER_METADATA",severity:"warning",path:"unclassifiedLines",message:"Additional metadata remains literal for review."});
  result.warnings.push(...result.fields.images.map((_,index)=>({code:"IMAGE_REFERENCE_REQUIRES_CONFIRMATION",severity:"info",path:`fields.images[${index}]`,message:"Image references are unfetched and require confirmation before mapping."})));
  if(result.warnings.length)result.status="needs_review";
  appendHtmlAuditEvidence(result,htmlAudit,discardedRemoteResourceCount(htmlAudit,result.fields.images));
  return result;
}

function parseFoodNetworkVisibleRecipe(html, sourceUrl, htmlAudit) {
  try {const url=new URL(sourceUrl);if(!["http:","https:"].includes(url.protocol)||!["foodnetwork.com","www.foodnetwork.com"].includes(url.hostname))return null;}catch{return null;}
  const markerClasses=["o-Recipe","o-AssetTitle__a-Headline","o-Ingredients","o-Ingredients__m-Body","o-Ingredients__a-Ingredient","o-Ingredients__a-Ingredient--CheckboxLabel","o-Ingredients__a-SubHeadline","o-Method","o-Method__m-Body","o-Method__m-Step","o-Method__a-SubHeadline","o-Method__m-Yield","o-Method__a-Description","o-RecipeInfo","o-RecipeInfo__a-Headline","o-RecipeInfo__a-Description","o-RecipeInfo__a-Note","o-AssetDescription__a-Description","o-Attribution__m-Author","o-ChefNotes","o-ChefNotes__a-Description","o-Warnings","o-Warnings__a-Description"];
  const c=visibleProviderContext(html,"o-Recipe",markerClasses);
  if(!c||c.root.tag!=="section")return null;
  const {all,inside,children,loose,text,inline,consume}=c;
  const containers=["o-Ingredients","o-Ingredients__m-Body","o-Method","o-Method__m-Body"].map(all);
  if(containers.some(es=>es.length!==1))return null;
  const [[ingredients],[ingredientBody],[method],[methodBody]]=containers;
  if(!inside(ingredientBody,ingredients)||!inside(methodBody,method)||ingredients.end>method.index)return null;
  for(const e of [ingredients,ingredientBody,method,methodBody])consume(e);
  const titles=all("o-AssetTitle__a-Headline");
  const cardTitles=titles.filter(e=>e.tag==="h2");
  const extended=all("o-RecipeInfo").filter(e=>hasClass(e.attributes,"o-RecipeInfo--extendedTemplate"));
  // One measured extended article layout uses editorial H1 + recipe-card H2.
  // Prefer the owned H2 only with that template; expose H1 instead of losing it.
  if(!titles.length||titles.some(e=>!["h1","h2"].includes(e.tag)||!inline(e)||e.end>ingredients.index)||
    (cardTitles.length ? cardTitles.length!==1||extended.length!==1||titles.filter(e=>e.tag==="h1").length!==1||elementsWithClass(html,"recipeHead").filter(e=>inside(e)&&inside(cardTitles[0],e)).length!==1 : new Set(titles.map(e=>text(e.innerHtml))).size!==1))return null;
  for(const e of titles)consume(e);
  const result=candidate({id:"foodnetwork-visible",version:"1",tier:"source_adapter",sourceAdapter:"foodnetwork"},sourceUrl);
  result.source.provider="Food Network";result.source.provenance.provider=provenance("source_url","hostname foodnetwork.com");
  const selectedTitle=cardTitles[0]??titles[0];
  result.fields.title=scalar(text(selectedTitle.innerHtml),"text/html",".o-AssetTitle__a-Headline");
  if(cardTitles.length&&text(titles.find(e=>e.tag==="h1").innerHtml)!==result.fields.title.value){result.competingCandidates.push({reason:"Editorial title differs from the explicit card title",title:text(titles.find(e=>e.tag==="h1").innerHtml),provenance:provenance("text/html","h1.o-AssetTitle__a-Headline")});result.warnings.push({code:"COMPETING_RECIPE_REGION",severity:"warning",path:"competingCandidates[0]",message:"Confirm the explicit recipe card title against the editorial heading."});}
  const parts=children(ingredientBody);if(loose(ingredientBody,parts))return null;
  let group=null,awaiting=false;
  for(const part of parts){
    if(hasClass(part.attributes,"o-Ingredients__a-SubHeadline")){if(awaiting||part.tag!=="h3"||!inline(part))return null;consume(part);group=text(part.innerHtml).replace(/:\s*$/u,"");awaiting=true;continue;}
    if(!hasClass(part.attributes,"o-Ingredients__a-Ingredient")||part.tag!=="p")return null;
    const labels=all("o-Ingredients__a-Ingredient--CheckboxLabel").filter(e=>inside(e,part));
    if(labels.length!==1||!inline(labels[0])||text(part.innerHtml)!==text(labels[0].innerHtml))return null;
    consume(part);consume(labels[0]);
    if(hasClass(part.attributes,"o-Ingredients__a-Ingredient--SelectAll")){if(!["Select All","Deselect All"].includes(text(part.innerHtml)))return null;continue;}
    if(!result.fields.ingredientSections.length||awaiting)result.fields.ingredientSections.push({name:group,items:[]});
    result.fields.ingredientSections.at(-1).items.push(visibleProviderIngredient(text(labels[0].innerHtml),".o-Ingredients__a-Ingredient--CheckboxLabel"));awaiting=false;
  }
  if(awaiting||!result.fields.ingredientSections.length)return null;
  const blocks=children(methodBody);if(loose(methodBody,blocks))return null;
  group=null;awaiting=false;
  for(const block of blocks){
    if(block.tag==="h3"&&hasClass(block.attributes,"o-Method__a-SubHeadline")){if(awaiting||!inline(block))return null;consume(block);group=text(block.innerHtml).replace(/:\s*$/u,"");awaiting=true;continue;}
    if(hasClass(block.attributes,"o-Method__m-Yield")){
      const labels=all("o-Method__a-SubHeadline").filter(e=>inside(e,block)),values=all("o-Method__a-Description").filter(e=>inside(e,block));
      if(block.tag!=="div"||labels.length!==1||values.length!==1||!inline(labels[0])||!inline(values[0])||text(labels[0].innerHtml)!=="Yield:"||htmlText(block.innerHtml)!==`${text(labels[0].innerHtml)} ${text(values[0].innerHtml)}`)return null;
      for(const e of [block,...labels,...values])consume(e);
      result.unclassifiedLines.push({raw:`${group?`${group} — `:""}Yield: ${text(values[0].innerHtml)}`,provenance:provenance("text/html",".o-Method__m-Yield")});continue;
    }
    if(block.tag!=="ol")return null;
    const rows=children(block);if(!rows.length||loose(block,rows))return null;
    const steps=[];for(const row of rows){if(row.tag!=="li"||!hasClass(row.attributes,"o-Method__m-Step")||!inline(row))return null;consume(row);steps.push(instruction(text(row.innerHtml),"text/html",".o-Method__m-Step"));}
    result.fields.instructionSections.push({name:group,steps});awaiting=false;
  }
  if(awaiting||!result.fields.instructionSections.length)return null;
  const descriptions=all("o-AssetDescription__a-Description");if(descriptions.length>1||descriptions.some(e=>e.end>ingredients.index||!inline(e)))return null;
  if(descriptions.length){consume(descriptions[0]);result.fields.description=scalar(text(descriptions[0].innerHtml),"text/html",".o-AssetDescription__a-Description");}
  const authors=all("o-Attribution__m-Author"),authorNames=[];
  for(const author of authors){if(author.end>ingredients.index||!inline(author))return null;const name=text(author.innerHtml).replace(/^Recipe courtesy of\s+/iu,"");if(!name)return null;consume(author);authorNames.push(name);}
  if(new Set(authorNames).size>1)return null;
  if(authorNames.length){result.source.author=authorNames[0];result.source.provenance.author=provenance("text/html",".o-Attribution__m-Author");}
  const metadata=new Map();
  for(const info of all("o-RecipeInfo")){
    if(info.end>ingredients.index)return null;consume(info);
    const infoParts=[...all("o-RecipeInfo__a-Headline"),...all("o-RecipeInfo__a-Description"),...all("o-RecipeInfo__a-Note")].filter(e=>inside(e,info));
    const nutritionButtons=elementsMatchingAttributes(info.innerHtml,(_,tag)=>tag==="button").map(e=>({...e,index:info.openingEnd+e.index,openingEnd:info.openingEnd+e.openingEnd,contentEnd:info.openingEnd+e.contentEnd,end:info.openingEnd+e.end}));
    if(nutritionButtons.some(e=>!inline(e)||text(e.innerHtml)!=="Nutrition Info")||htmlText(info.innerHtml)!==[...infoParts,...nutritionButtons].sort((a,b)=>a.index-b.index).map(e=>text(e.innerHtml)).join(" "))return null;
    for(const label of all("o-RecipeInfo__a-Headline").filter(e=>inside(e,info))){
      // The containing LI or extended-template DIV owns one label/value/note.
      const owners=elementsMatchingAttributes(info.innerHtml,(_,tag)=>["li","div"].includes(tag)).map(e=>({...e,index:info.openingEnd+e.index,openingEnd:info.openingEnd+e.openingEnd,contentEnd:info.openingEnd+e.contentEnd,end:info.openingEnd+e.end})).filter(e=>inside(label,e)).sort((a,b)=>(a.end-a.index)-(b.end-b.index));
      const owner=owners[0];if(!owner)return null;
      const labels=all("o-RecipeInfo__a-Headline").filter(e=>inside(e,owner)),values=all("o-RecipeInfo__a-Description").filter(e=>inside(e,owner)),notes=all("o-RecipeInfo__a-Note").filter(e=>inside(e,owner));
      if(labels.length!==1||values.length!==1||notes.length>1||[label,...values,...notes].some(e=>!inline(e))||htmlText(owner.innerHtml)!==[label,...values,...notes].map(e=>text(e.innerHtml)).join(" "))return null;
      for(const e of [label,...values,...notes])consume(e);
      const key=text(label.innerHtml).replace(/:\s*$/u,""),value=[...values,...notes].map(e=>text(e.innerHtml)).join(" ");
      if(metadata.has(key)&&metadata.get(key)!==value)return null;metadata.set(key,value);
    }
  }
  for(const [label,value] of metadata){
    if(label==="Yield"){const count=value.match(/^(\d+)\s+servings?$/iu);const servings=count&&Number.isSafeInteger(Number(count[1]))&&Number(count[1])>0?Number(count[1]):null;result.fields.yield={...scalar(`Yield: ${value}`,"text/html",".o-RecipeInfo (Yield)"),value:{display:`Yield: ${value}`,servings}};}
    else visibleProviderDuration(result,label,value,`.o-RecipeInfo (${label})`);
  }
  for(const [containerClass,paragraphClass] of [["o-ChefNotes","o-ChefNotes__a-Description"],["o-Warnings","o-Warnings__a-Description"]]){
    for(const container of all(containerClass)){
      const paragraphs=all(paragraphClass).filter(e=>inside(e,container)),parts=children(container);
      if(!paragraphs.length||loose(container,parts)||parts.some(e=>!paragraphs.some(p=>p.index===e.index)&&!(e.tag==="header"&&children(e).length===1&&children(e)[0].tag==="h3"&&inline(children(e)[0])&&!loose(e))))return null;
      consume(container);for(const paragraph of paragraphs){if(paragraph.tag!=="p"||!inline(paragraph))return null;consume(paragraph);result.fields.notes.push(scalar(text(paragraph.innerHtml),"text/html",`.${paragraphClass}`));}
    }
  }
  const media=elementsWithClass(html,"o-RecipeLead__m-RecipeMedia").filter(e=>inside(e));
  if(media.length===1&&!c.isHidden(media[0],false)){
    const images=elementsMatchingAttributes(media[0].innerHtml,(a,tag)=>tag==="img"&&hasClass(a,"m-MediaBlock__a-Image"));
    const image=images.find(e=>!c.isHidden({...e,index:media[0].openingEnd+e.index,end:media[0].openingEnd+e.end}));
    const imageSource=image?.attributes.get("src")?.trim();
    if(imageSource){try{const url=new URL(imageSource,result.source.normalizedUrl);if(["http:","https:"].includes(url.protocol))result.fields.images.push({sourceUrl:url.href,alt:image.attributes.get("alt")||null,credit:null,provenance:provenance("text/html",".o-RecipeLead__m-RecipeMedia img")});}catch{}}
  }
  if(!c.complete())return null;
  return finishVisibleProvider(result,htmlAudit);
}

function parseNYTCookingVisibleRecipe(inertHtml, sourceUrl, htmlAudit) {
  try {
    const url = new URL(sourceUrl);
    if (!["http:", "https:"].includes(url.protocol) || url.hostname !== "cooking.nytimes.com") return null;
  } catch { return null; }
  // CSS-module hashes change between builds; match the component name, not a
  // particular hash or a loose substring that could select a related card.
  const hasModuleClass = (attributes, name) => (attributes.get("class") ?? "").split(/\s+/u).some(value => value.startsWith(`${name}__`));
  const modules = (html, name) => elementsMatchingAttributes(html, attributes => hasModuleClass(attributes, name));
  const mains = elementsMatchingAttributes(inertHtml, (_, tag) => tag === "main");
  const roots = elementsMatchingAttributes(inertHtml, attributes => hasClass(attributes, "recipe") && hasModuleClass(attributes, "pagecontent_recipe-wrap"));
  if (mains.length !== 1 || roots.length !== 1 || roots[0].tag !== "div" || roots[0].index < mains[0].openingEnd || roots[0].end > mains[0].contentEnd) return null;
  const root = roots[0];
  const coreMarkers = ["ingredient_ingredient", "ingredientgroup_name", "preparation_stepList", "preparation_step", "preparation_stepNumber", "preparation_stepContent"];
  const markedCore = elementsMatchingAttributes(inertHtml, attributes => coreMarkers.some(name => hasModuleClass(attributes, name)));
  const hiddenRanges = elementsMatchingAttributes(inertHtml, attributes => listIsHidden({ attributes }) || hasClass(attributes, "hidden") ||
    hasModuleClass(attributes, "headermediacarousel_hidden") || hasModuleClass(attributes, "notes_isHidden"));
  let mainOpenings = 0, coreOpenings = 0, rootOpenings = 0;
  for (const match of inertHtml.matchAll(/<([a-z][a-z0-9:-]*)\b((?:"[^"]*"|'[^']*'|[^>"'])*)>/giu)) {
    const attributes = htmlAttributes(match[2]);
    if (match[1].toLowerCase() === "main") mainOpenings += 1;
    if (hasClass(attributes, "recipe") && hasModuleClass(attributes, "pagecontent_recipe-wrap")) rootOpenings += 1;
    if (coreMarkers.some(name => hasModuleClass(attributes, name))) coreOpenings += 1;
    // An unclosed hidden ancestor still hides required content in the browser.
    if ((listIsHidden({ attributes }) || hasClass(attributes, "hidden")) && !hiddenRanges.some(element => element.index === match.index)) {
      hiddenRanges.push({ index: match.index, end: inertHtml.length, innerHtml: inertHtml.slice(match.index + match[0].length) });
    }
  }
  if (mainOpenings !== 1 || rootOpenings !== 1 || coreOpenings !== markedCore.length) return null;
  const consumedCore = new Set();
  const isHidden = (element, offset = root.openingEnd, includeText = false) => hiddenRanges.some(range =>
    (range.index <= offset + element.index && offset + element.index < range.end) ||
    (includeText && offset + element.index <= range.index && range.index < offset + element.end && htmlText(range.innerHtml)));
  if (isHidden(root, 0) || hasAncestorTag(inertHtml, root.index, "nav")) return null;
  const rootHtml = root.innerHtml;
  const text = html => htmlText(html).replace(/\s+([,.;:!?])/gu, "$1").replace(/\s+\)/gu, ")");
  const directElements = html => elementsMatchingAttributes(html, () => true).filter(element => tagStackContext(html, element.index).length === 0);
  const hasLooseText = (html, elements) => {
    let end = 0;
    for (const element of elements) { if (htmlText(html.slice(end, element.index))) return true; end = element.end; }
    return Boolean(htmlText(html.slice(end)));
  };
  const titles = elementsMatchingAttributes(mains[0].innerHtml, (_, tag) => tag === "h1");
  const headers = modules(rootHtml, "recipeintro_header-block");
  const ingredientContainers = modules(inertHtml, "ingredients_ingredients");
  const preparationContainers = elementsMatchingAttributes(inertHtml, attributes => attributes.get("id") === "preparation_section");
  if (titles.length !== 1 || headers.length !== 1 || ingredientContainers.length !== 1 || preparationContainers.length !== 1) return null;
  const title = titles[0], header = headers[0], ingredients = ingredientContainers[0], preparation = preparationContainers[0];
  const titleIndex = mains[0].openingEnd + title.index;
  if (!htmlText(title.innerHtml) || titleIndex < root.openingEnd + header.openingEnd || titleIndex + title.end - title.index > root.openingEnd + header.contentEnd ||
    isHidden(title, mains[0].openingEnd, true) || isHidden(header) || ingredients.index < root.openingEnd + header.end ||
    ingredients.end > preparation.index || preparation.end > root.contentEnd || isHidden(ingredients, 0) || isHidden(preparation, 0) ||
    hasAncestorTag(inertHtml, ingredients.index, "nav") || hasAncestorTag(inertHtml, preparation.index, "nav")) return null;
  if (markedCore.some(element => ![ingredients, preparation].some(container => container.openingEnd <= element.index && element.end <= container.contentEnd))) return null;
  const result = candidate({ id: "nyt-cooking-visible", version: "1", tier: "source_adapter", sourceAdapter: "nyt-cooking" }, sourceUrl);
  result.source.provider = "NYT Cooking";
  result.source.provenance.provider = provenance("source_url", "hostname cooking.nytimes.com");
  result.fields.title = scalar(htmlText(title.innerHtml), "text/html", ".recipe .recipeintro_header-block h1");
  const ingredientChildren = directElements(ingredients.innerHtml);
  if (hasLooseText(ingredients.innerHtml, ingredientChildren) || ingredientChildren[0]?.tag !== "h2" ||
    !hasModuleClass(ingredientChildren[0].attributes, "ingredients_heading") || htmlText(ingredientChildren[0].innerHtml) !== "Ingredients" ||
    isHidden(ingredientChildren[0], ingredients.openingEnd, true)) return null;
  let groupName = null, waitingForList = false, sawList = false, sawButtons = false, sawYield = false;
  const nytAmount = `(?:\\d+[\\u00bc-\\u00be\\u2150-\\u215e]|${TASTY_AMOUNT_PATTERN})`;
  const amountPattern = `${nytAmount}(?:\\s*(?:to|[-–—])\\s*${nytAmount})?`;
  const units = `${UNIT_PATTERN}|quarts?|pints?|gallons?|sticks?|bunch(?:es)?|heads?`;
  const measured = new RegExp(`^(${amountPattern})\\s+(${units})\\s+(.+)$`, "iu");
  const compound = new RegExp(`^(${amountPattern}\\s*(?:${units})\\s*(?:/|plus|minus)\\s*${amountPattern}\\s*(?:${units}))\\s+(.+)$`, "iu");
  const packaged = new RegExp(`^(${amountPattern})\\s+(\\(${TASTY_AMOUNT_PATTERN}[- ](?:ounces?|oz|pounds?|grams?)\\)\\s+(?:cans?|packages?|boxes?))\\s+(.+)$`, "iu");
  for (const child of ingredientChildren.slice(1)) {
    if (isHidden(child, ingredients.openingEnd)) return null;
    if (hasModuleClass(child.attributes, "ingredients_recipeYield")) {
      const parts = directElements(child.innerHtml);
      if (sawYield || sawList || waitingForList || sawButtons || child.tag !== "div" || parts.length !== 2 || parts.some(part => part.tag !== "span" || isHidden(part, ingredients.openingEnd + child.openingEnd, true)) ||
        hasLooseText(child.innerHtml, parts) || htmlText(parts[0].innerHtml) !== "Yield:" || !htmlText(parts[1].innerHtml)) return null;
      sawYield = true;
      const value = htmlText(parts[1].innerHtml), raw = `Yield: ${value}`;
      const count = value.match(/^(\d+)\s+servings?$/iu);
      const servings = count && Number.isSafeInteger(Number(count[1])) && Number(count[1]) > 0 ? Number(count[1]) : null;
      result.fields.yield = { ...scalar(raw, "text/html", ".recipe .ingredients_recipeYield"), value: { display: raw, servings } };
      continue;
    }
    if (child.tag === "h3" && hasModuleClass(child.attributes, "ingredientgroup_name")) {
      if (waitingForList || sawButtons || !htmlText(child.innerHtml) || isHidden(child, ingredients.openingEnd, true)) return null;
      consumedCore.add(ingredients.openingEnd + child.index);
      groupName = htmlText(child.innerHtml); waitingForList = true; continue;
    }
    if (hasModuleClass(child.attributes, "ingredients_buttons")) {
      if (!sawList || waitingForList || sawButtons || child !== ingredientChildren.at(-1) || child.tag !== "div") return null;
      sawButtons = true; continue;
    }
    if (child.tag !== "ul" || sawButtons || (sawList && !waitingForList)) return null;
    const rows = directElements(child.innerHtml);
    if (!rows.length || hasLooseText(child.innerHtml, rows)) return null;
    const items = [];
    for (const [index, row] of rows.entries()) {
      const children = directElements(row.innerHtml), rowOffset = ingredients.openingEnd + child.openingEnd;
      if (row.tag !== "li" || isHidden(row, rowOffset) || children.length !== 1 || children[0].tag !== "p" || !hasModuleClass(children[0].attributes, "ingredient_ingredient") ||
        hasLooseText(row.innerHtml, children) || isHidden(children[0], rowOffset + row.openingEnd, true) || !htmlText(children[0].innerHtml) || /<(?:ul|ol|h[1-6]|div|p)\b/iu.test(children[0].innerHtml)) return null;
      const raw = text(children[0].innerHtml), locator = `.recipe .ingredients_ingredients group ${result.fields.ingredientSections.length + 1} li:nth-child(${index + 1})`;
      // Keep conversion and plus/minus measurements as one literal expression;
      // the editor has only one unit slot and cannot safely perform conversions.
      consumedCore.add(rowOffset + row.openingEnd + children[0].index);
      const compoundAmount = raw.match(compound), packageAmount = raw.match(packaged), measuredAmount = raw.match(measured);
      const parsed = compoundAmount
        ? { raw, amount: compoundAmount[1], unit: null, name: compoundAmount[2], confidence: "high", provenance: provenance("text/html", locator) }
        : packageAmount
          ? { raw, amount: packageAmount[1], unit: packageAmount[2], name: packageAmount[3], confidence: "high", provenance: provenance("text/html", locator) }
          : measuredAmount
            ? { raw, amount: measuredAmount[1], unit: measuredAmount[2], name: measuredAmount[3], confidence: "high", provenance: provenance("text/html", locator) }
            : ingredient(raw, "text/html", locator, amountPattern);
      items.push(parsed);
    }
    result.fields.ingredientSections.push({ name: groupName, items });
    sawList = true; waitingForList = false; groupName = null;
  }
  if (!sawList || waitingForList) return null;
  const preparationChildren = directElements(preparation.innerHtml);
  if (preparationChildren.length !== 2 || preparationChildren[0].tag !== "h2" || !hasModuleClass(preparationChildren[0].attributes, "preparation_title") ||
    htmlText(preparationChildren[0].innerHtml) !== "Preparation" || isHidden(preparationChildren[0], preparation.openingEnd, true) ||
    preparationChildren[1].tag !== "ol" || !hasModuleClass(preparationChildren[1].attributes, "preparation_stepList") || hasLooseText(preparation.innerHtml, preparationChildren)) return null;
  const list = preparationChildren[1], rows = directElements(list.innerHtml), steps = [];
  const listOffset = preparation.openingEnd + list.openingEnd;
  if (!rows.length || hasLooseText(list.innerHtml, rows) || isHidden(list, preparation.openingEnd)) return null;
  consumedCore.add(preparation.openingEnd + list.index);
  for (const [index, row] of rows.entries()) {
    const children = directElements(row.innerHtml), offset = listOffset + row.openingEnd;
    if (row.tag !== "li" || !hasModuleClass(row.attributes, "preparation_step") || isHidden(row, listOffset) || children.length !== 2 || children.some(child => child.tag !== "div") ||
      !hasModuleClass(children[0].attributes, "preparation_stepNumber") || !hasModuleClass(children[1].attributes, "preparation_stepContent") ||
      htmlText(children[0].innerHtml) !== `Step ${index + 1}` || isHidden(children[0], offset, true) || isHidden(children[1], offset) || hasLooseText(row.innerHtml, children)) return null;
    const paragraphs = directElements(children[1].innerHtml);
    if (!paragraphs.length || paragraphs[0].tag !== "p" || hasLooseText(children[1].innerHtml, paragraphs) ||
      paragraphs.some(paragraph => paragraph.tag !== "p" && !(paragraph.tag === "div" && hasModuleClass(paragraph.attributes, "stepmedia_stepMedia")))) return null;
    const textParagraphs = paragraphs.filter(paragraph => paragraph.tag === "p");
    if (textParagraphs.some(paragraph => !htmlText(paragraph.innerHtml) || isHidden(paragraph, offset + children[1].openingEnd, true) || /<(?:ul|ol|h[1-6]|div|p)\b/iu.test(paragraph.innerHtml))) return null;
    consumedCore.add(listOffset + row.index);
    children.forEach(child => consumedCore.add(offset + child.index));
    steps.push(instruction(textParagraphs.map(paragraph => text(paragraph.innerHtml)).join(" "), "text/html", `.recipe #preparation_section li:nth-child(${index + 1}) .preparation_stepContent`));
  }
  result.fields.instructionSections = [{ name: null, steps }];
  // Every marked required component must be consumed, even inside otherwise
  // ignorable grocery controls or step-media blocks.
  if (consumedCore.size !== markedCore.length || markedCore.some(element => !consumedCore.has(element.index))) return null;
  const bylines = modules(header.innerHtml, "byline_byline");
  if (bylines.length > 1) return null;
  if (bylines.length) {
    const parts = directElements(bylines[0].innerHtml), roles = new Map(), offset = root.openingEnd + header.openingEnd + bylines[0].openingEnd;
    if (!parts.length || isHidden(bylines[0], root.openingEnd + header.openingEnd) || hasLooseText(bylines[0].innerHtml, parts)) return null;
    for (const part of parts) {
      const match = text(part.innerHtml).match(/^(By|Recipe from|Adapted by)\s+(.+)$/u);
      if (part.tag !== "h2" || !hasModuleClass(part.attributes, "byline_bylinePart") || !match || roles.has(match[1]) || isHidden(part, offset, true)) return null;
      roles.set(match[1], match[2]);
    }
    if (roles.has("By") && roles.size !== 1) return null;
    result.source.author = roles.get("By") ?? roles.get("Adapted by") ?? roles.get("Recipe from") ?? null;
    result.source.provenance.author = provenance("text/html", ".recipe .byline_byline");
    if (!roles.has("By")) {
      result.source.attribution = parts.map(part => text(part.innerHtml)).join("; ");
      result.source.provenance.attribution = provenance("text/html", ".recipe .byline_bylinePart");
    }
  }
  const descriptions = modules(rootHtml, "topnote_topnoteParagraphs");
  if (descriptions.length > 1) return null;
  if (descriptions.length) {
    const paragraphs = directElements(descriptions[0].innerHtml);
    if (isHidden(descriptions[0], root.openingEnd, true) || paragraphs.some(paragraph => paragraph.tag !== "p") || hasLooseText(descriptions[0].innerHtml, paragraphs)) return null;
    const descriptionText = text(descriptions[0].innerHtml);
    if (descriptionText) result.fields.description = scalar(descriptionText, "text/html", ".recipe .topnote_topnoteParagraphs");
  }
  const timeTables = modules(rootHtml, "stats_cookingTimeTable");
  if (timeTables.length > 1) return null;
  if (timeTables.length) {
    const table = timeTables[0], parts = directElements(table.innerHtml), labels = new Set();
    if (parts.length % 2 || hasLooseText(table.innerHtml, parts) || isHidden(table)) return null;
    const fractionValues = { "¼": 1 / 4, "½": 1 / 2, "¾": 3 / 4, "⅐": 1 / 7, "⅑": 1 / 9, "⅒": 1 / 10, "⅓": 1 / 3, "⅔": 2 / 3, "⅕": 1 / 5, "⅖": 2 / 5, "⅗": 3 / 5, "⅘": 4 / 5, "⅙": 1 / 6, "⅚": 5 / 6, "⅛": 1 / 8, "⅜": 3 / 8, "⅝": 5 / 8, "⅞": 7 / 8 };
    for (let index = 0; index < parts.length; index += 2) {
      const label = htmlText(parts[index].innerHtml), value = htmlText(parts[index + 1].innerHtml), offset = root.openingEnd + table.openingEnd;
      const field = { "Total Time": "totalTime", "Ready In": "totalTime", "Prep Time": "prepTime", "Cook Time": "cookTime" }[label];
      if (parts[index].tag !== "dt" || parts[index + 1].tag !== "dd" || !value || !field || labels.has(field) || parts.slice(index, index + 2).some(part => isHidden(part, offset, true))) return null;
      labels.add(field);
      const normalized = value.replace(/(\d)([\u00bc-\u00be\u2150-\u215e])/gu, "$1 $2");
      const tokens = [...normalized.matchAll(new RegExp(`(${TASTY_AMOUNT_PATTERN})\\s*(days?|hours?|hrs?|hr|minutes?|mins?|min)`, "giu"))];
      const durationToken = `${TASTY_AMOUNT_PATTERN}\\s*(?:days?|hours?|hrs?|hr|minutes?|mins?|min)`;
      const exactGrammar = new RegExp(`^${durationToken}(?:(?:\\s+|\\s+and\\s+|,\\s*(?:and\\s+)?)${durationToken})*$`, "iu").test(normalized);
      let minutes = 0;
      for (const token of tokens) {
        let number = 0;
        for (const part of token[1].split(/\s+/u)) {
          const fraction = part.match(/^(\d+)\/(\d+)$/u);
          number += fraction ? Number(fraction[1]) / Number(fraction[2]) : fractionValues[part] ?? Number(part.replace(",", "."));
        }
        minutes += number * (token[2].toLowerCase().startsWith("d") ? 1440 : token[2].toLowerCase().startsWith("h") ? 60 : 1);
      }
      const exact = tokens.length > 0 && exactGrammar && Number.isSafeInteger(minutes) && minutes >= 0;
      result.fields[field] = { ...scalar(`${label}: ${value}`, "text/html", `.recipe .stats_cookingTimeTable (${label})`), value: exact ? minutes : null, confidence: exact ? "high" : "low" };
      if (!exact) result.warnings.push({ code: "UNRESOLVED_PROVIDER_DURATION", severity: "warning", path: `fields.${field}`, message: "Qualified or unsupported duration text is retained literally; exact minutes are not inferred." });
    }
  }
  const tips = modules(rootHtml, "tips_tips");
  if (tips.length > 1) return null;
  if (tips.length) {
    const tip = tips[0], parts = directElements(tip.innerHtml);
    if (isHidden(tip, root.openingEnd, true) || parts.length !== 2 || parts[0].tag !== "span" || !/^(?:Tip|Tips)$/u.test(htmlText(parts[0].innerHtml)) ||
      parts[1].tag !== "ul" || !hasModuleClass(parts[1].attributes, "tips_tipsList") || hasLooseText(tip.innerHtml, parts)) return null;
    const rows = directElements(parts[1].innerHtml);
    if (!rows.length || hasLooseText(parts[1].innerHtml, rows)) return null;
    for (const [index, row] of rows.entries()) {
      if (row.tag !== "li" || !hasModuleClass(row.attributes, "tips_tipsItem") || !htmlText(row.innerHtml)) return null;
      result.fields.notes.push(scalar(text(row.innerHtml), "text/html", `.recipe .tips_tipsItem:nth-child(${index + 1})`));
    }
  }
  const imageBlocks = modules(rootHtml, "recipeintro_image-block");
  if (imageBlocks.length === 1 && !isHidden(imageBlocks[0])) {
    const block = imageBlocks[0], offset = root.openingEnd + block.openingEnd;
    const figures = elementsMatchingAttributes(block.innerHtml, (_, tag) => tag === "figure");
    for (const image of modules(block.innerHtml, "headermediacarousel_hero")) {
      if (image.tag !== "img" || isHidden(image, offset)) continue;
      const figure = figures.find(figure => figure.openingEnd <= image.index && image.end <= figure.contentEnd);
      const credits = figure ? modules(figure.innerHtml, "headermediacarousel_credit") : [];
      const credit = credits.length === 1 && !isHidden(credits[0], offset + figure.openingEnd, true) ? htmlText(credits[0].innerHtml) || null : null;
      const urls = [image.attributes.get("src"), ...(image.attributes.get("srcset") ?? "").split(",").map(value => value.trim().split(/\s+/u)[0])];
      for (const url of urls) {
        try { if (!url || !["http:", "https:"].includes(new URL(url).protocol)) continue; } catch { continue; }
        if (!result.fields.images.some(retained => retained.sourceUrl === url)) result.fields.images.push({ sourceUrl: url, alt: image.attributes.get("alt") || null, credit, provenance: provenance("text/html", ".recipe .recipeintro_image-block img.headermediacarousel_hero") });
      }
    }
  }
  if (result.fields.ingredientSections.some(section => section.name)) result.warnings.push({ code: "SECTION_NAMES_NOT_PERSISTED", severity: "warning", path: "fields.ingredientSections", message: "Explicit group names remain in preview and optional commentary; the current editor stores flat content." });
  result.warnings.push(...result.fields.images.map((_, index) => ({ code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION", severity: "info", path: `fields.images[${index}]`, message: "Image references are unfetched and require confirmation before mapping." })));
  if (result.warnings.length) result.status = "needs_review";
  appendHtmlAuditEvidence(result, htmlAudit, discardedRemoteResourceCount(htmlAudit, result.fields.images));
  return result;
}

function parseHtml(html, sourceUrl, htmlAudit) {
  // Remove active/raw-text regions before scanning. No markup is inserted into a DOM.
  const inertHtml = discardRawContent(html);
  const sourceAdapterCandidate = parseFoodNetworkVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseSallysVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseDotdashVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseNYTCookingVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseFoodComVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseFood52VisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseAllRecipesVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseBudgetBytesWprm(inertHtml, sourceUrl, htmlAudit) ??
    parseTastyVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
    parseRecipeTinEatsWprm(inertHtml, sourceUrl, htmlAudit);
  if (sourceAdapterCandidate) return sourceAdapterCandidate;
  const headings = htmlHeadings(inertHtml);
  const roots = rootRanges(inertHtml);
  const regions = recipeRegions(headings, roots)
    .map((region) => {
      const lists = extractRegionLists(inertHtml, headings, roots, region);
      if (!lists) return null;
      const title = regionTitle(headings, roots, region);
      return {
        ...region,
        ...title,
        lists,
        navigationIngredientList: (lists.groupedIngredients
          ? lists.groupedIngredients.map((section) => section.list)
          : [lists.ingredients])
          .find((list) => hasAncestorTag(inertHtml, list.index, "nav")) ?? null,
      };
    })
    .filter(Boolean)
    .sort((left, right) => scoreRegion(right) - scoreRegion(left));
  if (regions.length === 0) return null;

  const region = regions[0];
  const { ingredientHeading, root, title, collectionTitle, lists } = region;
  const {
    groupedIngredients,
    groupedInstructions,
    ingredients,
    instructions,
    competingMeasurementVariants,
  } = lists;
  if (region.navigationIngredientList) {
    const unsupported = unsupportedNavigationCandidate(
      inertHtml,
      sourceUrl,
      root,
      region.navigationIngredientList,
    );
    appendHtmlAuditEvidence(
      unsupported,
      htmlAudit,
      discardedRemoteResourceCount(htmlAudit, []),
    );
    return unsupported;
  }
  if (!title) {
    const unsupported = unsupportedTitlelessCandidate(sourceUrl, root, lists);
    appendHtmlAuditEvidence(
      unsupported,
      htmlAudit,
      discardedRemoteResourceCount(htmlAudit, []),
    );
    return unsupported;
  }

  const editorialRegions = editorialIngredientRegions(inertHtml, headings, roots, region);
  const lowerScoringRegions = regions.slice(1).filter(
    ({ title: alternativeTitle, navigationIngredientList }) =>
      alternativeTitle && navigationIngredientList === null,
  );
  const hasCompetingRegions = editorialRegions.length > 0 || lowerScoringRegions.length > 0;
  const usesRegionScorer = hasCompetingRegions || collectionTitle !== null;

  const result = candidate(
    {
      id: usesRegionScorer ? "generic-region-scorer" : "generic-headings",
      version: "1",
      tier: "generic_html",
      sourceAdapter: null,
    },
    sourceUrl,
  );
  result.fields.title = scalar(
    title.text,
    "text/html",
    elementLocator(inertHtml, root, title, { ordinal: title.tag !== "h1" }),
  );
  const recipeContentStart = root?.contentStart ?? 0;
  const afterTitleStart = title.end ?? recipeContentStart;
  const afterTitle = inertHtml.slice(afterTitleStart, ingredientHeading.index);
  const durations = regionDurations(
    inertHtml,
    afterTitleStart,
    ingredientHeading.index,
    root,
  );
  result.fields.prepTime = durations.get("prepTime") ?? null;
  result.fields.cookTime = durations.get("cookTime") ?? null;
  result.fields.totalTime = durations.get("totalTime") ?? null;
  result.fields.yield = selectedYield(inertHtml, afterTitleStart, ingredientHeading.index);
  const descriptionMatch = [...afterTitle.matchAll(/<p\b[^>]*>([\s\S]*?)<\/p\s*>/giu)]
    .find((match) => labeledDuration(htmlText(match[1])) === null);
  if (descriptionMatch) {
    const description = htmlText(descriptionMatch[1]);
    if (description) {
      const descriptionElement = {
        tag: "p",
        index: afterTitleStart + (descriptionMatch.index ?? 0),
      };
      result.fields.description = scalar(
        description,
        "text/html",
        elementLocator(inertHtml, root, descriptionElement),
      );
    }
  }
  const supplementary = supplementarySections(inertHtml, headings, roots, root);
  result.fields.equipment = supplementary.equipment;
  result.fields.notes = supplementary.notes;
  const imageExtraction = imageReferences(inertHtml, roots, root);
  result.fields.images = imageExtraction.images;
  const remoteResourceCount = discardedRemoteResourceCount(htmlAudit, result.fields.images);
  const usesSafetyScopedListLocators = root && (
    htmlAudit.scriptCount > 0 || htmlAudit.iframeCount > 0 || remoteResourceCount > 0
  );
  const usesRootedListLocators =
    result.fields.equipment.length > 0 ||
    result.fields.notes.length > 0 ||
    result.fields.images.length > 0;
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
        ingredient(
          raw,
          "text/html",
          usesRootedListLocators
            ? `${rootedElementLocator(inertHtml, root, ingredients)} > li${ingredients.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : usesSafetyScopedListLocators
            ? `${root.tag} ${ingredients.tag} > li${ingredients.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : competingMeasurementVariants.length > 0
            ? `${listLocator(ingredients)} > li${ingredients.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : editorialRegions.length > 0 && root
            ? `${root.tag} ${ingredients.tag} > li${ingredients.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : `${ingredients.tag} > li:nth-child(${index + 1})`,
        )),
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
        instruction(
          raw,
          "text/html",
          usesRootedListLocators
            ? `${rootedElementLocator(inertHtml, root, instructions)} > li${instructions.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : usesSafetyScopedListLocators
            ? `${root.tag} ${instructions.tag} > li${instructions.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : competingMeasurementVariants.length > 0
            ? `${instructions.tag} > li${instructions.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : editorialRegions.length > 0 && root
            ? `${root.tag} ${instructions.tag} > li${instructions.items.length > 1 ? `:nth-child(${index + 1})` : ""}`
            : `${instructions.tag} > li:nth-child(${index + 1})`,
        )),
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
  if (collectionTitle) {
    result.status = "needs_review";
    result.competingCandidates.push({
      reason: "Page title describes a collection above the selected recipe region",
      title: collectionTitle.text,
      provenance: provenance("text/html", elementLocator(inertHtml, root, collectionTitle)),
    });
    result.warnings.push({
      code: "COMPETING_RECIPE_REGION",
      severity: "warning",
      path: "competingCandidates[0]",
      message: "The selected recipe is nested below a different page title; confirm the embedded recipe title.",
    });
  } else if (hasCompetingRegions) {
    result.status = "needs_review";
    result.competingCandidates.push(
      ...editorialRegions,
      ...lowerScoringRegions.map((alternative) => ({
        reason: "Lower-scoring coherent recipe region",
        title: alternative.title.text,
        provenance: provenance(
          "text/html",
          elementLocator(inertHtml, alternative.root, alternative.title, {
            ordinal: alternative.title.tag !== "h1",
          }),
        ),
      })),
    );
    result.warnings.push({
      code: "COMPETING_RECIPE_REGION",
      severity: "warning",
      path: "competingCandidates[0]",
      message: editorialRegions.length > 0
        ? "A lower-scoring ingredient-like region was excluded; confirm the selected card."
        : "A lower-scoring recipe region was excluded; confirm the selected card.",
    });
  }
  if (competingMeasurementVariants.length > 0) {
    result.status = "needs_review";
    result.competingCandidates.push(...competingMeasurementVariants.map(({ list, state }) => ({
      reason: measurementVariantReason(list, state),
      title: null,
      provenance: provenance("text/html", listLocator(list)),
    })));
  }
  if (
    result.fields.cookTime &&
    result.fields.totalTime &&
    result.fields.cookTime.value > result.fields.totalTime.value
  ) {
    result.status = "needs_review";
    result.fields.cookTime.confidence = "low";
    result.warnings.push({
      code: "SUSPICIOUS_DURATION",
      severity: "warning",
      path: "fields.cookTime",
      message: "Active time exceeds total time; preserve the source value for review.",
    });
  }
  if (result.fields.images.length > 0) {
    result.status = "needs_review";
    if (imageExtraction.attribution) {
      result.source.attribution = imageExtraction.attribution.value;
      result.source.provenance.attribution = imageExtraction.attribution.provenance;
    }
    result.warnings.push(...result.fields.images.map((_, index) => ({
      code: "IMAGE_REFERENCE_REQUIRES_CONFIRMATION",
      severity: "info",
      path: `fields.images[${index}]`,
      message: "The image reference is not fetched and requires confirmation before mapping.",
    })));
  }
  if (result.fields.notes.length > 0) {
    result.status = "needs_review";
    result.warnings.push({
      code: "NOTES_REQUIRE_MAPPING",
      severity: "info",
      path: "fields.notes",
      message: "Notes remain separate until the user chooses where to place them.",
    });
  }
  appendHtmlAuditEvidence(result, htmlAudit, remoteResourceCount);
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
    const htmlAudit = clipboardHtmlAudit(html);
    const htmlCandidate = parseHtml(html, sourceUrl, htmlAudit);
    if (htmlCandidate) return htmlCandidate;
    if (plainText.trim()) {
      const textCandidate = parsePlainText(plainText, sourceUrl);
      if (textCandidate) {
        appendHtmlAuditEvidence(
          textCandidate,
          htmlAudit,
          discardedRemoteResourceCount(htmlAudit, []),
        );
        return textCandidate;
      }
    }
  }
  if (plainText.trim()) {
    const textCandidate = parsePlainText(plainText, sourceUrl);
    if (textCandidate) return textCandidate;
  }
  throw new Error("Clipboard input does not contain a recognizable recipe.");
}
