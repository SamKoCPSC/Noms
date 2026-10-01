import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import { parseRecipeClipboard } from '../recipe-import-parser.mjs';
import { validateImportCandidate, validateJsonSchema } from '../validate-recipe-import-fixtures.mjs';
// All recipe copy is invented; only the observed visible component structure is modeled.
const recipe=await fs.readFile(new URL('../../tests/fixtures/recipe-import/providers/nyt-visible.html',import.meta.url),'utf8');
const schema=JSON.parse(await fs.readFile(new URL('../../tests/fixtures/recipe-import/v1/import-candidate.schema.json',import.meta.url),'utf8'));
const parse=(html=recipe,sourceUrl='https://cooking.nytimes.com/recipes/1-orbit-stew')=>parseRecipeClipboard({html,plainText:null,sourceUrl});
function refuses(html,sourceUrl) {
 let c;try{c=parse(html,sourceUrl);}catch(error){assert.equal(error.message,'Clipboard input does not contain a recognizable recipe.');return;}
 assert.notEqual(c.parser.sourceAdapter,'nyt-cooking');
}
test('NYT Cooking recovers the recipe description rather than the update timestamp, with author and adaptation credit',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'nyt-cooking');assert.equal(c.fields.title.value,'Orbit Garden Stew');
 assert.equal(c.fields.description.value,'A bright stew with gentle spice. Serve with moon bread.');
 assert.equal(c.source.author,'Aster Brook');assert.equal(c.source.attribution,'Recipe from Rowan Vale; Adapted by Aster Brook');
 assert.deepEqual(validateJsonSchema(c,schema),[]);assert.deepEqual(validateImportCandidate(c),[]);assert.deepEqual(parse(),c);
});
test('NYT Cooking preserves every explicit ingredient group and compound measurement literally',()=>{
 const c=parse();assert.deepEqual(c.fields.ingredientSections.map(s=>s.name),['For the stew','To finish']);
 assert.deepEqual(c.fields.ingredientSections.flatMap(s=>s.items).map(i=>[i.amount,i.unit,i.name]),[
 ['1 ½ cups/180 grams',null,'moon beans'],['2 cups minus 2 tablespoons',null,'cloud flour'],['1','(28-ounce) can','sky tomatoes'],['1 cup plus 2 tablespoons',null,'pale sugar'],['½','teaspoon','sea salt, to taste']]);
 assert.ok(c.warnings.some(w=>w.code==='SECTION_NAMES_NOT_PERSISTED'));
});
test('NYT Cooking removes only structural step labels and media, retaining complete paragraph text and recipe tips',()=>{
 const c=parse();assert.deepEqual(c.fields.instructionSections[0].steps.map(s=>s.text),['Combine the beans and tomatoes. Stir gently.','Simmer until tender.']);
 assert.deepEqual(c.fields.notes.map(n=>n.value),['Cool leftovers promptly.']);assert.deepEqual(c.fields.equipment,[]);
 assert.ok(!JSON.stringify(c.fields).includes('reader suggests'));assert.ok(!JSON.stringify(c.fields.instructionSections).includes('Media credit'));
});
test('NYT Cooking keeps labeled exact durations distinct and supports attached Unicode fractions',()=>{
 const c=parse();assert.equal(c.fields.totalTime.value,90);assert.equal(c.fields.prepTime.value,10);assert.equal(c.fields.cookTime.value,80);
 assert.equal(parse(recipe.replace('Total Time','Ready In')).fields.totalTime.value,90);
});
test('NYT Cooking preserves qualified, ranged and fractional-minute duration text without guessing exact minutes',()=>{
 for(const value of ['45 minutes, plus chilling','About 30 minutes','At least 4 hours','6½ to 8½ hours','1½ minutes','9007199254740993 hours']){
  const c=parse(recipe.replace('1½ hours',value));assert.equal(c.fields.totalTime.value,null);assert.equal(c.fields.totalTime.raw,'Total Time: '+value);
  assert.equal(c.fields.totalTime.confidence,'low');assert.ok(c.warnings.some(w=>w.code==='UNRESOLVED_PROVIDER_DURATION'));
 }
});
test('NYT Cooking numeric servings require an exact positive serving count, not a range or output count',()=>{
 assert.equal(parse().fields.yield.value.servings,4);
 for(const value of ['4 to 6 servings','About 4 servings','1½ dozen moon cookies','Two 12-inch pies','0 servings','9007199254740993 servings']){
  const c=parse(recipe.replace('4 servings',value));assert.equal(c.fields.yield.value.servings,null);assert.equal(c.fields.yield.value.display,'Yield: '+value);
 }
});
test('NYT Cooking retains ranges and optional unmeasured ingredients without inventing quantities',()=>{
 const c=parse(recipe.replace('½ teaspoon sea salt, to taste','1 to 2 teaspoons sea salt, to taste'));assert.equal(c.fields.ingredientSections[1].items[1].amount,'1 to 2');
 const missing=parse(recipe.replace('½ teaspoon sea salt, to taste','Sea salt, to taste')).fields.ingredientSections[1].items[1];assert.equal(missing.amount,null);assert.equal(missing.name,'Sea salt, to taste');
});
test('NYT Cooking absent optional metadata stays missing and cannot borrow reader notes or related recipes',()=>{
 const c=parse(recipe.replace(/<div class="recipeintro_topnote-block__invented">[\s\S]*?<\/div><button>Read More<\/button><\/div>/u,'').replace(/<div class="byline_byline__invented">[\s\S]*?<\/h2><\/div>/u,'').replace(/<div class="recipeintro_stats-block__invented">[\s\S]*?<\/dl><\/div>/u,'').replace(/<div class="ingredients_recipeYield__invented">[\s\S]*?<\/div>/u,''));
 assert.equal(c.parser.sourceAdapter,'nyt-cooking');assert.equal(c.fields.description,null);assert.equal(c.source.author,null);assert.equal(c.source.attribution,null);assert.equal(c.fields.yield,null);assert.equal(c.fields.totalTime,null);
});
test('NYT Cooking exact cooking hostname gate and component-name matching tolerate hash changes, not lookalike providers',()=>{
 assert.equal(parse(recipe.replaceAll('__invented','__newHash'),'http://cooking.nytimes.com/recipes/1').parser.sourceAdapter,'nyt-cooking');
 for(const url of [null,'https://nytimes.com/recipes/1','https://www.nytimes.com/','https://cooking.nytimes.com.example.invalid/','https://example.invalid/?source=cooking.nytimes.com','ftp://cooking.nytimes.com/'])refuses(recipe,url);
});
test('NYT Cooking refuses competing or incomplete recipe roots and unaccounted marked content',()=>{
 for(const html of [recipe+recipe,recipe.replace('</main>','<div class="recipe pagecontent_recipe-wrap__invented"><h1>Other recipe</h1></div></main>'),recipe.replace('</main>','<p class="ingredient_ingredient__invented">Extra required row</p></main>'),recipe.replace('ingredient_ingredient__invented','ingredient_changed__invented'),recipe.replace('Simmer until tender.</p>','Simmer until tender.</p><p>Extra required paragraph.</p>').replace('preparation_stepContent__invented','preparation_changed__invented'),recipe.replace('</main>','<main>Unclosed other main'),recipe.replace('<h1>Orbit Garden Stew</h1>','<h1></h1>')])refuses(html);
});
test('NYT Cooking refuses hidden required content, unclosed hidden ancestors and navigation-contained core regions',()=>{
 for(const html of ['<div hidden>'+recipe,'<div style="display:none">'+recipe,recipe.replace('<main>','<main aria-hidden="true">'),recipe.replace('<h1>','<h1 hidden>'),recipe.replace('<p class="ingredient_ingredient','<p hidden class="ingredient_ingredient'),recipe.replace('Simmer until tender.','<span hidden>Simmer until tender.</span>'),'<nav>'+recipe+'</nav>'])refuses(html);
});
test('NYT Cooking rejects unknown required text and malformed or reordered step labels rather than dropping them',()=>{
 for(const html of [recipe.replace('</ul><h3','</ul>Use another ingredient.<h3'),recipe.replace('moon beans</p>','moon beans</p>Unexpected row text'),recipe.replace('Step 2</div>','Step 3</div>'),recipe.replace('Simmer until tender.</p>','</p>'),recipe.replace('</ol></div>','</ol><p>One more required step.</p></div>'),recipe.replace('<h3 class="ingredientgroup_name__invented">To finish</h3>','<h3 class="ingredientgroup_name__invented"></h3>')])refuses(html);
});
test('NYT Cooking excludes hidden and related image references and retains unfetched explicit hero URLs with credit',()=>{
 const c=parse();assert.deepEqual(c.fields.images.map(i=>i.sourceUrl),['https://images.example.invalid/stew-small.jpg','https://images.example.invalid/stew.jpg']);assert.equal(c.fields.images[0].credit,'Photo by Orin Sky.');
 assert.ok(c.warnings.some(w=>w.code==='IMAGE_REFERENCE_REQUIRES_CONFIRMATION'));
 assert.deepEqual(parse(recipe.replace(/https:\/\/images.example.invalid\/stew(?:-small)?\.jpg/gu,'javascript:alert(1)')).fields.images,[]);
});
test('NYT Cooking raw active bodies stay opaque, and access shells cannot activate the adapter',()=>{
 assert.equal(parse('<script>'+recipe+'</script><template>'+recipe+'</template>'+recipe).parser.sourceAdapter,'nyt-cooking');
 refuses('<main><div class="recipe pagecontent_recipe-wrap__invented"><h1>Sign in to view this recipe</h1><p>Subscribe to continue</p></div></main>');
});
test('NYT Cooking duplicate duration labels, yield containers and author roles remain ambiguous',()=>{
 for(const html of [recipe.replace('<dt>Total Time</dt>','<dt>Total Time</dt><dd>20 minutes</dd><dt>Total Time</dt>'),recipe.replace('Adapted by <a>Aster Brook</a>','By <a>Aster Brook</a></h2><h2 class="byline_bylinePart__invented">By <a>Other Person</a>'),recipe.replace('<span>Yield:</span>','<span>Yield:</span><span>Yield:</span>')])refuses(html);
});
test('NYT Cooking refuses unmatched recipe roots and required markers inside ignored controls or media',()=>{
 for(const html of [
  recipe.replace('</main>','<div class="recipe pagecontent_recipe-wrap__invented">Other recipe</main>'),
  recipe.replace('<main>','<main><div class="recipe pagecontent_recipe-wrap__invented">'),
  recipe.replace('<button>Add ingredients to Grocery List</button>','<p class="ingredient_ingredient__invented">1 teaspoon missing spice</p><button>Add ingredients to Grocery List</button>'),
  recipe.replace('Media credit','<div class="preparation_stepContent__invented"><p>Add missing spice.</p></div>Media credit'),
 ])refuses(html);
});
test('NYT Cooking splits attached ingredient fractions and standalone additional units',()=>{
 const row=raw=>parse(recipe.replace('½ teaspoon sea salt, to taste',raw)).fields.ingredientSections[1].items[1];
 assert.deepEqual([row('1½ teaspoons sea salt').amount,row('1½ teaspoons sea salt').unit,row('1½ teaspoons sea salt').name],['1½','teaspoons','sea salt']);
 assert.deepEqual([row('2 sticks butter').amount,row('2 sticks butter').unit,row('2 sticks butter').name],['2','sticks','butter']);
 assert.equal(row('1½ cups/180 grams moon beans').amount,'1½ cups/180 grams');
});
test('NYT Cooking exact duration grammar does not accept dangling conjunctions or punctuation',()=>{
 for(const value of ['and 1 hour','1 hour and',',45 minutes','45 minutes,','1 hour,, 15 minutes']){
  assert.equal(parse(recipe.replace('1½ hours',value)).fields.totalTime.value,null);
 }
 for(const value of ['1 hour 15 minutes','1 hour and 15 minutes','1 hour, 15 minutes','1 hour, and 15 minutes']){
  assert.equal(parse(recipe.replace('1½ hours',value)).fields.totalTime.value,75);
 }
});
