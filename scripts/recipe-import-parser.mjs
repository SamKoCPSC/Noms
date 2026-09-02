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

const DISCARDED_RAW_CONTENT_TAGS = "script|style|iframe|template|noscript";
const REMOTE_RESOURCE_ATTRIBUTES = new Set(["action", "href", "poster", "src", "srcset"]);

function discardRawContent(html, preserveOpeningTags = false) {
  const pattern = new RegExp(
    `(<(${DISCARDED_RAW_CONTENT_TAGS})\\b(?:"[^"]*"|'[^']*'|[^>"'])*>)(?:[\\s\\S]*?<\\/\\2\\s*>|[\\s\\S]*$)`,
    "giu",
  );
  return html.replace(pattern, preserveOpeningTags ? "$1" : "");
}

function openingHtmlElements(html) {
  // Raw element bodies are intentionally opaque so markup-looking script text is
  // not mistaken for page controls or resources during the safety audit.
  const auditableHtml = discardRawContent(html.replace(/<!--[\s\S]*?-->/gu, ""), true);
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

function imageReferences(html, roots, root) {
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
    const sourceUrl = normalizeWhitespace(attributes.get("src") ?? "");
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

function parseHtml(html, sourceUrl) {
  const htmlAudit = clipboardHtmlAudit(html);
  // Remove active/raw-text regions before scanning. No markup is inserted into a DOM.
  const inertHtml = discardRawContent(html);
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
    return unsupportedNavigationCandidate(
      inertHtml,
      sourceUrl,
      root,
      region.navigationIngredientList,
    );
  }
  if (!title) {
    return unsupportedTitlelessCandidate(sourceUrl, root, lists);
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
    const htmlCandidate = parseHtml(html, sourceUrl);
    if (htmlCandidate) return htmlCandidate;
  }
  if (plainText.trim()) {
    const textCandidate = parsePlainText(plainText, sourceUrl);
    if (textCandidate) return textCandidate;
  }
  throw new Error("Clipboard input does not contain a recognizable recipe.");
}
