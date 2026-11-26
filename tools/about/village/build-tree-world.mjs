// Add the painted counterparts of the trees used in the outdoor daylight bake.
import { NodeIO } from '@gltf-transform/core';
import { reorder } from '@gltf-transform/functions';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';
import { addTreeMeshes } from './trees/grow.mjs';
import { treePlantings } from './trees/plantings.mjs';

export async function buildTreeWorld(source, output) {
  await Promise.all([MeshoptDecoder.ready, MeshoptEncoder.ready]);
  const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({
    'meshopt.decoder': MeshoptDecoder,
    'meshopt.encoder': MeshoptEncoder
  });
  const doc = await io.read(source);
  await addTreeMeshes(doc, await treePlantings());
  await doc.transform(reorder({ encoder: MeshoptEncoder, target: 'size' }));
  await io.write(output, doc);
}
