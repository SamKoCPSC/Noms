import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder } from 'meshoptimizer/meshopt_decoder.module.js';
import { createHash } from 'node:crypto';
const io=new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.decoder':MeshoptDecoder});
await MeshoptDecoder.ready;
const signature=doc=>doc.getRoot().listMeshes().flatMap(m=>m.listPrimitives()).filter(p=>p.getMaterial()?.getName().startsWith('Baked master lighting ')).map(p=>{
 const hash=createHash('sha256');
 for(const a of [p.getIndices(),p.getAttribute('POSITION'),p.getAttribute('TEXCOORD_0')])hash.update(Buffer.from(a.getArray().buffer,a.getArray().byteOffset,a.getArray().byteLength));
 hash.update(Buffer.from(p.getMaterial().getBaseColorTexture().getImage()));
 return [p.getMaterial().getName(),hash.digest('hex')];
}).sort();
const [beforePath,afterPath]=process.argv.slice(2);
if(!beforePath||!afterPath)throw new Error('Usage: node verify_material_reexport.mjs BEFORE_GLB AFTER_GLB');
const before=await io.read(beforePath),after=await io.read(afterPath);
if(JSON.stringify(signature(before))!==JSON.stringify(signature(after)))throw new Error('Baked geometry, UVs or texture changed; do not reuse the lighting');
const live=after.getRoot().listMeshes().flatMap(m=>m.listPrimitives()).filter(p=>!p.getMaterial()?.getName().startsWith('Baked master lighting '));
if(!live.length||!live.every(p=>p.getAttribute('COLOR_0')))throw new Error('Missing recovered live vertex colors');
console.log('PASS: opaque geometry/UV/texture byte-identical; live vertex colors preserved');
