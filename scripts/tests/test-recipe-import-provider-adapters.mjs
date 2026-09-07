import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { parseRecipeClipboard } from "../recipe-import-parser.mjs";
import {
  validateImportCandidate,
  validateJsonSchema,
} from "../validate-recipe-import-fixtures.mjs";

const schemaPath = fileURLToPath(
  new URL("../../tests/fixtures/recipe-import/v1/import-candidate.schema.json", import.meta.url),
);
const schema = JSON.parse(await fs.readFile(schemaPath, "utf8"));

const recipeTinHtml = `
<main>
  <h1>Long editorial page title</h1>
  <script>
    <div class="wprm-recipe-container">
      <h2 class="wprm-recipe-name">Injected recipe</h2>
      <h3 class="wprm-recipe-ingredients-header">Ingredients</h3>
      <ul class="wprm-recipe-ingredients"><li class="wprm-recipe-ingredient">1 bad token</li></ul>
      <h3 class="wprm-recipe-instructions-header">Instructions</h3>
      <ul class="wprm-recipe-instructions"><li class="wprm-recipe-instruction">Run script.</li></ul>
    </div>
  </script>
  <div id="wprm-recipe-container-42" class="wprm-recipe-container" data-recipe-id="42" data-servings="4">
    <h2 class="wprm-recipe-name">Moonlit Noodles</h2>
    <span class="wprm-recipe-author">Aster Vale</span>
    <span class="wprm-recipe-details wprm-recipe-prep_time wprm-recipe-prep_time-hours">1<span class="screen-reader-text"> hour</span></span>
    <span class="wprm-recipe-details wprm-recipe-prep_time wprm-recipe-prep_time-minutes">15<span class="screen-reader-text"> minutes</span></span>
    <span class="wprm-recipe-details wprm-recipe-cook_time wprm-recipe-cook_time-minutes">30<span class="screen-reader-text"> minutes</span></span>
    <span class="wprm-recipe-details wprm-recipe-total_time wprm-recipe-total_time-minutes">45<span class="screen-reader-text"> minutes</span></span>
    <span class="wprm-recipe-servings" data-servings="4">4</span>
    <span class="wprm-recipe-servings-unit">people</span>
    <div class="wprm-recipe-summary"><strong>Quick:</strong> a bright noodle supper.</div>
    <div class="wprm-recipe-ingredients-container">
      <h3 class="wprm-recipe-ingredients-header">Ingredients</h3>
      <div class="wprm-recipe-ingredient-group">
        <ul class="wprm-recipe-ingredients">
          <li class="wprm-recipe-ingredient">
            <span class="wprm-checkbox-container"><input type="checkbox"><span class="screen-reader-text">▢</span></span>
            <span class="wprm-recipe-ingredient-amount">2</span>
            <span class="wprm-recipe-ingredient-unit">tbsp</span>
            <span class="wprm-recipe-ingredient-name">starlight sauce</span>
            <span class="wprm-recipe-ingredient-notes">, divided</span>
          </li>
          <li class="wprm-recipe-ingredient">
            <span class="wprm-checkbox-container"><input type="checkbox"><span class="screen-reader-text">▢</span></span>
            <span class="wprm-recipe-ingredient-amount">1/2</span>
            <span class="wprm-recipe-ingredient-unit">cup</span>
            <span class="wprm-recipe-ingredient-name">moon broth</span>
            <span class="wprm-recipe-ingredient-notes">(Note 1)</span>
          </li>
          <li class="wprm-recipe-ingredient">
            <span class="wprm-recipe-ingredient-amount">400g/14oz</span>
            <span class="wprm-recipe-ingredient-unit">can</span>
            <span class="wprm-recipe-ingredient-name">star corn</span>
          </li>
        </ul>
      </div>
      <div class="wprm-recipe-ingredient-group">
        <h4 class="wprm-recipe-ingredient-group-name">Serving:</h4>
        <ul class="wprm-recipe-ingredients">
          <li class="wprm-recipe-ingredient">
            <span class="wprm-checkbox-container"><input type="checkbox"><span class="screen-reader-text">▢</span></span>
            <span class="wprm-recipe-ingredient-amount">300 g / 10 oz</span>
            <span class="wprm-recipe-ingredient-unit">oz</span>
            <span class="wprm-recipe-ingredient-name">comet noodles</span>
          </li>
        </ul>
      </div>
    </div>
    <div class="wprm-recipe-instructions-container">
      <h3 class="wprm-recipe-instructions-header">Instructions</h3>
      <div class="wprm-recipe-instruction-group">
        <ul class="wprm-recipe-instructions">
          <li class="wprm-recipe-instruction"><div class="wprm-recipe-instruction-text"><strong>Simmer:</strong> Warm the sauce.</div></li>
        </ul>
      </div>
      <div class="wprm-recipe-instruction-group">
        <h4 class="wprm-recipe-instruction-group-name">Finish:</h4>
        <ul class="wprm-recipe-instructions">
          <li class="wprm-recipe-instruction"><div class="wprm-recipe-instruction-text">Fold in the noodles.</div></li>
        </ul>
      </div>
    </div>
  </div>
  <figure>
    <img src="https://images.example.invalid/moonlit-noodles.webp" alt="Moonlit noodles in a bowl">
    <figcaption>Photo by Aster Vale</figcaption>
  </figure>
</main>`;

const tastyHtml = `
<main>
  <script>
    <h1 class="recipe-name">Injected Moon Pie</h1>
    <div class="ingredients-prep">
      <div class="ingredients__section"><ul><li class="ingredient">1 bad token</li></ul></div>
    </div>
    <div class="preparation"><ol class="prep-steps"><li>Run script.</li></ol></div>
  </script>
  <div class="recipe-header--content">
    <h1 class="recipe-name extra-bold">Starlight Hand Pies</h1>
    <p class="description">Flaky hand pies with a bright lunar filling.</p>
    <div class="recipe-attribution">
      <div class="byline extra-bold">Aster Vale</div>
      <div class="role">Tasty Team</div>
    </div>
    <div class="desktop-cooktimes">
      <div class="recipe-time-container">
        <div class="recipe-time">
          <h5>Total Time</h5>
          <p class="xs-hide md-block">1 hr 15 min</p>
          <p class="md-hide">1 hr 15 min</p>
        </div>
        <div class="recipe-time">
          <h5>Prep Time</h5>
          <p class="xs-hide md-block">25 minutes</p>
          <p class="md-hide">25 min</p>
        </div>
        <div class="recipe-time">
          <h5>Cook Time</h5>
          <p class="xs-hide md-block">50 minutes</p>
          <p class="md-hide">50 min</p>
        </div>
      </div>
    </div>
  </div>
  <div class="video-thumbnail">
    <img src="https://images.example.invalid/starlight-hand-pies.webp" alt="Starlight hand pies">
  </div>
  <div id="recipe" class="two-columns-recipe">
    <div id="recipe-ingredients" class="ingredients-prep recipe-ingredients">
      <div>
        <h2>Ingredients</h2>
        <p class="servings-display">for 6 servings</p>
        <div class="ingredients__section">
          <p class="ingredient-section-name">Dough:</p>
          <ul>
            <li class="ingredient">1 ½ cups moon flour</li>
            <li class="ingredient">2 tbsp comet butter, chilled</li>
          </ul>
        </div>
        <div class="ingredients__section">
          <p class="ingredient-section-name">Filling:</p>
          <ul>
            <li class="ingredient">3 star pears, diced</li>
          </ul>
        </div>
      </div>
    </div>
    <div class="preparation">
      <h2>Preparation</h2>
      <ol class="prep-steps">
        <li class="xs-mb2">Mix the dough.</li>
        <li class="xs-mb2">Read the <a href="/helpful-baking-tip">helpful baking tip</a>, then fill the pies.</li>
        <li class="xs-mb2">Enjoy!</li>
        <li class="xs-mb2">Cook smarter with the Tasty app. <a href="https://tasty.co/download?ct=prepsteps">Download now</a>.</li>
      </ol>
    </div>
  </div>
  <div class="video-thumbnail">
    <img src="https://images.example.invalid/starlight-hand-pies.webp" alt="Starlight hand pies">
  </div>
</main>`;

test("Tasty adapter preserves all visible recipe groups and excludes its app promotion", () => {
  const candidate = parseRecipeClipboard({
    html: tastyHtml,
    plainText: null,
    sourceUrl: "https://tasty.co/recipe/starlight-hand-pies",
  });

  assert.deepEqual(candidate.parser, {
    id: "tasty-visible-recipe",
    version: "1",
    tier: "source_adapter",
    sourceAdapter: "tasty",
  });
  assert.equal(candidate.source.provider, "Tasty");
  assert.deepEqual(candidate.source.provenance.provider, [{
    representation: "source_url",
    locator: "hostname tasty.co",
  }]);
  assert.equal(candidate.source.author, "Aster Vale");
  assert.equal(candidate.fields.title.value, "Starlight Hand Pies");
  assert.equal(candidate.fields.description.value, "Flaky hand pies with a bright lunar filling.");
  assert.equal(candidate.fields.prepTime.value, 25);
  assert.equal(candidate.fields.cookTime.value, 50);
  assert.equal(candidate.fields.totalTime.value, 75);
  assert.deepEqual(candidate.fields.yield.value, { display: "for 6 servings", servings: 6 });
  assert.deepEqual(
    candidate.fields.ingredientSections.map(({ name, items }) => ({
      name,
      items: items.map(({ raw, amount, unit, name: ingredientName }) => ({
        raw,
        amount,
        unit,
        name: ingredientName,
      })),
    })),
    [
      {
        name: "Dough:",
        items: [
          { raw: "1 ½ cups moon flour", amount: "1 ½", unit: "cups", name: "moon flour" },
          { raw: "2 tbsp comet butter, chilled", amount: "2", unit: "tbsp", name: "comet butter, chilled" },
        ],
      },
      {
        name: "Filling:",
        items: [
          { raw: "3 star pears, diced", amount: "3", unit: null, name: "star pears, diced" },
        ],
      },
    ],
  );
  assert.deepEqual(
    candidate.fields.instructionSections.flatMap(({ steps }) => steps.map(({ text }) => text)),
    ["Mix the dough.", "Read the helpful baking tip, then fill the pies.", "Enjoy!"],
  );
  assert.deepEqual(
    candidate.fields.images.map(({ sourceUrl, alt, credit }) => ({ sourceUrl, alt, credit })),
    [{
      sourceUrl: "https://images.example.invalid/starlight-hand-pies.webp",
      alt: "Starlight hand pies",
      credit: null,
    }],
  );
  assert.equal(candidate.status, "needs_review");
  assert(candidate.warnings.some(({ code }) => code === "SECTION_NAMES_NOT_PERSISTED"));
  assert(candidate.warnings.some(({ code }) => code === "IMAGE_REFERENCE_REQUIRES_CONFIRMATION"));
  assert(candidate.discardedContent.some(({ kind }) => kind === "script"));
  assert.doesNotMatch(JSON.stringify(candidate), /Injected Moon Pie|bad token|Run script|Cook smarter/u);
  assert.deepEqual(validateJsonSchema(candidate, schema), []);
  assert.deepEqual(validateImportCandidate(candidate), []);
});

test("Tasty adapter maps its compact unlabeled duration to total time", () => {
  const candidate = parseRecipeClipboard({
    html: tastyHtml.replace(
      /<div class="desktop-cooktimes">[\s\S]*?<\/div>\s*<\/div>\s*<\/div>/u,
      `<div class="desktop-cooktimes">
        <div class="recipe-time-container"><p>4 hr 10 min</p></div>
      </div>`,
    ),
    plainText: null,
    sourceUrl: "https://www.tasty.co/recipe/starlight-hand-pies",
  });

  assert.equal(candidate.parser.id, "tasty-visible-recipe");
  assert.equal(candidate.fields.prepTime, null);
  assert.equal(candidate.fields.cookTime, null);
  assert.equal(candidate.fields.totalTime.value, 250);
});

test("Tasty adapter preserves a compound measurement as one amount expression", () => {
  const candidate = parseRecipeClipboard({
    html: tastyHtml.replace(
      "3 star pears, diced",
      "3 tablespoons, plus 4 teaspoons curry powder, divided",
    ),
    plainText: null,
    sourceUrl: "https://tasty.co/recipe/starlight-hand-pies",
  });

  const compoundIngredient = candidate.fields.ingredientSections[1].items[0];
  assert.deepEqual(
    {
      raw: compoundIngredient.raw,
      amount: compoundIngredient.amount,
      unit: compoundIngredient.unit,
      name: compoundIngredient.name,
    },
    {
      raw: "3 tablespoons, plus 4 teaspoons curry powder, divided",
      amount: "3 tablespoons, plus 4 teaspoons",
      unit: null,
      name: "curry powder, divided",
    },
  );
  assert.deepEqual(validateJsonSchema(candidate, schema), []);
  assert.deepEqual(validateImportCandidate(candidate), []);
});

test("Tasty provider detection rejects lookalike hostnames", () => {
  const candidate = parseRecipeClipboard({
    html: tastyHtml,
    plainText: null,
    sourceUrl: "https://tasty.co.evil.example/recipe/starlight-hand-pies",
  });

  assert.equal(candidate.parser.tier, "generic_html");
  assert.equal(candidate.parser.sourceAdapter, null);
  assert.equal(candidate.source.provider, null);
});

test("Tasty URLs fall back safely when its visible recipe structure is incomplete", () => {
  const candidate = parseRecipeClipboard({
    html: `
      <main>
        <h1>Archived Star Soup</h1>
        <h2>Ingredients</h2><ul><li>2 cups moon broth</li></ul>
        <h2>Preparation</h2><ol><li>Simmer gently.</li></ol>
      </main>`,
    plainText: null,
    sourceUrl: "https://tasty.co/recipe/archived-star-soup",
  });

  assert.equal(candidate.parser.tier, "generic_html");
  assert.equal(candidate.parser.sourceAdapter, null);
  assert.equal(candidate.fields.title.value, "Archived Star Soup");
});

test("Tasty adapter does not silently choose between multiple visible recipes", () => {
  const secondRecipe = `
    <h1 class="recipe-name">Moon Soup</h1>
    <div class="ingredients-prep">
      <h2>Ingredients</h2>
      <div class="ingredients__section"><ul><li class="ingredient">moon broth</li></ul></div>
    </div>
    <div class="preparation"><h2>Preparation</h2><ol class="prep-steps"><li>Simmer.</li></ol></div>`;
  const candidate = parseRecipeClipboard({
    html: tastyHtml.replace("</main>", `${secondRecipe}</main>`),
    plainText: null,
    sourceUrl: "https://tasty.co/recipe/two-visible-recipes",
  });

  assert.equal(candidate.parser.tier, "generic_html");
  assert.equal(candidate.parser.sourceAdapter, null);
});

test("RecipeTinEats adapter extracts complete WPRM groups and structured fields", () => {
  const candidate = parseRecipeClipboard({
    html: recipeTinHtml,
    plainText: null,
    sourceUrl: "https://www.recipetineats.com/moonlit-noodles/#recipe",
  });

  assert.deepEqual(candidate.parser, {
    id: "recipetineats-wprm",
    version: "1",
    tier: "source_adapter",
    sourceAdapter: "recipetineats",
  });
  assert.equal(candidate.source.provider, "RecipeTinEats");
  assert.deepEqual(candidate.source.provenance.provider, [{
    representation: "source_url",
    locator: "hostname recipetineats.com",
  }]);
  assert.equal(candidate.source.author, "Aster Vale");
  assert.equal(candidate.fields.title.value, "Moonlit Noodles");
  assert.equal(candidate.fields.description.value, "Quick: a bright noodle supper.");
  assert.equal(candidate.fields.prepTime.value, 75);
  assert.equal(candidate.fields.cookTime.value, 30);
  assert.equal(candidate.fields.totalTime.value, 45);
  assert.deepEqual(candidate.fields.yield.value, { display: "4 people", servings: 4 });

  assert.deepEqual(
    candidate.fields.ingredientSections.map(({ name, items }) => ({
      name,
      items: items.map(({ raw, amount, unit, name: ingredientName }) => ({
        raw,
        amount,
        unit,
        name: ingredientName,
      })),
    })),
    [
      {
        name: null,
        items: [
          {
            raw: "2 tbsp starlight sauce, divided",
            amount: "2",
            unit: "tbsp",
            name: "starlight sauce, divided",
          },
          {
            raw: "1/2 cup moon broth (Note 1)",
            amount: "1/2",
            unit: "cup",
            name: "moon broth (Note 1)",
          },
          {
            raw: "400g/14oz can star corn",
            amount: "400g/14oz",
            unit: "can",
            name: "star corn",
          },
        ],
      },
      {
        name: "Serving:",
        items: [{
          raw: "300 g / 10 oz comet noodles",
          amount: "300 g / 10 oz",
          unit: null,
          name: "comet noodles",
        }],
      },
    ],
  );
  assert.deepEqual(
    candidate.fields.instructionSections.map(({ name, steps }) => ({
      name,
      steps: steps.map(({ text }) => text),
    })),
    [
      { name: null, steps: ["Simmer: Warm the sauce."] },
      { name: "Finish:", steps: ["Fold in the noodles."] },
    ],
  );
  assert.deepEqual(
    candidate.fields.images.map(({ sourceUrl, alt, credit }) => ({ sourceUrl, alt, credit })),
    [{
      sourceUrl: "https://images.example.invalid/moonlit-noodles.webp",
      alt: "Moonlit noodles in a bowl",
      credit: "Photo by Aster Vale",
    }],
  );
  assert.equal(candidate.source.attribution, "Photo by Aster Vale");
  assert.equal(candidate.status, "needs_review");
  assert(candidate.warnings.some(({ code }) => code === "SECTION_NAMES_NOT_PERSISTED"));
  assert(candidate.warnings.some(({ code }) => code === "IMAGE_REFERENCE_REQUIRES_CONFIRMATION"));
  assert(candidate.discardedContent.some(({ kind }) => kind === "script"));
  assert.doesNotMatch(JSON.stringify(candidate), /Injected recipe|bad token|Run script/u);

  assert.deepEqual(validateJsonSchema(candidate, schema), []);
  assert.deepEqual(validateImportCandidate(candidate), []);
});

test("RecipeTinEats adapter ignores empty instruction placeholders when complete groups remain", () => {
  const candidate = parseRecipeClipboard({
    html: `
      <main>
        <div class="wprm-recipe-container">
          <h2 class="wprm-recipe-name">Moon Soup</h2>
          <div class="wprm-recipe-ingredients-container">
            <div class="wprm-recipe-ingredient-group">
              <ul class="wprm-recipe-ingredients">
                <li class="wprm-recipe-ingredient">
                  <span class="wprm-recipe-ingredient-name">moon broth</span>
                </li>
              </ul>
            </div>
          </div>
          <div class="wprm-recipe-instructions-container">
            <div class="wprm-recipe-instruction-group">
              <ul class="wprm-recipe-instructions">
                <li class="wprm-recipe-instruction">
                  <div class="wprm-recipe-instruction-text">Warm the broth.</div>
                </li>
              </ul>
            </div>
            <div class="wprm-recipe-instruction-group">
              <h4 class="wprm-recipe-instruction-group-name">FULL RECIPE</h4>
              <ul class="wprm-recipe-instructions"></ul>
            </div>
            <div class="wprm-recipe-instruction-group">
              <ul class="wprm-recipe-instructions">
                <li class="wprm-recipe-instruction">
                  <div class="wprm-recipe-instruction-text">Serve the soup.</div>
                </li>
              </ul>
            </div>
          </div>
        </div>
      </main>`,
    plainText: null,
    sourceUrl: "https://www.recipetineats.com/moon-soup/#recipe",
  });

  assert.equal(candidate.parser.id, "recipetineats-wprm");
  assert.deepEqual(
    candidate.fields.instructionSections.map(({ name, steps }) => ({
      name,
      steps: steps.map(({ text }) => text),
    })),
    [
      { name: null, steps: ["Warm the broth."] },
      { name: null, steps: ["Serve the soup."] },
    ],
  );
  assert.equal(candidate.status, "needs_review");
  assert.deepEqual(candidate.warnings, [{
    code: "EMPTY_PROVIDER_SECTION_IGNORED",
    severity: "warning",
    path: "fields.instructionSections",
    message: "Empty provider instruction sections were ignored while complete sections were retained.",
  }]);
  assert.deepEqual(validateJsonSchema(candidate, schema), []);
  assert.deepEqual(validateImportCandidate(candidate), []);
});

test("RecipeTinEats adapter rejects a card with only empty instruction groups", () => {
  assert.throws(
    () => parseRecipeClipboard({
      html: `
        <main>
          <div class="wprm-recipe-container">
            <h2 class="wprm-recipe-name">Empty Moon Soup</h2>
            <div class="wprm-recipe-ingredients-container">
              <div class="wprm-recipe-ingredient-group">
                <ul class="wprm-recipe-ingredients">
                  <li class="wprm-recipe-ingredient">
                    <span class="wprm-recipe-ingredient-name">moon broth</span>
                  </li>
                </ul>
              </div>
            </div>
            <div class="wprm-recipe-instructions-container">
              <div class="wprm-recipe-instruction-group">
                <h4 class="wprm-recipe-instruction-group-name">FULL RECIPE</h4>
                <ul class="wprm-recipe-instructions"></ul>
              </div>
            </div>
          </div>
        </main>`,
      plainText: null,
      sourceUrl: "https://www.recipetineats.com/empty-moon-soup/",
    }),
    /does not contain a recognizable recipe/u,
  );
});

test("RecipeTinEats adapter rejects non-empty instruction groups without structured steps", () => {
  assert.throws(
    () => parseRecipeClipboard({
      html: `
        <main>
          <div class="wprm-recipe-container">
            <h2 class="wprm-recipe-name">Malformed Moon Soup</h2>
            <div class="wprm-recipe-ingredients-container">
              <div class="wprm-recipe-ingredient-group">
                <ul class="wprm-recipe-ingredients">
                  <li class="wprm-recipe-ingredient">
                    <span class="wprm-recipe-ingredient-name">moon broth</span>
                  </li>
                </ul>
              </div>
            </div>
            <div class="wprm-recipe-instructions-container">
              <div class="wprm-recipe-instruction-group">
                <ul class="wprm-recipe-instructions">
                  <li class="wprm-recipe-instruction">Unstructured instruction text.</li>
                </ul>
              </div>
            </div>
          </div>
        </main>`,
      plainText: null,
      sourceUrl: "https://www.recipetineats.com/malformed-moon-soup/",
    }),
    /does not contain a recognizable recipe/u,
  );
});

test("provider detection requires the exact RecipeTinEats hostname", () => {
  const candidate = parseRecipeClipboard({
    html: recipeTinHtml,
    plainText: null,
    sourceUrl: "https://recipetineats.com.evil.example/moonlit-noodles/",
  });

  assert.equal(candidate.parser.tier, "generic_html");
  assert.equal(candidate.parser.sourceAdapter, null);
  assert.equal(candidate.source.provider, null);
});

test("RecipeTinEats URLs fall back safely when WPRM markup is absent", () => {
  const candidate = parseRecipeClipboard({
    html: `
      <article>
        <h1>Archived Moon Soup</h1>
        <h2>Ingredients</h2><ul><li>2 cups moon broth</li></ul>
        <h2>Instructions</h2><ol><li>Simmer gently.</li></ol>
      </article>`,
    plainText: null,
    sourceUrl: "https://www.recipetineats.com/archived-moon-soup/",
  });

  assert.equal(candidate.parser.tier, "generic_html");
  assert.equal(candidate.parser.sourceAdapter, null);
  assert.equal(candidate.source.provider, null);
  assert.equal(candidate.fields.title.value, "Archived Moon Soup");
});

test("RecipeTinEats adapter does not silently choose between multiple WPRM cards", () => {
  const secondCard = `
    <div class="wprm-recipe-container">
      <h2 class="wprm-recipe-name">Starlit Rice</h2>
      <div class="wprm-recipe-ingredients-container">
        <h3>Ingredients</h3><ul class="wprm-recipe-ingredients">
          <li class="wprm-recipe-ingredient"><span class="wprm-recipe-ingredient-name">moon rice</span></li>
        </ul>
      </div>
      <div class="wprm-recipe-instructions-container">
        <h3>Instructions</h3><ul class="wprm-recipe-instructions">
          <li class="wprm-recipe-instruction"><div class="wprm-recipe-instruction-text">Steam.</div></li>
        </ul>
      </div>
    </div>`;
  const candidate = parseRecipeClipboard({
    html: recipeTinHtml.replace("</main>", `${secondCard}</main>`),
    plainText: null,
    sourceUrl: "https://www.recipetineats.com/two-recipes/",
  });

  assert.equal(candidate.parser.tier, "generic_html");
  assert.equal(candidate.parser.sourceAdapter, null);
  assert.equal(candidate.status, "needs_review");
  assert(candidate.warnings.some(({ code }) => code === "COMPETING_RECIPE_REGION"));
});
