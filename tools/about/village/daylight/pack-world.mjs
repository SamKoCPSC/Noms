// Combine the tiled ground and vertex-lit landscape without removing triangles.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { mergeDocuments, prune, unpartition } from '@gltf-transform/functions';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
const [directory] = process.argv.slice(2);
if (!directory) throw new Error('Usage: node pack-world.mjs BAKE_DIRECTORY');
const base = 'assets/models/terraced-village/daylight';
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS);
const ground = await io.read(resolve(directory, 'ground.glb'));
const landscape = await io.read(resolve(directory, 'landscape-vertex.glb'));
const triangles = doc => doc.getRoot().listMeshes().flatMap(m => m.listPrimitives())
  .reduce((sum, p) => sum + (p.getIndices()?.getCount() ?? p.getAttribute('POSITION').getCount()) / 3, 0);
const expected = triangles(ground) + triangles(landscape);
// These surfaces carry final display-baked lighting. Their unlit browser shaders
// do not use normals or tangents; retain the detailed geometry and color instead
// of spending download/memory on unused lighting attributes.
let omittedLightingAttributes = 0;
for (const document of [ground, landscape]) {
  for (const mesh of document.getRoot().listMeshes()) {
    for (const primitive of mesh.listPrimitives()) {
      if (!primitive.getMaterial()?.getExtension('KHR_materials_unlit')) continue;
      for (const semantic of ['NORMAL', 'TANGENT']) {
        if (primitive.getAttribute(semantic)) {
          primitive.setAttribute(semantic, null);
          omittedLightingAttributes++;
        }
      }
    }
  }
}
mergeDocuments(ground, landscape);
const [scene, ...rest] = ground.getRoot().listScenes();
for (const other of rest) {
  for (const node of other.listChildren()) scene.addChild(node);
  other.dispose();
}
ground.getRoot().setDefaultScene(scene);
await ground.transform(prune(), unpartition());
if (triangles(ground) !== expected) throw new Error('World merge lost geometry');
const input = resolve(directory, 'world-uncompressed.glb');
await io.write(input, ground);
const metadata = resolve(base, 'source/world-bake.json');
const report = {
  ground: JSON.parse(await readFile(resolve(directory, 'ground.json'))),
  landscape: JSON.parse(await readFile(resolve(directory, 'vertex-verification.json'))),
  triangles: expected,
  unused_unlit_lighting_attributes_omitted: omittedLightingAttributes,
  glb_bytes: (await readFile(input)).length,
};
await writeFile(metadata, JSON.stringify(report, null, 2) + '\n');
const output = resolve(base, 'models/world.glb');
execFileSync('node', ['tools/about/baked/pack.mjs', input, output, metadata], { stdio: 'inherit' });
const packed = JSON.parse(await readFile(metadata));
packed.packed_sha256 = createHash('sha256').update(await readFile(output)).digest('hex');
await writeFile(metadata, JSON.stringify(packed, null, 2) + '\n');
