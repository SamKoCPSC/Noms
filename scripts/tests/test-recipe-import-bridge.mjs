import assert from 'node:assert/strict';
import test from 'node:test';
import { parseClipboardPayload, mountClipboardImport, MAX_CLIPBOARD_BYTES } from '../../assets/recipe-import-bridge.mjs';
import { parseRecipeClipboard } from '../recipe-import-parser.mjs';
import { readFile } from 'node:fs/promises';

test('bridge feeds full HTML and text to the existing parser with the supplied source', async () => {
  const html = await readFile(new URL('../../tests/fixtures/recipe-import/v1/basic-visible-recipe/input.html', import.meta.url), 'utf8');
  const payload = {html, plainText: null, sourceUrl: 'https://example.invalid/recipe'};
  assert.deepEqual(await parseClipboardPayload(payload, parseRecipeClipboard), parseRecipeClipboard(payload));
});
for (const [fixture, sourceUrl, adapter] of [
  ['bbcgoodfood-visible.html', 'https://www.bbcgoodfood.com/recipes/cloud-bean-stew', 'bbc-good-food'],
  ['kingarthur-visible.html', 'https://www.kingarthurbaking.com/recipes/aurora-cloud-rolls', 'king-arthur-baking'],
]) test(`bridge preserves rich clipboard provider dispatch for ${adapter}`, async () => {
  const html = await readFile(new URL(`../../tests/fixtures/recipe-import/providers/${fixture}`, import.meta.url), 'utf8');
  const payload = {html, plainText:'Full clipboard text fallback', sourceUrl};
  const parsed = await parseClipboardPayload(payload, parseRecipeClipboard);
  assert.equal(parsed.parser.sourceAdapter, adapter);
  assert.deepEqual(parsed, parseRecipeClipboard(payload));
  assert.equal(parsed.fields.ingredientSections.flatMap(section => section.items).length, 3);
  assert.equal(parsed.fields.instructionSections.flatMap(section => section.steps).length, 2);
  assert.ok(parsed.fields.description.value);
});
test('bridge supports plain text and rejects empty, invalid URL and oversized input before parsing', async () => {
  let calls = 0;
  const parser = input => { calls++; return input; };
  const plain = {html: '', plainText:'Recipe text', sourceUrl:''};
  assert.equal((await parseClipboardPayload(plain, parser)).plainText, 'Recipe text');
  for (const payload of [
    {html:'',plainText:'   ',sourceUrl:''},
    {...plain,sourceUrl:'javascript:alert(1)'},
    {...plain,sourceUrl:'not a URL'},
    {...plain,plainText:'a'.repeat(MAX_CLIPBOARD_BYTES+1)},
    {...plain,plainText:'🍜'.repeat(Math.floor(MAX_CLIPBOARD_BYTES/4)+1)},
  ]) await assert.rejects(async () => parseClipboardPayload(payload, parser));
  assert.equal(calls,1);
});

test('bridge cancels obsolete results, preserves both MIME types and cleans up on unmount', () => {
  const originalWorker = globalThis.Worker;
  const originalDocument = globalThis.document;
  const workers = [];
  class WorkerStub {
    constructor() { workers.push(this); }
    postMessage(payload) { this.payload = payload; }
    terminate() { this.terminated = true; }
  }
  globalThis.Worker = WorkerStub;
  globalThis.document = {baseURI:'http://localhost/recipes/import'};
  const textarea = Object.assign(new EventTarget(), {value:'', matches:() => true});
  const messages = [];
  const dispatch = (type, properties={}) => textarea.dispatchEvent(Object.assign(new Event(type, {cancelable:true}), properties));
  let cleanup;
  try {
    cleanup = mountClipboardImport(textarea, '/assets/parser.mjs', value => messages.push(value));
    assert.equal(messages[0]?.type, 'ready', 'UI must remain disabled until the paste listeners are attached');
    dispatch('paste', {clipboardData:{getData:type => type === 'text/html' ? '<h1>Soup</h1>' : 'Soup'}});
    dispatch('noms-preview', {detail:{sourceUrl:'https://example.invalid/soup'}});
    assert.equal(workers[0].payload.html, '<h1>Soup</h1>');
    assert.equal(workers[0].payload.plainText, 'Soup');
    dispatch('noms-source-change', {detail:{sourceUrl:'https://example.invalid/new'}});
    assert.equal(workers[0].terminated, true);
    workers[0].onmessage({data:{type:'result',candidate:{old:true}}});
    assert.equal(messages.some(m => m.type === 'result'), false);
    textarea.value = 'Edited soup'; dispatch('input'); dispatch('noms-preview');
    assert.equal(workers[1].payload.html, '');
    assert.equal(workers[1].payload.plainText, 'Edited soup');
    workers[1].onmessage({data:{type:'result',candidate:{fresh:true}}});
    assert.equal(messages.at(-1).candidate.fresh, true);
    cleanup(); cleanup = null;
    const count = messages.length;
    dispatch('paste', {clipboardData:{getData:()=>'After unmount'}});
    assert.equal(messages.length,count);
  } finally {
    cleanup?.(); globalThis.Worker = originalWorker; globalThis.document = originalDocument;
  }
});
