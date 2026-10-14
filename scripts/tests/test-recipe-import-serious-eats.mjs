import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import {parseRecipeClipboard} from '../recipe-import-parser.mjs';
import {validateImportCandidate,validateJsonSchema} from '../validate-recipe-import-fixtures.mjs';
const recipe=await fs.readFile(new URL('../../tests/fixtures/recipe-import/providers/serious-eats-visible.html',import.meta.url),'utf8');
const schema=JSON.parse(await fs.readFile(new URL('../../tests/fixtures/recipe-import/v1/import-candidate.schema.json',import.meta.url),'utf8'));
const parse=(html=recipe,sourceUrl='https://www.seriouseats.com/comet-bean-bake')=>parseRecipeClipboard({html,sourceUrl});
function refuses(html,sourceUrl){let c;try{c=parse(html,sourceUrl);}catch(e){assert.equal(e.message,'Clipboard input does not contain a recognizable recipe.');return;}assert.notEqual(c.parser.sourceAdapter,'serious-eats');}
test('Serious Eats extracts one coherent recipe, description and author rather than editorial or reviewer text',()=>{
 const c=parse();assert.equal(c.parser.sourceAdapter,'serious-eats');assert.equal(c.fields.title.value,'Comet Bean Bake');assert.equal(c.fields.description.value,'A cozy bake with bright herbs.');assert.equal(c.source.author,'Rowan Vale');assert.deepEqual(validateImportCandidate(c),[]);assert.deepEqual(validateJsonSchema(c,schema),[]);assert.deepEqual(c,parse());
});
test('Serious Eats retains every grouped row, multi-span names and conversion suffix without merging quantities',()=>{
 const c=parse();assert.deepEqual(c.fields.ingredientSections.map(s=>s.name),['For the filling','To finish']);
 assert.deepEqual(c.fields.ingredientSections.flatMap(s=>s.items.map(i=>[i.amount,i.unit,i.name])),[['12 1/2','ounces','assorted moon beans and lentils (355 g), drained'],['1½ cups plus 2 tablespoons',null,'cloud flour'],['1','(28-ounce) can','sky tomatoes'],[null,null,'Sea salt, to taste']]);
});
test('Serious Eats joins procedure paragraphs without image credits, equipment, storage or reader notes',()=>{
 const c=parse();assert.deepEqual(c.fields.instructionSections.flatMap(s=>s.steps.map(s=>s.text)),['For the filling: Combine the beans. Stir in tomatoes.','Bake until golden.']);assert.deepEqual(c.fields.equipment.map(e=>e.value),['Large baking dish, sturdy spoon']);assert.deepEqual(c.fields.notes.map(n=>n.value),['Notes: Drain the beans well.','Make-Ahead and Storage: Chill covered leftovers.']);assert.ok(!JSON.stringify(c.fields).includes('Reader suggests'));assert.ok(!JSON.stringify(c.fields).includes('Unrelated article'));
});
test('Serious Eats recovers exact labeled durations and preserves unknown metadata separately',()=>{
 const c=parse();assert.equal(c.fields.prepTime.value,15);assert.equal(c.fields.cookTime.value,65);assert.equal(c.fields.totalTime.value,80);assert.ok(c.unclassifiedLines.some(l=>l.raw==='Rest Time: 10 mins'));
 for(const value of ['45 mins, plus cooling','30 to 40 mins','About 1 hr','and 1 hr','1 hr and','9007199254740993 hrs']){const c=parse(recipe.replace('1 hr 20 mins',value));assert.equal(c.fields.totalTime.value,null);assert.equal(c.fields.totalTime.raw,'Total Time: '+value);}
});
test('Serious Eats servings stay literal for ranges and output counts',()=>{
 const c=parse();assert.equal(c.fields.yield.value.servings,null);assert.equal(c.fields.yield.value.display,'Servings: 4 to 6; Yield: One comet-sized casserole');
 for(const value of ['4','4 servings'])assert.equal(parse(recipe.replace('4 to 6',value)).fields.yield.value.servings,4);
 for(const value of ['32 cookies','About 4','0','9007199254740993'])assert.equal(parse(recipe.replace('4 to 6',value)).fields.yield.value.servings,null);
});
test('Serious Eats preserves an explicitly headed phase inside a step and a single-item nested procedure',()=>{
 const phase=parse(recipe.replace('<p class="mntl-sc-block-html">Bake until golden.</p>','<span class="heading-toc"></span><h2 class="mntl-sc-block-heading">Day 2</h2><p class="mntl-sc-block-html">Bake until golden.</p>'));
 assert.equal(phase.fields.instructionSections.at(-1).name,'Day 2');assert.equal(phase.fields.instructionSections.at(-1).steps[0].text,'Bake until golden.');
 const nested=parse(recipe.replace('<p class="mntl-sc-block-html">Bake until golden.</p>','<ol class="mntl-sc-block-html"><li>Bake until golden.</li></ol>'));assert.equal(nested.fields.instructionSections[0].steps[1].text,'Bake until golden.');assert.ok(nested.warnings.some(w=>w.code==='NESTED_SINGLE_STEP_FLATTENED'));
});
test('Serious Eats recognizes explicit supplements accidentally nested in the final step without losing that step',()=>{
 const c=parse(recipe.replace('Bake until golden.</p></li></ol>','Bake until golden.</p><h2 class="mntl-sc-block-heading">Notes</h2><p class="mntl-sc-block-html">Extra recipe tip.</p></li></ol>'));
 assert.equal(c.fields.instructionSections[0].steps[1].text,'Bake until golden.');assert.ok(c.fields.notes.some(n=>n.value==='Notes: Extra recipe tip.'));
});
test('Serious Eats uses only safe unfetched primary image references',()=>{
 const c=parse();assert.deepEqual(c.fields.images.map(i=>i.sourceUrl),['https://images.example.invalid/bake.jpg']);assert.equal(c.fields.images[0].credit,'Photo by Orin Sky');assert.deepEqual(parse(recipe.replace('https://images.example.invalid/bake.jpg','javascript:alert(1)')).fields.images,[]);
});
test('Serious Eats exact-host activation rejects lookalikes and supports bare/www hostnames',()=>{
 assert.equal(parse(recipe,'http://seriouseats.com/comet').parser.sourceAdapter,'serious-eats');for(const url of [null,'https://seriouseats.com.example.invalid/','https://allrecipes.com/','https://example.invalid/?site=seriouseats.com','ftp://seriouseats.com/'])refuses(recipe,url);
});
test('Serious Eats refuses competing, missing, disconnected or unmatched required roots',()=>{
 for(const html of [recipe+recipe,recipe.replace('</main>','<main>Unclosed'),recipe.replace('<article class="mntl-article">','<article><article class="mntl-article">'),recipe.replace('<h1 class="article-heading">','<h1>'),recipe.replace('</article>','</article><div class="mm-recipes-structured-ingredients">Extra required region</div>'),recipe.replace('</main>','<div class="mm-recipes-steps__content">Unclosed')])refuses(html);
});
test('Serious Eats refuses hidden core content and never chooses required markup inside navigation',()=>{
 for(const html of ['<div hidden>'+recipe,recipe.replace('<main>','<main aria-hidden="true">'),recipe.replace('<p>Sea salt','<p hidden>Sea salt'),recipe.replace('Bake until golden.','<span hidden>Bake until golden.</span>'),'<nav>'+recipe+'</nav>'])refuses(html);
});
test('Serious Eats refuses unaccounted rows, procedure text and core markers hidden inside ignored media/controls',()=>{
 for(const html of [recipe.replace('</ul>\n<p class="mm-recipes','</ul>Extra ingredient\n<p class="mm-recipes'),recipe.replace('Sea salt, to taste</p>','Sea salt, to taste</p>Extra required row text'),recipe.replace('Bake until golden.</p>','Bake until golden.</p><p>Extra unmarked procedure.</p>'),recipe.replace('Photographer credit, not a step.','<li class="mntl-sc-block-group--LI">Lost required step.</li>'),recipe.replace('<button>Keep screen awake</button>','<li class="mm-recipes-structured-ingredients__list-item">Lost required row.</li>')]){assert.notEqual(html,recipe);refuses(html);}
});
test('Serious Eats missing optional metadata remains missing, never borrowed from comments',()=>{
 const c=parse(recipe.replace(/<p class="article-subheading">[\s\S]*?<\/p>/u,'').replace(/<div class="mm-recipes-details">[\s\S]*?<button>Jump to nutrition<\/button><\/div><\/div>/u,''));assert.equal(c.parser.sourceAdapter,'serious-eats');assert.equal(c.fields.description,null);assert.equal(c.fields.totalTime,null);assert.equal(c.fields.yield,null);
});
test('Serious Eats duplicate metadata and unknown supplemental procedure are refused conservatively',()=>{
 for(const html of [recipe.replace('Prep Time:</div>','Prep Time:</div><div class="mm-recipes-details__value">10 mins</div>'),recipe.replace('Special Equipment</h2>','Unknown Required Section</h2>'),recipe.replace('<p class="mntl-sc-block-html">Drain','Loose required text<p class="mntl-sc-block-html">Drain')])refuses(html);
});
test('Serious Eats active bodies are opaque and access shells cannot activate an adapter',()=>{
 assert.equal(parse('<script>'+recipe+'</script><template>'+recipe+'</template>'+recipe).parser.sourceAdapter,'serious-eats');refuses('<main><article><h1>Subscribe</h1></article></main>');
});
test('Serious Eats conflicting Total aliases preserve both labels without choosing either duration',()=>{
 const html=recipe.replace('<div class="mm-recipes-details__item"><div class="mm-recipes-details__label">Total Time:', '<div class="mm-recipes-details__item"><div class="mm-recipes-details__label">Total:</div><div class="mm-recipes-details__value">1 hr</div></div><div class="mm-recipes-details__item"><div class="mm-recipes-details__label">Total Time:');
 const c=parse(html);assert.equal(c.parser.sourceAdapter,'serious-eats');assert.equal(c.fields.totalTime.value,null);assert.equal(c.fields.totalTime.confidence,'low');assert.equal(c.fields.totalTime.raw,'Total: 1 hr; Total Time: 1 hr 20 mins');assert.ok(c.warnings.some(w=>w.code==='AMBIGUOUS_PROVIDER_DURATION'));
});
test('Serious Eats retains explicit note lists as separate notes, never steps',()=>{
 const c=parse(recipe.replace('<p class="mntl-sc-block-html">Drain the beans well.</p>','<ul class="mntl-sc-block-html"><li>Drain the beans well.</li><li>Use a sturdy dish.</li></ul>'));
 assert.equal(c.parser.sourceAdapter,'serious-eats');assert.ok(c.fields.notes.some(n=>n.value==='Notes: Use a sturdy dish.'));assert.equal(c.fields.instructionSections[0].steps.length,2);
});
test('Serious Eats refuses recognized procedure blocks inside ignored media and ads',()=>{
 for(const html of [recipe.replace('Photographer credit, not a step.','<p class="mntl-sc-block-html">Additional required action.</p>'),recipe.replace('<div class="mntl-sc-block-adslot"></div>','<div class="mntl-sc-block-adslot"><p class="mntl-sc-block-html">Additional required action.</p></div>')])refuses(html);
});
test('Serious Eats excludes hidden lead images and hidden image ancestors',()=>{
 for(const attribute of ['hidden','aria-hidden="true"','style="display:none"'])assert.deepEqual(parse(recipe.replace('<img data-src=',`<img ${attribute} data-src=`)).fields.images,[]);
 assert.deepEqual(parse(recipe.replace('<img data-src="https://images.example.invalid/bake.jpg" alt="Invented bean bake">','<span hidden><img data-src="https://images.example.invalid/bake.jpg"></span>')).fields.images,[]);
});
test('Serious Eats refuses malformed optional metadata instead of treating it as absent',()=>{
 for(const html of [recipe.replace('Jump to nutrition</button>','Jump to nutrition</button><div class="mm-recipes-details__item"><div class="mm-recipes-details__label">Cook Time:</div><div class="mm-recipes-details__value">99 mins'),recipe.replace('A cozy bake with bright herbs.</p>','A cozy bake with bright herbs.</p><p class="article-subheading">Unclosed description'),recipe.replace('Rowan Vale</span>','Rowan Vale</span><span class="mntl-attribution__item-name">Unclosed author')])refuses(html);
});
test('Serious Eats refuses unmatched procedural markers inside ignored media',()=>{
 for(const html of [recipe.replace('<div class="mntl-sc-block-adslot"></div>','<div class="mntl-sc-block-adslot"><p class="mntl-sc-block-html">Additional action.</div>'),recipe.replace('Photographer credit, not a step.','<p class="mntl-sc-block-html">Additional action.')])refuses(html);
});
