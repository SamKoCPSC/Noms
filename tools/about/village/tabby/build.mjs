// From the repository root: node tools/about/village/tabby/build.mjs
import { Document, NodeIO } from '@gltf-transform/core';
import { EXTMeshoptCompression, KHRMeshQuantization } from '@gltf-transform/extensions';
import { MeshoptEncoder } from 'meshoptimizer';
import sharp from 'sharp';
import { makeTabby } from './model.mjs';

const cat = makeTabby();
const doc = new Document();
const buffer = doc.createBuffer();
const { data, width, height } = cat.material.map.image;
const texture = doc
  .createTexture('Guard hairs')
  .setMimeType('image/png')
  .setImage(
    await sharp(data, { raw: { width, height, channels: 4 } })
      .png()
      .toBuffer()
  );
doc.createExtension(KHRMeshQuantization).setRequired(true);
const coat = doc
  .createMaterial('Grey mackerel tabby')
  .setMetallicFactor(0)
  .setRoughnessFactor(1)
  .setDoubleSided(true)
  .setBaseColorTexture(texture);
const primitive = doc.createPrimitive().setMaterial(coat);
for (const [name, semantic, type] of [
  ['position', 'POSITION', 'VEC3'],
  ['normal', 'NORMAL', 'VEC3'],
  ['color', 'COLOR_0', 'VEC3'],
  ['uv', 'TEXCOORD_0', 'VEC2']
]) {
  const array = cat.geometry.getAttribute(name).array;
  const attribute = doc.createAccessor().setBuffer(buffer).setType(type);
  if (name === 'position')
    attribute.setArray(Int16Array.from(array, (v) => Math.round(32767 * v))).setNormalized(true);
  else if (name === 'normal')
    attribute.setArray(Int8Array.from(array, (v) => Math.round(127 * v))).setNormalized(true);
  else if (name === 'color')
    attribute.setArray(Uint8Array.from(array, (v) => Math.round(255 * v))).setNormalized(true);
  else attribute.setArray(Float32Array.from(array, (v) => Math.round(v * 4096) / 4096));
  primitive.setAttribute(semantic, attribute);
}
primitive.setIndices(
  doc.createAccessor().setBuffer(buffer).setType('SCALAR').setArray(cat.geometry.index.array)
);
doc.createScene().addChild(doc.createNode('Tabby').setMesh(doc.createMesh('Tabby').addPrimitive(primitive)));
doc
  .createExtension(EXTMeshoptCompression)
  .setRequired(true)
  .setEncoderOptions({ method: EXTMeshoptCompression.EncoderMethod.QUANTIZE });
await MeshoptEncoder.ready;
await new NodeIO()
  .registerExtensions([EXTMeshoptCompression, KHRMeshQuantization])
  .registerDependencies({ 'meshopt.encoder': MeshoptEncoder })
  .write('assets/about/village/tabby.glb', doc);
