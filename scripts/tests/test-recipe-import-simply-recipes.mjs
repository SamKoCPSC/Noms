import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import { parseRecipeClipboard } from '../recipe-import-parser.mjs';
import { validateImportCandidate, validateJsonSchema } from '../validate-recipe-import-fixtures.mjs';

const recipe = await fs.readFile(new URL('../../tests/fixtures/recipe-import/providers/simply-recipes-visible.html', import.meta.url), 'utf8');
const schema = JSON.parse(await fs.readFile(new URL('../../tests/fixtures/recipe-import/v1/import-candidate.schema.json', import.meta.url), 'utf8'));
const parse = (html = recipe, sourceUrl = 'https://www.simplyrecipes.com/comet-bean-bake') => parseRecipeClipboard({ html, sourceUrl });
function refuses(html, sourceUrl) {
  let candidate;
  try { candidate = parse(html, sourceUrl); }
  catch (error) { assert.equal(error.message, 'Clipboard input does not contain a recognizable recipe.'); return; }
  assert.notEqual(candidate.parser.sourceAdapter, 'simply-recipes');
}

test('Simply Recipes selects the explicit card title and retains the differing editorial title for review', () => {
  const c = parse();
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.equal(c.fields.title.value, 'Comet Bean Bake');
  assert.equal(c.competingCandidates[0].title, 'The Bake I Make Every Moon Day');
  assert.equal(c.status, 'needs_review');
  assert.equal(c.fields.description.value, 'A bright invented bake for chilly evenings.');
  assert.equal(c.source.author, 'Rowan Vale');
  assert.deepEqual(validateImportCandidate(c), []);
  assert.deepEqual(validateJsonSchema(c, schema), []);
  assert.deepEqual(c, parse());
});
test('Simply Recipes falls back to the article title when the optional card title is absent', () => {
  const c = parse(recipe.replace('<h2 class="mm-recipes-details__title">Comet Bean Bake</h2>', ''));
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.equal(c.fields.title.value, 'The Bake I Make Every Moon Day');
  assert.deepEqual(c.competingCandidates, []);
});
test('Simply Recipes keeps groups, written fractions, package jars and compound measures', () => {
  const c = parse();
  assert.deepEqual(c.fields.ingredientSections.map(s => s.name), ['For the filling', 'To finish']);
  assert.deepEqual(c.fields.ingredientSections.flatMap(s => s.items.map(i => [i.amount, i.unit, i.name])), [
    ['1 and 1/2', 'cups', 'cloud flour (190g), spooned'],
    ['1', '(7-ounce) jar', 'comet beans, drained'],
    ['1½ cups plus 2 tablespoons', null, 'sky tomatoes'],
    [null, null, 'Butter or oil, to taste'],
  ]);
});
test('Simply Recipes preserves inline text adjacency and entire preparation suffixes', () => {
  const c = parse(recipe.replace('cloud flour</span>', 'cloud</span><span> flour</span>'));
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.equal(c.fields.ingredientSections[0].items[0].raw, '1 and 1/2 cups cloud flour (190g), spooned');
  assert.equal(c.fields.ingredientSections[1].items[0].raw, 'Butter or oil, to taste');
});
test('Simply Recipes keeps named steps and inline tips in order without photo credits or exact review CTA', () => {
  const c = parse();
  assert.deepEqual(c.fields.instructionSections.flatMap(s => s.steps.map(s => s.text)), [
    'Mix: Combine flour and beans. Tip: Keep the bowl chilled. Stir in tomatoes.',
    'Bake: Bake until golden.',
  ]);
  assert.deepEqual(c.fields.notes.map(n => n.value), ['Recipe Notes: Chill leftovers.', 'Recipe Notes: Use fresh zest.', 'Note: Drain the beans well.']);
  assert.ok(!JSON.stringify(c.fields).includes('Photo credit'));
  assert.ok(!JSON.stringify(c.fields).includes('reader suggestion'));
});
test('Simply Recipes never strips cooking content merely because it mentions reviews or comments', () => {
  const c = parse(recipe.replace('Love the recipe? Leave us stars and a comment below!', 'Love the recipe? Leave us stars and a comment below! Chill for 20 minutes.'));
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.ok(c.fields.instructionSections[0].steps[1].text.endsWith('Chill for 20 minutes.'));
});
test('Simply Recipes retains a headingless inline callout and refuses unsupported or misplaced callouts', () => {
  const c = parse(recipe.replace('<h3 class="mntl-sc-block-universal-callout__heading">Tip</h3>', ''));
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.ok(c.fields.instructionSections[0].steps[0].text.includes('Keep the bowl chilled.'));
  for (const html of [recipe.replace('Recipe Notes</h3>', 'Another Recipe</h3>'), recipe.replace('<p>Keep the bowl chilled.</p>', '<h2>Other recipe</h2><p>Lost action.</p>'), recipe.replace('Tip</h3>', 'Unknown section</h3>')]) refuses(html);
});
test('Simply Recipes exact durations and numeric servings preserve literal output counts and ranges', () => {
  const c = parse();
  assert.equal(c.fields.prepTime.value, 15);
  assert.equal(c.fields.cookTime.value, 65);
  assert.equal(c.fields.totalTime.value, 80);
  assert.equal(c.fields.yield.value.display, 'Servings: 4 to 6 servings; Yield: One casserole');
  assert.equal(c.fields.yield.value.servings, null);
  assert.equal(parse(recipe.replace('4 to 6 servings', '4 servings')).fields.yield.value.servings, 4);
  for (const duration of ['About 1 hour', '1 hr, plus cooling', '30 to 40 mins']) assert.equal(parse(recipe.replace('1 hr 20 mins', duration)).fields.totalTime.value, null);
});
test('Simply Recipes keeps unmapped metadata separate and does not infer equipment', () => {
  const c = parse();
  assert.ok(c.unclassifiedLines.some(l => l.raw === 'Rest Time: 10 mins'));
  assert.deepEqual(c.fields.equipment, []);
});
test('Simply Recipes exposes safe unfetched lead image and credit but not step images', () => {
  const c = parse();
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.deepEqual(c.fields.images.map(i => [i.sourceUrl, i.credit]), [['https://images.example.invalid/comet-bake.jpg', 'Photo by Orin Sky']]);
  assert.deepEqual(parse(recipe.replace('<img data-src=', '<img hidden data-src=')).fields.images, []);
});
test('Simply Recipes requires the exact HTTP(S) bare or www hostname', () => {
  assert.equal(parse(recipe, 'http://simplyrecipes.com/comet').parser.sourceAdapter, 'simply-recipes');
  for (const url of [null, 'https://simplyrecipes.com.example.invalid/', 'ftp://simplyrecipes.com/', 'https://www.seriouseats.com/']) refuses(recipe, url);
});
test('Simply Recipes cannot borrow core content from active blocks or navigation', () => {
  assert.equal(parse('<script>' + recipe + '</script><template>' + recipe + '</template>' + recipe).parser.sourceAdapter, 'simply-recipes');
  refuses('<nav>' + recipe + '</nav>');
});
test('Simply Recipes refuses hidden, malformed, duplicate or disconnected required regions', () => {
  for (const html of [recipe + recipe, recipe.replace('<main>', '<main hidden>'), '<div hidden>' + recipe, recipe.replace('</main>', '<div class="mm-recipes-steps">Unclosed'), recipe.replace('mm-recipes-steps__content', 'unknown-content'), recipe.replace('Combine flour and beans.', '<span hidden>Combine flour and beans.</span>')]) refuses(html);
});
test('Simply Recipes refuses duplicate, empty or unmatched card titles and optional metadata', () => {
  for (const html of [recipe.replace('Comet Bean Bake</h2>', '</h2>'), recipe.replace('</main>', '<h2 class="mm-recipes-details__title">Other card</h2>'), recipe.replace('</main>', '<h2 class="mm-recipes-details__title">Unclosed'), recipe.replace('15 mins</div>', '15 mins</div><div class="mm-recipes-details__value">20 mins</div>')]) refuses(html);
});
test('Simply Recipes refuses dropped marked content inside media, ads, tips or metadata', () => {
  for (const html of [recipe.replace('<div class="mntl-sc-block-adslot"></div>', '<div class="mntl-sc-block-adslot"><h3 class="mntl-sc-block-subheading">Lost action</h3></div>'), recipe.replace('<figcaption>Photo credit, not a step.</figcaption>', '<p class="mntl-sc-block-html">Lost action.</p>'), recipe.replace('<p>Keep the bowl chilled.</p>', '<p>Keep the bowl chilled.</p><p class="mm-recipes-structured-ingredients__list-heading">Lost ingredient group</p>'), recipe.replace('<div class="mm-recipes-details__content">', '<div class="mm-recipes-details__content"><p>Unmapped required card text.</p>')]) refuses(html);
});
test('Simply Recipes access shells and missing mandatory bodies cannot activate the adapter', () => {
  refuses('<main><h1>Subscribe to continue</h1></main>');
  refuses(recipe.replace('mm-recipes-structured-ingredients__list-item', 'unknown-row'));
});
test('Simply Recipes retains a complete heading-only instruction at its source position', () => {
  const c = parse(recipe.replace('<h3 class="mntl-sc-block-subheading"><span class="mntl-sc-block-subheading__text">Bake:</span></h3><p class="mntl-sc-block-html">Bake until golden.</p>', '<h3 class="mntl-sc-block-subheading"><span class="mntl-sc-block-subheading__text">Preheat to 400°F.</span></h3>'));
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.equal(c.fields.instructionSections[0].steps[1].text, 'Preheat to 400°F.');
  refuses(recipe.replace('<h3 class="mntl-sc-block-subheading"><span class="mntl-sc-block-subheading__text">Bake:</span></h3><p class="mntl-sc-block-html">Bake until golden.</p>', ''));
});
test('Simply Recipes refuses detached note markers and cooking text inside nutrition controls', () => {
  const note = '<div class="mm-recipes-details__note"><span class="mm-recipes-details__note-label">Note:</span><p>Refrigerate for 2 hours before serving.</p></div>';
  for (const html of [recipe.replace('</article>', note + '</article>'), recipe.replace('</article>', '<span class="mm-recipes-details__note-label">Note:</span></article>'), recipe.replace('<button>Jump to Nutrition Facts</button>', '<button>Jump to Nutrition Facts</button><p>Refrigerate for 2 hours before serving.</p>')]) refuses(html);
});
test('Simply Recipes requires title and metadata body as direct siblings, with inline-only title', () => {
  const html = recipe.replace('Comet Bean Bake</h2><div class="mm-recipes-details__content">', 'Comet Bean Bake<div class="mm-recipes-details__content">').replace('</div><div class="mm-recipes-details__note">', '</div></h2><div class="mm-recipes-details__note">');
  refuses(html);
  refuses(recipe.replace('Comet Bean Bake</h2>', 'Comet Bean Bake<p>Other card text.</p></h2>'));
});
test('Simply Recipes discarded wake controls and ads cannot contain extra visible cooking text', () => {
  for (const html of [recipe.replace('<button>Keep Screen Awake</button>', '<button>Keep Screen Awake</button><p>Additional required flour.</p>'), recipe.replace('<div class="mntl-sc-block-adslot"></div>', '<div class="mntl-sc-block-adslot"><p>Chill for 2 hours.</p></div>')]) refuses(html);
});
test('Simply Recipes complete-sentence step headings do not gain an extra colon', () => {
  const c = parse(recipe.replace('Mix:</span>', 'Preheat to 400°F.</span>'));
  assert.equal(c.parser.sourceAdapter, 'simply-recipes');
  assert.ok(c.fields.instructionSections[0].steps[0].text.startsWith('Preheat to 400°F. Combine flour and beans.'));
});
test('Simply Recipes recognizes singular and plural pinch measures without losing the ingredient name', () => {
  for (const unit of ['pinch', 'pinches']) {
    const c = parse(recipe.replace('<span>Butter or oi</span><span>l</span>, to taste', `1 ${unit} comet salt`));
    const item = c.fields.ingredientSections[1].items[0];
    assert.deepEqual([item.amount, item.unit, item.name], ['1', unit, 'comet salt']);
  }
});
