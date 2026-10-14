import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import {parseRecipeClipboard} from '../recipe-import-parser.mjs';
import {validateImportCandidate,validateJsonSchema} from '../validate-recipe-import-fixtures.mjs';
const recipe=await fs.readFile(new URL('../../tests/fixtures/recipe-import/providers/sallys-visible.html',import.meta.url),'utf8');
const schema=JSON.parse(await fs.readFile(new URL('../../tests/fixtures/recipe-import/v1/import-candidate.schema.json',import.meta.url),'utf8'));
const parse=(html=recipe,sourceUrl='https://sallysbakingaddiction.com/comet-berry-bars/')=>parseRecipeClipboard({html,sourceUrl});
function refuses(html,url){let c;try{c=parse(html,url);}catch(e){assert.equal(e.message,'Clipboard input does not contain a recognizable recipe.');return;}assert.notEqual(c.parser.sourceAdapter,'sallys-baking-addiction');}
test('Sallys adapter extracts a coherent recipe, description and author deterministically',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.equal(c.fields.title.value,'Comet Berry Bars');assert.equal(c.fields.description.value,'A bright bake with tart berries.');assert.equal(c.source.author,'Rowan Vale');assert.deepEqual(validateImportCandidate(c),[]);assert.deepEqual(validateJsonSchema(c,schema),[]);assert.deepEqual(c,parse());
});
test('Sallys keeps every group and conversion and uses visible quantities rather than checkbox/data attributes',()=>{
 const c=parse();assert.deepEqual(c.fields.ingredientSections.map(s=>s.name),['Crust','Filling']);assert.deepEqual(c.fields.ingredientSections.flatMap(s=>s.items.map(i=>[i.amount,i.unit,i.name])),[['1 and 1/2','cups','(190g) cloud flour (spooned & leveled)'],['1','(8-ounce) package','moon butter, softened'],['1½ cups plus 2 Tablespoons',null,'comet berries'],['1–2','teaspoons','star zest'],[null,null,'salt, to taste']]);
});
test('Sallys joins complete procedure text without ads or supplementary notes',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.deepEqual(c.fields.instructionSections[0].steps.map(s=>s.text),['Make the crust: Mix the flour. Stir in butter.','Add berries and bake until golden.']);assert.deepEqual(c.fields.notes.map(n=>n.value),['Storage: Chill leftovers.','Use fresh zest.','Adapted from an invented notebook.']);assert.ok(!JSON.stringify(c.fields).includes("reader's"));
});
test('Sallys supports unnamed ingredient groups and the explicit legacy ingredient wrapper',()=>{
 const c=parse(recipe.replace('<h4>Crust</h4>','').replace('<h4>Filling</h4>','<h4>Filling</h4>'));
 assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.equal(c.fields.ingredientSections[0].name,null);
 const wrapped=parse(recipe.replace('<h4>Crust</h4><ul>','<h4>Crust</h4><div class="ERSIngredients"><ul>').replace('<h4>Filling</h4>','<h4 class="ERSClear">Filling</h4></div>'));
 assert.equal(wrapped.parser.sourceAdapter,'sallys-baking-addiction');assert.deepEqual(wrapped.fields.ingredientSections,c.fields.ingredientSections.map((s,i)=>({...s,name:i?'Filling':'Crust'})));
});
test('Sallys duration fields stay literal unless exact and supported',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.equal(c.fields.prepTime.value,15);assert.equal(c.fields.cookTime.value,30);assert.equal(c.fields.totalTime.value,null);assert.equal(c.fields.totalTime.raw,'45 minutes (plus chilling)');
 assert.equal(parse(recipe.replace('45 minutes (plus chilling)','1 hour, 15 minutes')).fields.totalTime.value,75);
 for(const v of ['About 1 hour','30–40 minutes','1 hour (includes chilling)','9007199254740993 hours'])assert.equal(parse(recipe.replace('45 minutes (plus chilling)',v)).fields.totalTime.value,null);
});
test('Sallys yield never interprets output counts or ranges as servings',()=>{
 assert.equal(parse().fields.yield.value.servings,null);for(const v of ['serves 6','6 servings'])assert.equal(parse(recipe.replace('12 bars',v)).fields.yield.value.servings,6);
 for(const v of ['12','serves 6–8','one loaf; 8 servings','about 6 servings','serves 0'])assert.equal(parse(recipe.replace('12 bars',v)).fields.yield.value.servings,null);
});
test('Sallys keeps unknown card metadata as unclassified review evidence',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.ok(c.unclassifiedLines.some(l=>l.raw==='Category: Dessert'));assert.ok(c.warnings.some(w=>w.code==='UNCLASSIFIED_PROVIDER_METADATA'));
});
test('Sallys collection titles never displace the complete embedded card and require review',()=>{
 const c=parse(recipe.replace('<h1>Comet Berry Bars</h1>','<h1>Twenty Comet Desserts</h1>'));assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.equal(c.fields.title.value,'Comet Berry Bars');assert.equal(c.status,'needs_review');assert.equal(c.competingCandidates[0].title,'Twenty Comet Desserts');assert.ok(c.warnings.some(w=>w.code==='COMPETING_RECIPE_REGION'));
});
test('Sallys lead images use safe lazy references only and exclude hidden nodes/ancestors',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.deepEqual(c.fields.images.map(i=>i.sourceUrl),['https://images.example.invalid/comet-bars.jpg']);
 for(const html of [recipe.replace('<img src=','<img hidden src='),recipe.replace('<div class="tasty-recipes-image">','<div class="tasty-recipes-image" aria-hidden="true">'),recipe.replace('https://images.example.invalid/comet-bars.jpg','javascript:alert(1)')])assert.deepEqual(parse(html).fields.images,[]);
});
test('Sallys activation requires exact HTTP(S) bare/www host',()=>{
 assert.equal(parse(recipe,'http://www.sallysbakingaddiction.com/comet').parser.sourceAdapter,'sallys-baking-addiction');for(const url of [null,'https://sallysbakingaddiction.com.example.invalid/','https://tasty.co/','ftp://sallysbakingaddiction.com/'])refuses(recipe,url);
});
test('Sallys ignores active bodies and does not borrow required markup from navigation',()=>{
 assert.equal(parse('<script>'+recipe+'</script><template>'+recipe+'</template>'+recipe).parser.sourceAdapter,'sallys-baking-addiction');refuses('<nav>'+recipe+'</nav>');
});
test('Sallys missing optional fields remain missing',()=>{
 const c=parse(recipe.replace(/<div class="tasty-recipes-description">[\s\S]*?<\/div><\/div>/u,'').replace(/<div class="tasty-recipes-details">[\s\S]*?<\/ul><\/div>/u,''));assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.equal(c.fields.description,null);assert.equal(c.fields.totalTime,null);assert.equal(c.fields.yield,null);assert.equal(c.source.author,null);
});
test('Sallys refuses competing, disconnected, unmatched or missing required blocks',()=>{
 for(const html of [recipe+recipe,recipe.replace('</main>','<main>Unclosed'),recipe.replace('tasty-recipes-instructions-body','unknown-body'),recipe.replace('tasty-recipes-title','unknown-title'),recipe.replace('</main>','<div class="tasty-recipes-ingredients-body">Unclosed'),recipe.replace('</main>','<div class="tasty-recipes"><h2>Another recipe</h2></div>')])refuses(html);
});
test('Sallys hidden required text and quantities refuse the adapter',()=>{
 for(const html of [recipe.replace('<main>','<main hidden>'),'<div hidden>'+recipe,recipe.replace('<strong>cloud flour</strong>','<strong hidden>cloud flour</strong>'),recipe.replace('<span data-amount="1">1</span> and','<span data-amount="1" style="display:none">1</span> and'),recipe.replace('<li id="instruction-step-2">','<li id="instruction-step-2" aria-hidden="true">')])refuses(html);
});
test('Sallys refuses unaccounted core text, empty rows and unknown structural wrappers',()=>{
 for(const html of [recipe.replace('<h4>Crust</h4>','Loose required text<h4>Crust</h4>'),recipe.replace('salt, to taste',''),recipe.replace('<h4>Crust</h4>','<h4></h4>'),recipe.replace('<h4>Crust</h4><ul>','<h4>Crust</h4><div class="unknown-wrapper"><ul>').replace('<h4>Filling</h4>','</div><h4>Filling</h4>'),recipe.replace('</ol><p></p></div></div>','</ol><p>Additional required action.</p></div></div>')])refuses(html);
});
test('Sallys ignored ads cannot hide marked or unmarked cooking content',()=>{
 for(const html of [recipe.replace('<div class="adthrive-ad"></div>','<div class="adthrive-ad">Additional action.</div>'),recipe.replace('<div class="adthrive-ad"></div>','<div class="adthrive-ad"><li id="instruction-step-extra">Lost action.</li></div>'),recipe.replace('<div class="adthrive-ad"></div>','<div class="adthrive-ad"><li data-tr-ingredient-checkbox="">Unclosed lost ingredient</div>')])refuses(html);
});
test('Sallys duplicate or malformed optional metadata refuses rather than disappearing',()=>{
 for(const html of [recipe.replace('15 minutes</span>','15 minutes</span><span class="tasty-recipes-prep-time">20 minutes</span>'),recipe.replace('15 minutes</span>','15 minutes</span><span class="tasty-recipes-prep-time">Unclosed'),recipe.replace('</header>','</header><p class="tasty-recipes-description-body">Unclosed'),recipe.replace('<span class="tasty-recipes-label">Prep Time:','<span class="tasty-recipes-label">Cook Time:')])refuses(html);
});
test('Sallys access-shell-only markup cannot activate an adapter',()=>{refuses('<main><h1>Subscribe to continue</h1></main>');});
test('Sallys refuses unaccounted text beside required bodies or their headings',()=>{
 for(const html of [recipe.replace('<div class="tasty-recipes-ingredients-body">','<p>2 cups additional flour</p><div class="tasty-recipes-ingredients-body">'),recipe.replace('<div class="tasty-recipes-instructions-body">','<p>Mandatory cooling action.</p><div class="tasty-recipes-instructions-body">'),recipe.replace('<h3>Instructions</h3>','<h3>Instructions</h3><p>Required chilling action.</p>'),recipe.replace('<h3>Ingredients</h3>','<h3>Ingredients</h3><p>Additional required ingredient.</p>'),recipe.replace('<h4>Crust</h4>','<h4>:</h4>')])refuses(html);
});
test('Sallys retains complete descriptions inside one leaf div wrapper',()=>{
 const c=parse(recipe.replace('<p><em>A bright bake with tart berries.</em></p>','<div role="textbox"><em>A bright bake with tart berries.</em></div>'));assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.equal(c.fields.description.value,'A bright bake with tart berries.');refuses(recipe.replace('<p><em>A bright bake with tart berries.</em></p>','<div><h2>Another recipe title</h2><p>A different recipe.</p></div>'));
});
test('Sallys ignores structurally empty editor placeholders between complete ingredient groups',()=>{
 const c=parse(recipe.replace('<h4>Filling</h4>','<h4></h4><ul class="wp-block-list"></ul><h4></h4><h4>Filling</h4>'));assert.equal(c.parser.sourceAdapter,'sallys-baking-addiction');assert.deepEqual(c.fields.ingredientSections,parse().fields.ingredientSections);assert.ok(c.warnings.some(w=>w.code==='EMPTY_PROVIDER_PLACEHOLDER'));
 const ulOnly=parse(recipe.replace('<h4>Filling</h4>','<ul class="wp-block-list"></ul><h4>Filling</h4>'));assert.equal(ulOnly.parser.sourceAdapter,'sallys-baking-addiction');
 for(const html of [recipe.replace('<h4>Filling</h4>','<ul class="wp-block-list"><li>Extra required ingredient.</li></ul><h4>Filling</h4>'),recipe.replace('<h4>Crust</h4>','<h4></h4>'),recipe.replace('<h4>Filling</h4>','<h4>Unfinished group</h4><ul class="wp-block-list"></ul><h4>Filling</h4>')])refuses(html);
});
