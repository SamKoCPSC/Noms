// Copy the actual production assets byte-for-byte into a portable review folder.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';
import { build } from 'esbuild';
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const out = process.argv[2] || '/tmp/noms-village-preview';
await mkdir(`${out}/models`, { recursive: true });
const layout = JSON.parse(await readFile('assets/models/terraced-village/layout.json'));
await MeshoptDecoder.ready; await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const receipts = [];
for (const stop of layout.stops) {
  const before = await readFile(stop.model);
  const doc = await io.read(stop.model);
  const count = () => doc.getRoot().listMeshes().flatMap(m => m.listPrimitives()).reduce((n,p) => n+(p.getIndices()?.getCount()||p.getAttribute('POSITION').getCount())/3,0);
  const triangles = count();
  await copyFile(stop.model,`${out}/models/${stop.id}.glb`);
  receipts.push({id:stop.id,source:stop.model,sha256:createHash('sha256').update(before).digest('hex'),triangles});
  stop.lighting = JSON.parse(await readFile(stop.lighting));
  stop.model = `models/${stop.id}.glb`;
  console.log('PREVIEW',stop.id,triangles);
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
