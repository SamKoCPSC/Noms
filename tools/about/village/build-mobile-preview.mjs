// Package an Android content:// compatible preview with no neighboring files.
// Re-encode only texture images; preserve geometry bytes and image dimensions.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import sharp from 'sharp';

const folder = process.argv[2] || '/tmp/noms-village-preview';
const out = process.argv[3] || '/tmp/noms-village-mobile';
await mkdir(out, { recursive: true });
const layout = JSON.parse(await readFile(`${folder}/layout.json`, 'utf8'));
// Full-village bakes use a slightly smaller download encoding; originals stay intact.
const textureQuality = Number(process.argv[4] ?? (layout.shared_lighting ? 93 : 95));
if (!Number.isInteger(textureQuality) || textureQuality < 1 || textureQuality > 100) throw Error('WebP quality must be an integer from 1 to 100');
const receipts = [];
let html = (await readFile(`${folder}/index.html`, 'utf8'))
  .replace('<script type="module" src="preview.js"></script></html>', '');
html += '<script type="text/plain" id="droid-font-notice">' + (await readFile('tools/about/village/DROID-FONT-NOTICE.txt','utf8')).replaceAll('</','<\\/') + '</script>\n';
html += '<script>globalThis.__VILLAGE_LAYOUT__=' + JSON.stringify(layout).replaceAll('</', '<\\/') + ';</script>\n';
const assets=[...layout.stops,...(layout.shared_lighting?[{id:'world',model:layout.shared_lighting.environment}]:[])];
for (const stop of assets) {
  const original = await readFile(`${folder}/${stop.model}`);
  const jsonLength = original.readUInt32LE(12);
  const gltf = JSON.parse(original.subarray(20, 20 + jsonLength).toString());
  const bin = original.subarray(28 + jsonLength);
  const chunks = [];
  let offset = 0;
  function append(bytes) {
    const padding = (8 - offset % 8) % 8;
    if (padding) { chunks.push(Buffer.alloc(padding)); offset += padding; }
    const start = offset; chunks.push(bytes); offset += bytes.length; return start;
  }
  const images = new Map(gltf.images.map(image => [image.bufferView, image]));
  const dimensions = [];
  let geometryViews = 0;
  for (let index = 0; index < gltf.bufferViews.length; index++) {
    const view = gltf.bufferViews[index];
    if (view.buffer === 0) {
      const before = bin.subarray(view.byteOffset || 0, (view.byteOffset || 0) + view.byteLength);
      let bytes = before;
      if (images.has(index)) {
        const meta = await sharp(before).metadata();
        bytes = await sharp(before).webp({ quality: textureQuality, alphaQuality: 100, effort: 4 }).toBuffer();
        const after = await sharp(bytes).metadata();
        if (meta.width !== after.width || meta.height !== after.height) throw Error('Texture dimensions changed');
        images.get(index).mimeType = 'image/webp';
        dimensions.push([meta.width, meta.height]);
      } else { geometryViews++; }
      view.byteOffset = append(bytes); view.byteLength = bytes.length;
    }
    const compressed = view.extensions?.EXT_meshopt_compression;
    if (compressed) {
      if (compressed.buffer !== 0) throw Error('Unexpected compressed buffer');
      const bytes = bin.subarray(compressed.byteOffset || 0, (compressed.byteOffset || 0) + compressed.byteLength);
      compressed.byteOffset = append(bytes); geometryViews++;
    }
  }
  for (const texture of gltf.textures) {
    if (texture.source === undefined) throw Error('Unexpected texture source');
    texture.extensions ??= {};
    texture.extensions.EXT_texture_webp = { source: texture.source };
    delete texture.source;
  }
  for (const key of ['extensionsUsed', 'extensionsRequired']) {
    gltf[key] = [...new Set([...(gltf[key] || []), 'EXT_texture_webp'])];
  }
  const bodyPadding = (4 - offset % 4) % 4;
  if (bodyPadding) { chunks.push(Buffer.alloc(bodyPadding)); offset += bodyPadding; }
  gltf.buffers[0].byteLength = offset;
  const body = Buffer.concat(chunks);
  // Check every non-image buffer range against the source, including meshopt.
  const source = JSON.parse(original.subarray(20, 20 + jsonLength).toString());
  for (let i = 0; i < gltf.bufferViews.length; i++) {
    const a = source.bufferViews[i], b = gltf.bufferViews[i];
    for (const [oldRange, newRange] of [
      ...(!images.has(i) && a.buffer === 0 ? [[a, b]] : []),
      ...(a.extensions?.EXT_meshopt_compression ? [[a.extensions.EXT_meshopt_compression, b.extensions.EXT_meshopt_compression]] : []),
    ]) {
      const oldBytes = bin.subarray(oldRange.byteOffset || 0, (oldRange.byteOffset || 0) + oldRange.byteLength);
      const newBytes = body.subarray(newRange.byteOffset || 0, (newRange.byteOffset || 0) + newRange.byteLength);
      if (!oldBytes.equals(newBytes)) throw Error(`Geometry changed: ${stop.id}/${i}`);
    }
  }
  let json = Buffer.from(JSON.stringify(gltf));
  json = Buffer.concat([json, Buffer.alloc((4 - json.length % 4) % 4, 32)]);
  const header = Buffer.alloc(20); header.writeUInt32LE(0x46546c67); header.writeUInt32LE(2, 4);
  header.writeUInt32LE(28 + json.length + body.length, 8); header.writeUInt32LE(json.length, 12); header.writeUInt32LE(0x4e4f534a, 16);
  const binHeader = Buffer.alloc(8); binHeader.writeUInt32LE(body.length); binHeader.writeUInt32LE(0x004e4942, 4);
  const result = Buffer.concat([header, json, binHeader, body]);
  html += `<script type="application/octet-stream" id="asset-${stop.id}">${result.toString('base64')}</script>\n`;
  receipts.push({ id: stop.id, source_sha256: createHash('sha256').update(original).digest('hex'), original_bytes: original.length, preview_bytes: result.length, texture_dimensions: dimensions, geometry_views_verified: geometryViews, texture_format: `WebP quality ${textureQuality}, original resolution` });
  console.log('EMBEDDED', stop.id, original.length, result.length);
}
const bundle = await build({ entryPoints: ['tools/about/village/preview.js'], bundle: true, format: 'esm', target: 'es2022', minify: true, write: false, legalComments: 'eof' });
html += '<script type="module">' + bundle.outputFiles[0].text.replaceAll('</script', '<\\/script') + '</script></html>';
await writeFile(`${out}/OPEN-VILLAGE-MOBILE.html`, html);
await writeFile(`${out}/mobile-package-receipts.json`, JSON.stringify(receipts, null, 2) + '\n');
console.log('MOBILE_HTML_READY', html.length);
