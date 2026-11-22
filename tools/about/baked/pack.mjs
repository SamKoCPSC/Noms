// Quantize and compress buffers without simplifying any triangles or resizing textures.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { dedup, meshopt } from '@gltf-transform/functions';
import sharp from 'sharp';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';
import { readFile, writeFile, stat } from 'node:fs/promises';
const [input, output, metadata] = process.argv.slice(2);
if (!input || !output || !metadata) throw new Error('Usage: node pack.mjs INPUT OUTPUT LIGHTING_JSON');
await MeshoptEncoder.ready; await MeshoptDecoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.encoder': MeshoptEncoder, 'meshopt.decoder': MeshoptDecoder });
const document = await io.read(input);
const triangles = () => document.getRoot().listMeshes().flatMap(m => m.listPrimitives()).reduce((n, p) => n + (p.getIndices()?.getCount() || p.getAttribute('POSITION').getCount()) / 3, 0);
const before = triangles();
// Browsers sample these display-baked colors as 8-bit sRGB. Keep every texel,
// remove only unused alpha and 16-bit PNG storage (linear EXRs stay in the bake).
for (const texture of document.getRoot().listTextures()) {
  if (texture.getMimeType() === 'image/png') {
    const pixels = await sharp(Buffer.from(texture.getImage())).removeAlpha().png({ compressionLevel: 9 }).toBuffer();
    texture.setImage(new Uint8Array(pixels));
  }
}
await document.transform(dedup(), meshopt({ encoder: MeshoptEncoder, level: 'medium', quantizePosition: 16, quantizeNormal: 12, quantizeTexcoord: 16 }));
if (triangles() !== before) throw new Error('Packing changed triangle count');
await io.write(output, document);
const lighting = JSON.parse(await readFile(metadata, 'utf8'));
lighting.uncompressed_bytes ??= lighting.glb_bytes;
lighting.glb_bytes = (await stat(output)).size;
lighting.display_texture_format = 'PNG RGB8 sRGB, original resolution';
lighting.geometry_compression = { extension: 'EXT_meshopt_compression', position_bits: 16, normal_bits: 12, uv_bits: 16, triangles_removed: 0 };
await writeFile(metadata, JSON.stringify(lighting, null, 2) + '\n');
console.log(`PACKED ${before} triangles: ${lighting.uncompressed_bytes} -> ${lighting.glb_bytes} bytes`);
