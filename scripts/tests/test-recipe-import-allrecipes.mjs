import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import test from "node:test";
import { parseRecipeClipboard } from "../recipe-import-parser.mjs";
import { validateImportCandidate, validateJsonSchema } from "../validate-recipe-import-fixtures.mjs";

const schema = JSON.parse(await fs.readFile(new URL("../../tests/fixtures/recipe-import/v1/import-candidate.schema.json", import.meta.url), "utf8"));

// Invented content in the visible structures observed in the AllRecipes corpus.
const row = (quantity, unit, name) => `<li class="mm-recipes-structured-ingredients__list-item"><p><span data-ingredient-quantity="true">${quantity}</span> <span data-ingredient-unit="true">${unit}</span> <span data-ingredient-name="true">${name}</span></p></li>`;
const detail = (label, value) => `<div class="mm-recipes-details__item"><div class="mm-recipes-details__label">${label}:</div><div class="mm-recipes-details__value">${value}</div></div>`;
const recipe = `<article>
<h1 class="article-heading">Lunar Lentil Loaf</h1><p class="article-subheading">A <strong>bright</strong> supper.</p>
<div class="mntl-bylines__group--ugc_recipe"><span class="mntl-attribution__item-name">Aster Vale</span></div>
<div class="mntl-bylines__group--recipe_tester"><a class="mntl-attribution__item-name">Other Tester</a></div>
<div class="mm-recipes-details">${detail("Prep Time", "15 mins")}${detail("Cook Time", "1 hr 5 mins")}${detail("Total Time", "1 hr 20 mins")}${detail("Servings", "4")}${detail("Yield", "1 loaf")}</div>
<h2>Ingredients for an editorial example</h2><ul><li>Do not import this.</li></ul>
<div class="mm-recipes-structured-ingredients"><h2>Ingredients</h2><input type="radio" checked value="1" class="mm-recipes-serving-size-adjuster__input">
<ul class="mm-recipes-structured-ingredients__list">${row("2 ⅓", "cups", "cooked lentils")}${row("1", "(8 ounce) can", "moon tomatoes")}${row("", "", "salt, to taste")}</ul>
<p class="mm-recipes-structured-ingredients__list-heading">Finish:</p><ul class="mm-recipes-structured-ingredients__list">${row("½", "cup", "breadcrumbs")}</ul></div>
<div class="mm-recipes-steps"><h2>Directions</h2><div class="mm-recipes-steps__content"><ol class="mntl-sc-block-group--OL">
<li class="mntl-sc-block-group--LI"><p class="mntl-sc-block-html">Mix <strong>gently</strong>.</p><p class="mntl-sc-block-html">Leave some texture.</p><figure><img src="https://images.example.invalid/loaf.jpg" alt="Loaf"><figcaption><p class="mntl-sc-block-html">Photo credit only</p></figcaption></figure><div class="mntl-sc-block-adslot">Subscribe now</div></li>
<li class="mntl-sc-block-group--LI"><p class="mntl-sc-block-html">Bake and serve.</p></li></ol>
<div class="allrecipes-sc-block-callout--cooksnote"><h3>Cook’s Note</h3><div class="mntl-sc-block-universal-callout__body"><p>Cool before slicing.</p><p>Keep leftovers chilled.</p></div></div></div></div>
<h2>Reviews</h2><p>Unrelated review.</p><figure><img src="https://images.example.invalid/unrelated.jpg"></figure></article>`;

function parse(html = recipe, sourceUrl = "https://www.allrecipes.com/recipe/1/lunar-loaf/") {
  return parseRecipeClipboard({ html, plainText: null, sourceUrl });
}

test("AllRecipes extracts structured rows, all groups and clean multi-paragraph steps", () => {
  const c = parse();
  assert.equal(c.parser.sourceAdapter, "allrecipes");
  assert.equal(c.source.provider, "AllRecipes");
  assert.equal(c.fields.title.value, "Lunar Lentil Loaf");
  assert.equal(c.fields.description.value, "A bright supper.");
  assert.deepEqual(c.fields.ingredientSections.map(s => s.name), [null, "Finish"]);
  assert.deepEqual(c.fields.ingredientSections.flatMap(s => s.items).map(({ amount, unit, name }) => ({ amount, unit, name })), [
    { amount: "2 ⅓", unit: "cups", name: "cooked lentils" },
    { amount: "1", unit: "(8 ounce) can", name: "moon tomatoes" },
    { amount: null, unit: null, name: "salt, to taste" },
    { amount: "½", unit: "cup", name: "breadcrumbs" },
  ]);
  assert.deepEqual(c.fields.instructionSections.flatMap(s => s.steps).map(s => s.text), ["Mix gently. Leave some texture.", "Bake and serve."]);
  assert.deepEqual(validateJsonSchema(c, schema), []);
  assert.deepEqual(validateImportCandidate(c), []);
  assert.deepEqual(parse(), c);
});

test("AllRecipes recovers visible metadata and preserves notes and unfetched image credits", () => {
  const c = parse();
  assert.equal(c.source.author, "Aster Vale");
  assert.deepEqual([c.fields.prepTime.value, c.fields.cookTime.value, c.fields.totalTime.value], [15, 65, 80]);
  assert.deepEqual(c.fields.yield.value, { display: "4 servings; 1 loaf", servings: 4 });
  assert.deepEqual(c.fields.notes.map(n => n.value), ["Cool before slicing. Keep leftovers chilled."]);
  assert.equal(c.fields.images.length, 1);
  assert.equal(c.fields.images[0].credit, "Photo credit only");
  assert.equal(c.status, "needs_review");
});

test("AllRecipes preserves safe lazy image references without loading them", () => {
  const c = parse(recipe.replace('src="https://images.example.invalid/loaf.jpg"', 'data-src="https://images.example.invalid/loaf.jpg"'));
  assert.equal(c.fields.images.length, 1);
  assert.equal(c.fields.images[0].sourceUrl, "https://images.example.invalid/loaf.jpg");
  assert.equal(c.fields.images[0].credit, "Photo credit only");
  assert.equal(parse(recipe.replace('src="https://images.example.invalid/loaf.jpg"', 'data-src="javascript:alert(1)"')).fields.images.length, 0);
});

test("AllRecipes keeps procedure recipe tips separate from instructions", () => {
  const c = parse(recipe.replace('class="allrecipes-sc-block-callout--cooksnote"', 'class="allrecipes-sc-block-callout theme-recipetip"'));
  assert.deepEqual(c.fields.notes.map(n => n.value), ["Cool before slicing. Keep leftovers chilled."]);
  assert.equal(c.fields.instructionSections[0].steps.length, 2);
});

test("AllRecipes adapter requires an exact supported HTTP hostname", () => {
  assert.equal(parse(recipe, "http://allrecipes.com/recipe/1/").parser.sourceAdapter, "allrecipes");
  for (const source of [null, "https://allrecipes.com.example.invalid/", "https://notallrecipes.com/", "https://example.invalid/?site=allrecipes.com", "ftp://allrecipes.com/"]) {
    assert.notEqual(parse(recipe, source).parser.sourceAdapter, "allrecipes");
  }
});

test("AllRecipes incomplete, duplicated or competing structures retain generic fallback", () => {
  for (const html of [
    recipe.replace('class="mm-recipes-structured-ingredients"', 'class="changed-card"'),
    recipe.replace('class="mm-recipes-steps__content"', 'class="changed-steps"'),
    recipe.replace('<span data-ingredient-name="true">cooked lentils</span>', ''),
    recipe.replace('<span data-ingredient-quantity="true">2 ⅓</span>', '<span data-ingredient-quantity="true">2 ⅓</span><span data-ingredient-quantity="true">7</span>'),
    recipe.replace('<p class="mntl-sc-block-html">Bake and serve.</p>', 'Changed procedure markup.'),
    recipe.replace('salt, to taste</span>', 'salt, to taste</span> plus chopped herbs'),
    recipe.replace('<ul class="mm-recipes-structured-ingredients__list">', '<ul class="mm-recipes-structured-ingredients__list"><li>Unmarked extra ingredient</li>'),
    recipe.replace('<ol class="mntl-sc-block-group--OL">', '<ol class="mntl-sc-block-group--OL"><li>Unmarked extra step</li>'),
    recipe + recipe,
    recipe.replace('<h2>Reviews</h2>', '<h1 class="article-heading">Other recipe</h1><h2>Reviews</h2>'),
  ]) assert.notEqual(parse(html).parser.sourceAdapter, "allrecipes");
});

test("AllRecipes markup in executable, frame and template bodies is ignored", () => {
  const c = parse(`<script>${recipe}</script><iframe>${recipe}</iframe><template>${recipe}</template>${recipe}`);
  assert.equal(c.parser.sourceAdapter, "allrecipes");
  assert.equal(c.fields.ingredientSections.flatMap(s => s.items).length, 4);
  assert.ok(c.warnings.some(w => w.code === "ACTIVE_CONTENT_DISCARDED"));
  assert.deepEqual(validateImportCandidate(c), []);
});

test("AllRecipes scaling never silently combines scaled ingredients with original servings", () => {
  const c = parse(recipe.replace('checked value="1"', 'checked value="2"'));
  assert.equal(c.parser.sourceAdapter, "allrecipes");
  assert.equal(c.fields.yield.value.servings, null);
  assert.ok(c.warnings.some(w => w.code === "SCALED_INGREDIENTS_YIELD_UNCERTAIN"));
  assert.deepEqual(validateImportCandidate(c), []);
});

test("AllRecipes missing optional metadata stays missing rather than inventing values", () => {
  const c = parse(recipe.replace(/<div class="mm-recipes-details">[\s\S]*?<h2>Ingredients for/, '<h2>Ingredients for'));
  assert.equal(c.parser.sourceAdapter, "allrecipes");
  assert.equal(c.fields.prepTime, null);
  assert.equal(c.fields.yield, null);
});

test("AllRecipes retains a single nested-list procedure with an empty paragraph spacer", () => {
  const html = recipe.replace('<p class="mntl-sc-block-html">Bake and serve.</p>', '<ol class="mntl-sc-block-html"><li>Bake and serve.</li></ol><p class="mntl-sc-block-html"><br></p>');
  const c = parse(html);
  assert.equal(c.parser.sourceAdapter, "allrecipes");
  assert.equal(c.fields.instructionSections[0].steps[1].text, "Bake and serve.");
  assert.ok(c.warnings.some(w => w.code === "NESTED_SINGLE_STEP_FLATTENED"));
  assert.deepEqual(validateImportCandidate(c), []);
  assert.notEqual(parse(html.replace('Bake and serve.</li>', 'Bake and serve.</li><li>Extra nested step.</li>')).parser.sourceAdapter, "allrecipes");
});

test("AllRecipes recovers an unmarked leading quantity without treating arbitrary text as an amount", () => {
  const html = recipe.replace('<span data-ingredient-quantity="true">2 ⅓</span>', '2 ⅓');
  const c = parse(html);
  assert.equal(c.parser.sourceAdapter, "allrecipes");
  assert.equal(c.fields.ingredientSections[0].items[0].amount, "2 ⅓");
  assert.deepEqual(validateImportCandidate(c), []);
  assert.notEqual(parse(html.replace('2 ⅓ <span', 'About a handful <span')).parser.sourceAdapter, "allrecipes");
});

test("AllRecipes adapter does not associate recipe fragments across separate articles or navigation", () => {
  for (const html of [recipe.replace('<div class="mm-recipes-steps">', '</article><article><div class="mm-recipes-steps">'), `<nav>${recipe}</nav>`]) {
    // The generic path may reject disjoint fragments rather than preview them.
    try { assert.notEqual(parse(html).parser.sourceAdapter, "allrecipes"); }
    catch (error) { assert.equal(error.message, "Clipboard input does not contain a recognizable recipe."); }
  }
});
