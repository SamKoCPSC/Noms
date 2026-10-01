import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import test from "node:test";
import { parseRecipeClipboard } from "../recipe-import-parser.mjs";
import { validateImportCandidate, validateJsonSchema } from "../validate-recipe-import-fixtures.mjs";

const schema = JSON.parse(await fs.readFile(new URL("../../tests/fixtures/recipe-import/v1/import-candidate.schema.json", import.meta.url), "utf8"));
// Invented copy in the article/header, definition-list and grouped-list shapes
// seen in the saved Food52 captures. No provider recipe body is committed.
const measurement = text => `<span class="mr-1 inline-block font-bold lowercase">${text} </span>`;
const ingredient = (amount, name) => `<li>${measurement(amount)}${name}</li>`;
const step = (number, text) => `<li class="space-y-4"><span class="font-bold leading-4">Step ${number}</span><div class="RichText font-normal" role="document"><p>${text}</p></div></li>`;
const group = (name, rows) => `<div class="space-y-3">${name ? `<h3>${name}</h3>` : ""}<ul class="space-y-4 print:space-y-2">${rows}</ul></div>`;
const section = (name, groups) => `<div class="space-y-6"><h2 class="print:hidden">${name}</h2><div class="space-y-8">${groups}</div></div>`;
const details = `<dl><dt>Serves</dt><dd>4</dd></dl><dl><dt>Prep Time</dt><dd>15 Minutes</dd></dl><dl><dt>Cook Time</dt><dd>1 Hour 5 Minutes</dd></dl>`;
const recipe = `<nav><h2>Ingredients</h2><ul><li>Navigation noise</li></ul></nav><main><article>
<div class="hidden print:block"><svg></svg></div>
<header id="contentHeader"><h1>Moon Garden Noodles</h1><p><span>By</span><span>Aster Vale</span></p><p>Test Kitchen-Approved</p></header>
<div><img src="https://images.example.invalid/noodles.jpg" alt="Noodles"><p>Photo by Orin Sky</p></div>
<div><p>Featured Video</p><h2>Moon Garden Noodles</h2><div data-jw-placement-id="video"></div><a href="/watch">See all Food52 videos</a></div>
<div>${details}</div><div class="RichText font-serif" role="document"><p>A bright supper with <em>crunch</em>.</p><p>Make it your own.</p></div>
${section("Ingredients", group("Noodles", ingredient("2 ⅓ cup", "cooked noodles") + ingredient("10 to 15", "thin slices moon chile") + ingredient("1", "(8 ounce) package tofu")) + group("Finish", ingredient("About ½ cup", "chopped herbs") + ingredient("2 to 3 tablespoon", "crunchy seeds") + '<li>salt, to taste</li>'))}
<div id="aditude-in-content-recipes"><p>Promotion, not a recipe step.</p></div>
<div class="RichText" role="document"><p>Shared in partnership with Moon Farm.</p></div>
${section("Directions", group("Noodles", step(1, "Mix gently.") + step(2, "Cook and cool.")) + group("Finish", step(1, "Top with seeds and serve.")))}
</article></main><aside><h2>Recommended recipes</h2><img src="https://images.example.invalid/other.jpg"></aside>`;
function parse(html = recipe, sourceUrl = "https://food52.com/recipes/1-moon-noodles") {
  return parseRecipeClipboard({ html, plainText: null, sourceUrl });
}
function refusesAdapter(html, sourceUrl, message) {
  let candidate;
  try { candidate = parse(html, sourceUrl); }
  catch (error) { assert.equal(error.message, "Clipboard input does not contain a recognizable recipe."); return; }
  assert.notEqual(candidate.parser.sourceAdapter, "food52", message);
}

test("Food52 retains complete ingredient and direction groups without video, navigation or step labels", () => {
  const c = parse();
  assert.equal(c.parser.sourceAdapter, "food52");
  assert.equal(c.fields.title.value, "Moon Garden Noodles");
  assert.deepEqual(c.fields.ingredientSections.map(s => s.name), ["Noodles", "Finish"]);
  assert.deepEqual(c.fields.instructionSections.map(s => [s.name, s.steps.map(t => t.text)]), [
    ["Noodles", ["Mix gently.", "Cook and cool."]], ["Finish", ["Top with seeds and serve."]],
  ]);
  assert.deepEqual(c.fields.ingredientSections.flatMap(s => s.items).map(i => [i.amount, i.unit, i.name]), [
    ["2 ⅓", "cup", "cooked noodles"], ["10 to 15", null, "thin slices moon chile"],
    ["1", null, "(8 ounce) package tofu"], ["About ½", "cup", "chopped herbs"],
    ["2 to 3", "tablespoon", "crunchy seeds"], [null, null, "salt, to taste"],
  ]);
  assert.deepEqual(validateJsonSchema(c, schema), []);
  assert.deepEqual(validateImportCandidate(c), []);
  assert.deepEqual(parse(), c);
});

test("Food52 recovers metadata, complete description and safe hero attribution without fetching images", () => {
  const c = parse();
  assert.equal(c.source.provider, "Food52");
  assert.equal(c.source.author, "Aster Vale");
  assert.equal(c.fields.description.value, "A bright supper with crunch. Make it your own.");
  assert.equal(c.fields.prepTime.value, 15);
  assert.equal(c.fields.cookTime.value, 65);
  assert.equal(c.fields.totalTime, null);
  assert.deepEqual(c.fields.yield.value, { display: "Serves: 4", servings: 4 });
  assert.equal(c.fields.images.length, 1);
  assert.equal(c.fields.images[0].credit, "Orin Sky");
  assert.equal(c.status, "needs_review");
});

test("Food52 distinguishes literal recipe yield from a safe numeric serving count", () => {
  for (const value of ["24 deviled eggs", "1 cocktail", "4 to 6", "about 28 cookies", "2 dozen"]) {
    const c = parse(recipe.replace('<dd>4</dd>', `<dd>${value}</dd>`));
    assert.equal(c.fields.yield.value.display, `Serves: ${value}`);
    assert.equal(c.fields.yield.value.servings, null);
  }
});

test("Food52 preserves unmarked rows as reviewable ingredients, not guessed section headings", () => {
  const c = parse(recipe.replace('<li>salt, to taste</li>', '<li>Moon Sauce</li><li>salt, to taste</li>'));
  const items = c.fields.ingredientSections[1].items;
  assert.equal(items[2].name, "Moon Sauce");
  assert.equal(items[2].confidence, "low");
  assert.ok(c.warnings.some(w => w.code === "UNSTRUCTURED_PROVIDER_INGREDIENT"));
});

test("Food52 preserves bounded marked measurement ranges and unfamiliar units", () => {
  for (const [raw, amount, unit] of [["1 to 1 ½ pound", "1 to 1 ½", "pound"], ["4-5", "4-5", null], ["2 dash", "2", "dash"], ["¼ bunch", "¼", "bunch"]]) {
    const item = parse(recipe.replace('10 to 15 </span>', `${raw} </span>`)).fields.ingredientSections[0].items[1];
    assert.deepEqual([item.amount, item.unit, item.name], [amount, unit, "thin slices moon chile"]);
  }
});

test("Food52 adapter requires the exact supplied HTTP hostname", () => {
  assert.equal(parse(recipe, "http://www.food52.com/recipes/1").parser.sourceAdapter, "food52");
  for (const url of [null, "https://food52.com.example.invalid/", "https://notfood52.com/", "https://example.invalid/?source=food52.com", "ftp://food52.com/"]) {
    refusesAdapter(recipe, url);
  }
});

test("Food52 missing optional fields remain missing and do not borrow sponsor or video copy", () => {
  const html = recipe.replace(details, "").replace('<div class="RichText font-serif" role="document"><p>A bright supper with <em>crunch</em>.</p><p>Make it your own.</p></div>', "").replace('<p><span>By</span><span>Aster Vale</span></p>', "");
  const c = parse(html);
  assert.equal(c.parser.sourceAdapter, "food52");
  assert.equal(c.fields.description, null);
  assert.equal(c.source.author, null);
  assert.equal(c.fields.prepTime, null);
  assert.equal(c.fields.cookTime, null);
  assert.equal(c.fields.yield, null);
  assert.deepEqual(c.fields.notes, []);
});

test("Food52 preserves complete groups with an empty heading without inventing a label", () => {
  const c = parse(recipe.replaceAll('<h3>Noodles</h3>', '<h3></h3>'));
  assert.equal(c.parser.sourceAdapter, "food52");
  assert.equal(c.fields.ingredientSections[0].name, null);
  assert.equal(c.fields.ingredientSections[0].items.length, 3);
  assert.equal(c.fields.instructionSections[0].name, null);
  assert.equal(c.fields.instructionSections[0].steps.length, 2);
  assert.ok(c.warnings.some(w => w.code === "EMPTY_PROVIDER_SECTION_NAME"));
});

test("Food52 rejects duplicate, disconnected, incomplete and changed core layouts to generic fallback", () => {
  for (const [index, html] of [
    recipe + recipe,
    recipe.replace('id="contentHeader"', 'id="changedHeader"'),
    recipe.replace('<h1>Moon Garden Noodles</h1>', '<h1></h1>'),
    recipe.replace('Step 2</span>', 'Step 4</span>'),
    recipe.replace('role="document"><p>Cook and cool.', 'role="document"><p></p><p>'),
    recipe.replace('class="space-y-8"', 'class="changedGroups"'),
    recipe.replace('Cook and cool.</p></div>', 'Cook and cool.</p></div><p>Cook another hour.</p>'),
    recipe.replace('Cook and cool.</p></div>', 'Cook and cool.</p></div>Rest another hour.'),
    recipe.replace('class="print:hidden">Ingredients</h2>', 'class="print:hidden">Ingredients</h2><h2>Ingredients</h2>'),
    recipe.replace('>Directions</h2>', '>Directions</h2><h2>Directions</h2>'),
    recipe.replace('</header>', '</header></article><article>'),
    recipe.replace('<dd>4</dd>', '<dd>4</dd><dd>6</dd>'),
    recipe.replace('By</span><span>Aster Vale</span></p>', 'By</span><span>Aster Vale</span></p><p>By Other Person</p>'),
    recipe.replace('Cook and cool.</p></div>', 'Cook and cool.</p><ul><li>Unknown nested step.</li></ul></div>'),
  ].entries()) refusesAdapter(html, undefined, `variant ${index}`);
});

test("Food52 required fields cannot be explicitly hidden, including ancestors and nested text", () => {
  for (const html of [
    `<div hidden>${recipe}</div>`, `<div aria-hidden="true">${recipe}</div>`,
    recipe.replace('<article>', '<article style="display: none">'),
    recipe.replace('<h1>', '<h1 class="hidden">'),
    recipe.replace('>Directions</h2>', '>Directions</h2>').replace('class="space-y-8"', 'hidden class="space-y-8"'),
    recipe.replace('Cook and cool.</p>', '<span hidden>Cook and cool.</span></p>'),
    recipe.replace('10 to 15 </span>', '<em hidden>10 to 15</em> </span>'),
    recipe.replace('<dd>4</dd>', '<dd hidden>4</dd>'),
    recipe.replace('By</span><span>Aster Vale', 'By</span><span hidden>Aster Vale'),
    recipe.replace('class="RichText font-serif"', 'hidden class="RichText font-serif"'),
  ]) refusesAdapter(html);
});

test("Food52 active/raw markup is opaque and unsafe hero URLs are not retained", () => {
  assert.equal(parse(`<script>${recipe}</script><iframe>${recipe}</iframe><template>${recipe}</template>${recipe}`).parser.sourceAdapter, "food52");
  const c = parse(recipe.replace('src="https://images.example.invalid/noodles.jpg"', 'src="javascript:alert(1)"'));
  assert.deepEqual(c.fields.images, []);
  assert.ok(c.warnings.some(w => w.code === "ACTIVE_CONTENT_DISCARDED"));
});

test("Food52 refuses unconsumed recipe-shaped lists and procedure markers elsewhere in the article", () => {
  for (const extra of [
    group("Serving", step(1, "Chill overnight before serving.")),
    `<ul class="space-y-4">${ingredient("2 cup", "moon flour")}</ul>`,
    `<div>${step(1, "Rest another hour.")}</div>`,
  ]) refusesAdapter(recipe.replace('</article>', `${extra}</article>`));
});

test("Food52 core sections and header cannot be inside navigation", () => {
  for (const html of [
    recipe.replace('<header id="contentHeader">', '<nav><header id="contentHeader">').replace('</header>', '</header></nav>'),
    recipe.replace(section("Directions", group("Noodles", step(1, "Mix gently.") + step(2, "Cook and cool.")) + group("Finish", step(1, "Top with seeds and serve."))), `<nav>${section("Directions", group("Noodles", step(1, "Mix gently.") + step(2, "Cook and cool.")) + group("Finish", step(1, "Top with seeds and serve.")))}</nav>`),
  ]) refusesAdapter(html);
});

test("Food52 duration aliases must not silently overwrite the same mapped field", () => {
  refusesAdapter(recipe.replace(details, `${details}<dl><dt>Preparation Time</dt><dd>2 Hours</dd></dl>`));
});

test("Food52 hidden optional hero and credit are not imported", () => {
  const noImage = parse(recipe.replace('<img src=', '<img hidden src='));
  assert.equal(noImage.parser.sourceAdapter, "food52");
  assert.deepEqual(noImage.fields.images, []);
  const noCredit = parse(recipe.replace('<p>Photo by Orin Sky</p>', '<p hidden>Photo by Orin Sky</p>'));
  assert.equal(noCredit.parser.sourceAdapter, "food52");
  assert.equal(noCredit.fields.images[0].credit, null);
});
