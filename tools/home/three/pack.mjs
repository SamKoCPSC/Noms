// Lossless Meshopt buffer compression: keep every animated key and vertex intact.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS, EXTMeshoptCompression } from '@gltf-transform/extensions';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';
import { stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const [input, output] = process.argv.slice(2);
if (!input || !output) throw new Error('Usage: node tools/home/three/pack.mjs INPUT.glb OUTPUT.glb');
await MeshoptEncoder.ready; await MeshoptDecoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.encoder': MeshoptEncoder, 'meshopt.decoder': MeshoptDecoder});
const doc = await io.read(input);
doc.createExtension(EXTMeshoptCompression).setRequired(true).setEncoderOptions({method: EXTMeshoptCompression.EncoderMethod.QUANTIZE});
await io.write(output, doc);
const decoded = await io.read(output);
// Compare every accessor after decompression, including skin weights and animation keys.
const before = doc.getRoot().listAccessors(), after = decoded.getRoot().listAccessors();
if (before.length !== after.length) throw new Error('Accessor count changed');
const primitives = d => d.getRoot().listMeshes().flatMap(m=>m.listPrimitives());
const indices = d => new Set(primitives(d).map(p=>p.getIndices()).filter(Boolean));
const fingerprint = a => createHash('sha256').update(Buffer.from(new Float64Array(a.getArray()).buffer)).digest('hex');
const origIndices=indices(doc), packedIndices=indices(decoded);
const a = before.filter(a=>!origIndices.has(a)).map(fingerprint).sort();
const b = after.filter(a=>!packedIndices.has(a)).map(fingerprint).sort();
if (a.length!==b.length || a.some((v,i)=>v!==b[i])) throw new Error('Decoded vertex or animation values changed');
const triangles = p => {
  const values=p.getIndices()?.getArray() || Array.from({length:p.getAttribute('POSITION').getCount()},(_,i)=>i);
  const result=[];
  for(let i=0;i<values.length;i+=3) {
    const tri=Array.from(values.slice(i,i+3)), j=tri.indexOf(Math.min(...tri));
    result.push([tri[j],tri[(j+1)%3],tri[(j+2)%3]].join(','));
  }
  return result.sort().join(';');
};
const geometryBefore=primitives(doc).map(triangles).sort(), geometryAfter=primitives(decoded).map(triangles).sort();
if (geometryBefore.length!==geometryAfter.length || geometryBefore.some((v,i)=>v!==geometryAfter[i])) throw new Error('Triangle topology changed');
if (doc.getRoot().listSkins().length!==decoded.getRoot().listSkins().length || doc.getRoot().listAnimations().length!==decoded.getRoot().listAnimations().length) throw new Error('Skin or clip lost');
console.log(`Verified vertex/animation values, skin count, and triangle winding; ${(await stat(input)).size} -> ${(await stat(output)).size} bytes`);
