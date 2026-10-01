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

function parseHtml(html, sourceUrl, htmlAudit) {
  // Remove active/raw-text regions before scanning. No markup is inserted into a DOM.
  const inertHtml = discardRawContent(html);
  const sourceAdapterCandidate = parseAllRecipesVisibleRecipe(inertHtml, sourceUrl, htmlAudit) ??
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
