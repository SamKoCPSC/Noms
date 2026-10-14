import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import { parseRecipeClipboard } from '../recipe-import-parser.mjs';
import { validateImportCandidate } from '../validate-recipe-import-fixtures.mjs';

// Invented recipe text, with the sibling introduction/card ownership measured
// on both live pages. The old captures put everything inside the introduction.
for (const provider of ['bonappetit', 'epicurious']) {
  const fixture = await fs.readFile(new URL(`../../tests/fixtures/recipe-import/providers/${provider}-visible.html`, import.meta.url), 'utf8');
  const sourceUrl = `https://www.${provider}.com/recipe/invented-layout-check`;
  const parse = html => parseRecipeClipboard({ html, sourceUrl });
  const splitLayout = fixture
    .replace('<header ', '<article data-testid="RecipePageArticle"><header ')
    .replace('<div data-testid="recipe__main-content">', '<div data-testid="RecipePageContentBackground"><div data-testid="recipe__main-content">')
    .replace('<div data-testid="InfoSliceList">', '</div><div data-testid="InfoSliceList">')
    .replace('</div><footer>', '</div></article><footer>');
  const refuses = html => {
    let candidate;
    try { candidate = parse(html); }
    catch (error) { assert.equal(error.message, 'Clipboard input does not contain a recognizable recipe.'); return; }
    assert.notEqual(candidate.parser.tier, 'source_adapter');
  };

  test(`${provider}: live sibling introduction, metadata and card retain all fields`, () => {
    const candidate = parse(splitLayout);
    assert.deepEqual(candidate.fields, parse(fixture).fields);
    assert.equal(candidate.source.author, parse(fixture).source.author);
    assert.equal(candidate.parser.sourceAdapter, provider === 'bonappetit' ? 'bon-appetit' : provider);
    assert.deepEqual(validateImportCandidate(candidate), []);
  });
  test(`${provider}: sibling layout requires the explicit shared article and content boundaries`, () => {
    refuses(splitLayout.replace('data-testid="RecipePageArticle"', 'data-testid="UnrelatedArticle"'));
    refuses(splitLayout.replace('data-testid="RecipePageContentBackground"', 'data-testid="UnrelatedContent"'));
    refuses(splitLayout.replace('<article data-testid="RecipePageArticle">', '<div data-testid="RecipePageArticle">').replace('</article>', '</div>'));
    refuses(splitLayout.replace('</div><div data-testid="InfoSliceList">', '</div></div><div data-testid="InfoSliceList">').replace('</div></article><footer>', '</article><footer>'));
  });
  test(`${provider}: duplicate or nested content boundaries cannot select a partial recipe`, () => {
    refuses(splitLayout.replace('<div data-testid="RecipePageContentBackground">', '<div data-testid="RecipePageContentBackground"></div><div data-testid="RecipePageContentBackground">'));
    refuses(splitLayout.replace('<article data-testid="RecipePageArticle">', '<article data-testid="RecipePageArticle"></article><article data-testid="RecipePageArticle">'));
  });
  test(`${provider}: broad article shells may contain hidden non-recipe widgets, never hidden core text`, () => {
    const withWidget = splitLayout.replace('<div data-testid="RecipePageContentBackground">', '<div data-testid="RecipePageContentBackground"><aside hidden><p>Hidden unrelated widget</p></aside>');
    assert.deepEqual(parse(withWidget).fields, parse(fixture).fields);
    refuses(splitLayout.replace('<div data-testid="IngredientList">', '<div hidden data-testid="IngredientList">'));
  });
  test(`${provider}: hidden, navigation and missing sibling actions still refuse`, () => {
    refuses(splitLayout.replace('<div data-testid="RecipePageContentBackground">', '<div hidden data-testid="RecipePageContentBackground">'));
    refuses(splitLayout.replace('<div data-testid="RecipePageContentBackground">', '<nav><div data-testid="RecipePageContentBackground">').replace('</div></article>', '</div></nav></article>'));
    refuses(splitLayout.replace('data-testid="InstructionsWrapper"', 'data-testid="UnrelatedSteps"'));
  });
}
