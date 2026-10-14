import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import { parseRecipeClipboard } from '../recipe-import-parser.mjs';
import { validateImportCandidate, validateJsonSchema } from '../validate-recipe-import-fixtures.mjs';
const schema=JSON.parse(await fs.readFile(new URL('../../tests/fixtures/recipe-import/v1/import-candidate.schema.json',import.meta.url),'utf8'));
// Invented recipe copy in the stable visible Food.com clipboard structure.
const row=(quantity,text)=>`<li style="display: contents"><span class="ingredient-quantity">${quantity}</span> <span class="ingredient-text">${text}</span></li>`;
const group=(kind,name)=>`<li><h4 class="${kind}-heading">${name}</h4></li>`;
const fact=(label,value)=>`<div class="facts__item"><svg aria-hidden="true"></svg><dt class="facts__label">${label}:</dt><dd class="facts__value">${value}</dd></div>`;
const adjust=value=>`<div class="adjust"><span contenteditable="true" class="value">${value}</span><div class="adjust__controls"><button aria-label="increase serving size">+</button><button>-</button></div></div>`;
const recipe=`<nav><h2>Ingredients</h2><ul><li>Navigation noise</li></ul></nav><main id="recipe" class="recipe layout">
<div class="layout__item title"><h1>Star Meadow Loaves</h1></div>
<div class="layout__item author-description"><div class="author-description"><div class="byline">Submitted by <a href="/user/1">Aster Vale</a></div><div class="recipe-description"><div title="Hidden attribute must not replace visible description"><div class="text truncated">A warm loaf with <em>crunch</em>.</div></div></div><button>Edit</button></div></div>
<div class="layout__item media"><div class="primary-image"><img src="https://images.example.invalid/loaf.jpg" alt="Loaves"><div class="attribution">photo by Orin Sky<img src="https://images.example.invalid/avatar.jpg"></div></div><div class="other-images"><img src="https://images.example.invalid/other.jpg"></div></div>
<div class="layout__item details"><div class="facts"><dl>${fact('Ready In','1hr 15mins')}${fact('Ingredients','4')}${fact('Yields',adjust('2')+' loaves')}${fact('Serves',adjust('6'))}</dl></div><button>Nutrition information</button></div>
<section class="layout__item ingredients"><div class="heading"><h2 id="ingredients" class="ingredients__title">ingredients</h2><button class="button--units">Units: <span>US</span></button></div><ul class="ingredient-list">${group('ingredient','Dough')}${row('5 <sup>1</sup>⁄<sub>2</sub> - 6','cups <a>moon flour</a>')}${row('1','(8 ounce) package <a>moon cheese</a>, softened')}${group('ingredient','Finish')}${row('','salt, to taste')}${row('1 <sup>3</sup>⁄<sub>4</sub>','tablespoons oil or 2 tablespoons butter')}</ul><div class="chicory-order-ingredients"></div></section>
<section class="layout__item directions"><div class="heading"><h2 id="directions" class="directions__title">directions</h2></div><ul class="direction-list"><li class="direction">Mix gently.</li>${group('direction','Bake')}<li class="direction">Bake until golden.</li><li class="direction">**Note: Cool before slicing.</li></ul></section>
<div class="layout__item reviewFeed"><p>Do not import this review.</p></div></main>`;
const parse=(html=recipe,sourceUrl='https://www.food.com/recipe/star-loaves-1')=>parseRecipeClipboard({html,plainText:null,sourceUrl});
function refuses(html,sourceUrl) {
 let c;try { c=parse(html,sourceUrl); } catch(error) {assert.equal(error.message,'Clipboard input does not contain a recognizable recipe.');return;}
 assert.notEqual(c.parser.sourceAdapter,'foodcom');
}
test('Food.com supports selection without a recipe URL',()=>{
 const c=parseRecipeClipboard({html:recipe,providerId:'foodcom'});
 assert.equal(c.parser.sourceAdapter,'foodcom');assert.equal(c.source.suppliedUrl,null);
 assert.deepEqual(validateImportCandidate(c),[]);assert.deepEqual(validateJsonSchema(c,schema),[]);
});

test('Food.com preserves all explicit ingredient and direction groups, including inline recipe notes',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'foodcom');
 assert.deepEqual(c.fields.ingredientSections.map(s=>s.name),['Dough','Finish']);
 assert.deepEqual(c.fields.ingredientSections.flatMap(s=>s.items).map(i=>[i.amount,i.unit,i.name]),[
  ['5 1/2 - 6','cups','moon flour'],['1','(8 ounce) package','moon cheese, softened'],[null,null,'salt, to taste'],['1 3/4','tablespoons','oil or 2 tablespoons butter']]);
 assert.deepEqual(c.fields.instructionSections.map(s=>[s.name,s.steps.map(t=>t.text)]),[[null,['Mix gently.']],['Bake',['Bake until golden.','**Note: Cool before slicing.']]]);
 assert.deepEqual(c.fields.notes,[]);assert.deepEqual(validateJsonSchema(c,schema),[]);assert.deepEqual(validateImportCandidate(c),[]);assert.deepEqual(parse(),c);
});
test('Food.com metadata keeps servings, output yield and total duration distinct',()=>{
 const c=parse();assert.equal(c.fields.title.value,'Star Meadow Loaves');assert.equal(c.source.author,'Aster Vale');
 assert.equal(c.fields.description.value,'A warm loaf with crunch.');assert.equal(c.fields.prepTime,null);assert.equal(c.fields.cookTime,null);assert.equal(c.fields.totalTime.value,75);
 assert.deepEqual(c.fields.yield.value,{display:'Yields: 2 loaves; Serves: 6',servings:6});
 assert.equal(c.fields.images.length,1);assert.equal(c.fields.images[0].credit,'Orin Sky');
 assert.ok(c.warnings.some(w=>w.code==='IMAGE_REFERENCE_REQUIRES_CONFIRMATION'));
});
test('Food.com range servings remain literal, and output counts never become servings',()=>{
 const c=parse(recipe.replace(adjust('6'),adjust('4-6')));assert.equal(c.fields.yield.value.servings,null);assert.equal(c.fields.yield.value.display,'Yields: 2 loaves; Serves: 4-6');
 const onlyYield=parse(recipe.replace(fact('Serves',adjust('6')),''));assert.equal(onlyYield.fields.yield.value.servings,null);assert.equal(onlyYield.fields.yield.value.display,'Yields: 2 loaves');
});
test('Food.com unfamiliar marked quantities are retained literally with low confidence',()=>{
 const c=parse(recipe.replace('5 <sup>1</sup>⁄<sub>2</sub> - 6','a handful of'));const i=c.fields.ingredientSections[0].items[0];assert.equal(i.amount,'a handful of');assert.equal(i.confidence,'low');assert.ok(c.warnings.some(w=>w.code==='UNRECOGNIZED_PROVIDER_MEASUREMENT'));
});
test('Food.com empty optional fields remain missing without borrowing reviews or recommendations',()=>{
 const c=parse(recipe.replace(/<div class="layout__item (?:author-description|media|details)">[\s\S]*?(?=<section)/u,''));
 assert.equal(c.parser.sourceAdapter,'foodcom');for(const key of ['description','prepTime','cookTime','totalTime','yield'])assert.equal(c.fields[key],null);assert.equal(c.source.author,null);assert.deepEqual(c.fields.images,[]);
});
test('Food.com exact HTTP hostname gate preserves generic fallback',()=>{
 assert.equal(parse(recipe,'http://food.com/recipe/1').parser.sourceAdapter,'foodcom');
 for(const url of [null,'https://food.com.example.invalid/','https://notfood.com/','https://example.invalid/?site=food.com','ftp://food.com/','https://chinese.food.com/'])refuses(recipe,url);
});
test('Food.com rejects competing, incomplete and unaccounted core structures',()=>{
 for(const html of [recipe+recipe,recipe.replace('id="recipe"','id="other"'),recipe.replace('<h1>Star Meadow Loaves</h1>','<h1></h1>'),recipe.replace('class="ingredient-text"','class="changed-text"'),recipe.replace('Bake until golden.</li>','Bake until golden.</li>Rest another hour.'),recipe.replace('Bake until golden.</li>','Bake until golden.<ul><li>Hidden nested step</li></ul></li>'),recipe.replace('</main>','<ul class="direction-list"><li class="direction">Extra step.</li></ul></main>'),recipe.replace('</main>','<ul class="ingredient-list">'+row('2','cup other flour')+'</ul></main>'),recipe.replace('</ul><div class="chicory','</ul><p>Another required ingredient.</p><div class="chicory'),recipe.replace('**Note: Cool before slicing.</li>','**Note: Cool before slicing.</li>'+group('direction','Empty group'))])refuses(html);
});
test('Food.com refuses hidden core content or navigation-contained recipe regions',()=>{
 for(const html of [`<div hidden>${recipe}</div>`,recipe.replace('<main ','<main hidden '),recipe.replace('<h1>','<h1 aria-hidden="true">'),recipe.replace('class="ingredient-quantity"','hidden class="ingredient-quantity"'),recipe.replace('Bake until golden.','<span hidden>Bake until golden.</span>'),recipe.replace('<section class="layout__item directions">','<nav><section class="layout__item directions">').replace('</section>\n<div class="layout__item reviewFeed">','</section></nav>\n<div class="layout__item reviewFeed">')])refuses(html);
});
test('Food.com optional hidden metadata cannot create authoritative values',()=>{
 for(const html of [recipe.replace('class="facts__item"','hidden class="facts__item"'),recipe.replace('class="byline"','hidden class="byline"'),recipe.replace('class="recipe-description"','hidden class="recipe-description"')])refuses(html);
 assert.deepEqual(parse(recipe.replace('class="primary-image"','hidden class="primary-image"')).fields.images,[]);
 assert.equal(parse(recipe.replace('class="attribution"','hidden class="attribution"')).fields.images[0].credit,null);
});
test('Food.com duplicate facts and malformed durations remain conservative',()=>{
 refuses(recipe.replace(fact('Ready In','1hr 15mins'),fact('Ready In','1hr 15mins')+fact('Ready In','2hrs')));
 const c=parse(recipe.replace('1hr 15mins','perhaps 2hrs or tomorrow'));assert.equal(c.fields.totalTime,null);assert.ok(c.unclassifiedLines.some(l=>l.raw.includes('perhaps')));
});
test('Food.com unknown quantity-row content and empty ingredient groups cannot silently disappear',()=>{
 refuses(recipe.replace('class="ingredient-text">cups','class="ingredient-text">cups').replace('</span> <span class="ingredient-text">','</span><span>Extra required text</span> <span class="ingredient-text">'));
 refuses(recipe.replace(group('ingredient','Finish'),group('ingredient','Empty')+group('ingredient','Finish')));
});
test('Food.com active/raw bodies stay opaque and unsafe images are never retained',()=>{
 assert.equal(parse(`<script>${recipe}</script><iframe>${recipe}</iframe><template>${recipe}</template>${recipe}`).parser.sourceAdapter,'foodcom');
 assert.deepEqual(parse(recipe.replace('https://images.example.invalid/loaf.jpg','javascript:alert(1)')).fields.images,[]);
});
test('Food.com compact duration tokens retain all components and unsupported values retain literal evidence',()=>{
 assert.equal(parse(recipe.replace('1hr 15mins','1hr15mins')).fields.totalTime.value,75);
 assert.equal(parse(recipe.replace('1hr 15mins','1.5hrs')).fields.totalTime.value,90);
 for(const value of ['1.5mins','9007199254740993days']) {
  const c=parse(recipe.replace('1hr 15mins',value));assert.equal(c.fields.totalTime,null);
  assert.ok(c.unclassifiedLines.some(l=>l.raw===`Ready In: ${value}`));
  assert.ok(c.warnings.some(w=>w.code==='UNCLASSIFIED_PROVIDER_METADATA'));
 }
});
test('Food.com refuses unclosed hidden ancestors and competing main roots with changed classes',()=>{
 for(const prefix of ['<div hidden>','<div aria-hidden="true">','<div style="display:none">','<div class="hidden">'])refuses(prefix+recipe);
 refuses(recipe+'<main id="recipe" class="changed"><h1>Other recipe</h1></main>');
 refuses(recipe+'<main id="recipe"><h1>Other recipe</h1>');
});

test('Food.com selects the uniquely marked primary recipe instead of an appended recipe',()=>{
 const secondary=recipe.replace('id="recipe"','').replaceAll('Star Meadow Loaves','Moon Orchard Stew').replaceAll('A warm loaf with <em>crunch</em>.','A different recipe description.');
 const expected=parse();
 for(const html of [recipe+secondary,secondary+recipe,recipe+secondary+secondary]) {
  const c=parse(html);assert.equal(c.parser.sourceAdapter,'foodcom');
  assert.deepEqual(c.fields,expected.fields);assert.deepEqual(c.source,expected.source);
  assert.ok(c.warnings.some(w=>w.code==='APPENDED_PROVIDER_RECIPES_IGNORED'));
  assert.equal(c.status,'needs_review');assert.deepEqual(validateJsonSchema(c,schema),[]);assert.deepEqual(validateImportCandidate(c),[]);
 }
});
test('Food.com primary selection refuses duplicate IDs, nested roots and malformed competing mains',()=>{
 const secondary=recipe.replace('id="recipe"','');
 for(const html of [recipe+recipe,secondary+secondary,recipe+secondary.replace('<main ','<main id="other" '),recipe.replace('</main>',secondary+'</main>'),recipe+'<main class="recipe">Unclosed appended recipe',recipe.replace('class="ingredient-text"','class="changed-text"')+secondary,`<div hidden>${recipe}</div>${secondary}`])refuses(html);
});
test('Food.com description excludes only its recognized expand/collapse controls',()=>{
 const description='A warm loaf with <em>crunch</em>.';
 for(const label of ['See More','Less']) {
  const c=parse(recipe.replace(description,description+`<label class="toggle">${label} <input type="checkbox"></label>`));
  assert.equal(c.fields.description.value,'A warm loaf with crunch.');
 }
 const c=parse(recipe.replace(description,'See More flavor.'+'<label class="toggle">Recipe information <input type="checkbox"></label>'));
 assert.equal(c.fields.description.value,'See More flavor. Recipe information');
 assert.equal(parse(recipe.replace(description,'<label class="toggle">See More <input type="checkbox"></label>')).fields.description,null);
});
test('Food.com refuses nesting among appended recipe mains as well as primary overlap',()=>{
 const secondary=recipe.replace('id="recipe"','');
 refuses(recipe+secondary.replace('</main>',secondary+'</main>'));
});
