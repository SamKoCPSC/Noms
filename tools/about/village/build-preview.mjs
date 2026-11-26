// Assemble the portable review scene and the input to the production texture pass.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';
import { buildTreeWorld } from './build-tree-world.mjs';
import { build } from 'esbuild';
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const out = process.argv[2] || '/tmp/noms-village-preview';
await mkdir(`${out}/models`, { recursive: true });
const layout = JSON.parse(await readFile('assets/models/terraced-village/layout.json'));
await MeshoptDecoder.ready; await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const receipts = [];
if(layout.stone_paths) layout.stone_paths.verificationData=JSON.parse(await readFile(layout.stone_paths.verification));
if(layout.vegetation){
  const proof=JSON.parse(await readFile(layout.vegetation.verification));
  delete proof.original_foliage_bounds;
  layout.vegetation.verificationData=proof;
}
const anchors=layout.shared_lighting?JSON.parse(await readFile('assets/models/terraced-village/daylight/source/anchors.json')):null;
const poses=layout.shared_lighting?JSON.parse(await readFile('assets/models/terraced-village/source/camera-poses.json')):null;
for (const stop of layout.stops) {
  const source=stop.shared_model||stop.model;
  const before = await readFile(source);
  const doc = await io.read(source);
  const count = () => doc.getRoot().listMeshes().flatMap(m => m.listPrimitives()).reduce((n,p) => n+(p.getIndices()?.getCount()||p.getAttribute('POSITION').getCount())/3,0);
  const triangles = count();
  await copyFile(source,`${out}/models/${stop.id}.glb`);
  receipts.push({id:stop.id,source,sha256:createHash('sha256').update(before).digest('hex'),triangles});
  if(anchors){stop.reference_bounds=anchors[stop.id];stop.reference_box=poses.find(p=>p.id===stop.id).bounds;}
  stop.lighting = JSON.parse(await readFile(stop.lighting));
  stop.model = `models/${stop.id}.glb`;
  console.log('PREVIEW',stop.id,triangles);
}
if(layout.shared_lighting){
  const source=layout.shared_lighting.environment;
  await buildTreeWorld(source,`${out}/models/world.glb`);
  const bytes=await readFile(`${out}/models/world.glb`);
  receipts.push({id:'world',source,source_sha256:createHash('sha256').update(await readFile(source)).digest('hex'),
    sha256:createHash('sha256').update(bytes).digest('hex')});
  layout.shared_lighting.environment='models/world.glb';
  layout.shared_lighting.settings=JSON.parse(await readFile(layout.shared_lighting.settings));
}
await writeFile(`${out}/layout.json`, JSON.stringify(layout));
await writeFile(`${out}/sources.json`, JSON.stringify(receipts,null,2));
await copyFile('tools/about/village/index.html',`${out}/index.html`);
await build({entryPoints:['tools/about/village/preview.js'],bundle:true,format:'esm',target:'es2022',minify:true,outfile:`${out}/preview.js`,legalComments:'eof'});
await copyFile('tools/about/village/DROID-FONT-NOTICE.txt',`${out}/DROID-FONT-NOTICE.txt`);
await copyFile('tools/about/three/THREE-LICENSE.txt',`${out}/THREE-LICENSE.txt`);
await copyFile('tools/about/three/MESHOPT-LICENSE.txt',`${out}/MESHOPT-LICENSE.txt`);
await writeFile(`${out}/START.py`, 'from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler\nimport os, webbrowser\nfrom pathlib import Path\nos.chdir(Path(__file__).parent)\nwebbrowser.open("http://localhost:8766")\nThreadingHTTPServer(("127.0.0.1",8766),SimpleHTTPRequestHandler).serve_forever()\n');
console.log('READY',out);
