import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import * as parser from '../recipe-import-parser.mjs';
import {validateImportCandidate, validateJsonSchema} from '../validate-recipe-import-fixtures.mjs';

const fixtureRoot=new URL('../../tests/fixtures/recipe-import/',import.meta.url);
const schema=JSON.parse(await fs.readFile(new URL('v1/import-candidate.schema.json',fixtureRoot),'utf8'));
const choices=[['bbc-good-food','BBC Good Food','bbcgoodfood'],['king-arthur-baking','King Arthur Baking','kingarthur'],['bon-appetit','Bon Appétit','bonappetit'],['epicurious','Epicurious','epicurious'],['foodnetwork','Food Network','foodnetwork'],['sallys-baking-addiction',"Sally’s Baking Addiction",'sallys'],['simply-recipes','Simply Recipes','simply-recipes'],['serious-eats','Serious Eats','serious-eats'],['nyt-cooking','NYT Cooking','nyt']];

test('provider catalogue has all 15 unique supported choices and exact public HTTP(S) origins',()=>{
 assert.equal(parser.RECIPE_IMPORT_PROVIDERS.length,15);
 assert.equal(new Set(parser.RECIPE_IMPORT_PROVIDERS.map(p=>p.id)).size,15);
 assert.deepEqual(new Set(parser.RECIPE_IMPORT_PROVIDERS.map(p=>p.id)),new Set(['allrecipes','bbc-good-food','bon-appetit','budgetbytes','epicurious','foodcom','food52','foodnetwork','king-arthur-baking','nyt-cooking','recipetineats','sallys-baking-addiction','serious-eats','simply-recipes','tasty']));
 for(const p of parser.RECIPE_IMPORT_PROVIDERS){assert.ok(p.label);const url=new URL(p.origin);assert.equal(url.protocol,'https:');assert.equal(url.pathname,'/');}
});
for(const [providerId,,fixture] of choices)test(`${providerId} dispatches without a URL and never fabricates recipe attribution`,async()=>{
 const html=await fs.readFile(new URL(`providers/${fixture}-visible.html`,fixtureRoot),'utf8');
 const c=parser.parseRecipeClipboard({html,providerId});
 assert.equal(c.parser.sourceAdapter,providerId);
 assert.equal(c.source.suppliedUrl,null);assert.equal(c.source.normalizedUrl,null);
 assert.deepEqual(c.source.provenance.suppliedUrl,[]);
 assert.equal(c.source.provenance.provider[0].representation,'provider_selection');
 assert.deepEqual(validateImportCandidate(c),[]);assert.deepEqual(validateJsonSchema(c,schema),[]);
});
test('selection preserves optional matching recipe attribution and absolute media references',async()=>{
 const html=await fs.readFile(new URL('providers/kingarthur-visible.html',fixtureRoot),'utf8');
 const sourceUrl='https://www.kingarthurbaking.com/recipes/aurora#steps';
 const c=parser.parseRecipeClipboard({html,sourceUrl,providerId:'king-arthur-baking'});
 assert.equal(c.source.suppliedUrl,sourceUrl);assert.equal(c.source.normalizedUrl,sourceUrl.split('#')[0]);
 assert.deepEqual(c.fields,parser.parseRecipeClipboard({html,sourceUrl}).fields);
 const noUrl=parser.parseRecipeClipboard({html:html.replace('/images/rolls.jpg','https://images.example.invalid/rolls.jpg'),providerId:'king-arthur-baking'});
 assert.equal(noUrl.fields.images[0].sourceUrl,'https://images.example.invalid/rolls.jpg');
});
test('selection never guesses page-relative media URLs when recipe URL is absent',async()=>{
 const html=await fs.readFile(new URL('providers/kingarthur-visible.html',fixtureRoot),'utf8');
 const c=parser.parseRecipeClipboard({html:html.replace('/images/rolls.jpg','photo.jpg'),providerId:'king-arthur-baking'});
 assert.deepEqual(c.fields.images,[]);assert.ok(c.warnings.some(w=>w.code==='RELATIVE_IMAGE_SOURCE_URL_MISSING'));
});
test('NYT absolute srcset images survive selection without a URL',async()=>{
 const html=await fs.readFile(new URL('providers/nyt-visible.html',fixtureRoot),'utf8');
 const c=parser.parseRecipeClipboard({html,providerId:'nyt-cooking'});
 const withUrl=parser.parseRecipeClipboard({html,providerId:'nyt-cooking',sourceUrl:'https://cooking.nytimes.com/recipes/1'});
 assert.deepEqual(c.fields.images,withUrl.fields.images);assert.equal(c.fields.images.length,2);
 assert.ok(!c.warnings.some(w=>w.code==='RELATIVE_IMAGE_SOURCE_URL_MISSING'));
});
test('unrelated image attributes and other images cannot authorize a guessed page-relative URL',async()=>{
 const html=await fs.readFile(new URL('providers/kingarthur-visible.html',fixtureRoot),'utf8');
 for(const extra of [' data-unused="https://www.kingarthurbaking.com/images/rolls.jpg"','']){
  const c=parser.parseRecipeClipboard({html:html.replace('src="/images/rolls.jpg"','src="/images/rolls.jpg"'+extra)+'<img src="https://www.kingarthurbaking.com/images/rolls.jpg">',providerId:'king-arthur-baking'});
  assert.deepEqual(c.fields.images,[]);
 }
});
test('blank optional URL is equivalent to absent when selecting a provider',async()=>{
 const html=await fs.readFile(new URL('providers/kingarthur-visible.html',fixtureRoot),'utf8');
 assert.equal(parser.parseRecipeClipboard({html,providerId:'king-arthur-baking',sourceUrl:'   '}).source.suppliedUrl,null);
});
test('invalid, inherited and conflicting provider selections fail clearly',async()=>{
 const html=await fs.readFile(new URL('providers/kingarthur-visible.html',fixtureRoot),'utf8');
 for(const providerId of ['unknown','constructor','toString',42,{}])assert.throws(()=>parser.parseRecipeClipboard({html,providerId}),/provider/i);
 for(const sourceUrl of ['https://www.allrecipes.com/recipe/1','https://kingarthurbaking.com.evil.invalid/','ftp://kingarthurbaking.com/a','not a URL'])assert.throws(()=>parser.parseRecipeClipboard({html,providerId:'king-arthur-baking',sourceUrl}),/URL|provider/i);
});
test('automatic mode and conservative generic fallback retain actual source rather than a provider home URL',async()=>{
 const html=await fs.readFile(new URL('v1/basic-visible-recipe/input.html',fixtureRoot),'utf8');
 assert.deepEqual(parser.parseRecipeClipboard({html,providerId:''}),parser.parseRecipeClipboard({html}));
 const c=parser.parseRecipeClipboard({html,providerId:'king-arthur-baking'});
 assert.equal(c.parser.sourceAdapter,null);assert.equal(c.source.suppliedUrl,null);assert.ok(c.warnings.some(w=>w.code==='SELECTED_PROVIDER_FALLBACK'));
});
test('King Arthur native text accepts explicit selection without a URL',async()=>{
 const plainText=await fs.readFile(new URL('providers/kingarthur-plain.txt',fixtureRoot),'utf8');
 const c=parser.parseRecipeClipboard({plainText,providerId:'king-arthur-baking'});
 assert.equal(c.parser.sourceAdapter,'king-arthur-baking');assert.equal(c.source.suppliedUrl,null);
});
