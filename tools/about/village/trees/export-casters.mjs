// Export the tour geometry with explicit transforms and physical materials for baking.
import { Document, NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { uninstance } from '@gltf-transform/functions';
import { writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { addTreeMeshes } from './grow.mjs';
import { treePlantings } from './plantings.mjs';

export async function treeCasterBytes(trees) {
  const doc = new Document();
  doc.createBuffer();
  doc.getRoot().setDefaultScene(doc.createScene('Village tree shadow casters'));
  await addTreeMeshes(doc, trees);
  await doc.transform(uninstance());
  // Diffuse bark and leaves can bounce light in Cycles. Retain the tour's cutout
  // atlas, UV coordinates and alpha cutoff so daylight passes between the leaves.
  for (const mat of doc.getRoot().listMaterials()) {
    mat.setMetallicFactor(0).setRoughnessFactor(1).setExtension('KHR_materials_unlit', null);
  }
  return new NodeIO().registerExtensions(ALL_EXTENSIONS).writeBinary(doc);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await writeFile(process.argv[2], await treeCasterBytes(await treePlantings()));
}
