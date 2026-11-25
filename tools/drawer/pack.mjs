// Produce lightweight, display-baked navigation models from the editable exports.
import {mkdir} from 'node:fs/promises';
import {NodeIO} from '@gltf-transform/core';
import {ALL_EXTENSIONS, KHRMaterialsUnlit} from '@gltf-transform/extensions';
import {flatten, join, meshopt, prune, weld} from '@gltf-transform/functions';
import {MeshoptEncoder} from 'meshoptimizer';
import sharp from 'sharp';

const directory = 'assets/brand/icons-3d';
const icons = ['noms-logo', 'home', 'dashboard', 'collections', 'kitchen', 'explore',
  'search', 'editor', 'about', 'how-to', 'introduction', 'profile', 'settings', 'history', 'import', 'accounts'];
await mkdir(`${directory}/drawer`, {recursive: true});
await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
  .registerDependencies({'meshopt.encoder': MeshoptEncoder});
for (const name of icons) {
  const doc = await io.read(`${directory}/models/${name}.glb`);
  const unlit = doc.createExtension(KHRMaterialsUnlit);
  const paint = doc.createMaterial('Painted lighting').setDoubleSided(true)
    .setExtension('KHR_materials_unlit', unlit.createUnlit());
  for (const mesh of doc.getRoot().listMeshes()) {
    for (const part of mesh.listPrimitives()) {
      const original = part.getMaterial();
      const atlas = original.getBaseColorTexture();
      if (atlas) {
        paint.setBaseColorTexture(atlas);
        const uv = part.getAttribute(`TEXCOORD_${original.getBaseColorTextureInfo().getTexCoord()}`);
        part.setAttribute('TEXCOORD_0', uv).setMaterial(paint);
      } else {
        // The search magnifier only needs a transparent tint at drawer size.
        part.setMaterial(doc.createMaterial('Glass').setAlphaMode('BLEND')
          .setBaseColorFactor([.83, .95, .88, .13]).setDoubleSided(true)
          .setExtension('KHR_materials_unlit', unlit.createUnlit()));
      }
      for (const attribute of ['NORMAL', 'TANGENT', 'TEXCOORD_1']) part.setAttribute(attribute, null);
    }
  }
  const atlas = paint.getBaseColorTexture();
  atlas.setImage(await sharp(atlas.getImage()).resize(384, 384).jpeg({quality: 87}).toBuffer())
    .setMimeType('image/jpeg');
  await doc.transform(prune(), flatten(), join(), weld(),
    meshopt({encoder: MeshoptEncoder, level: 'medium'}));
  await io.write(`${directory}/drawer/${name}.glb`, doc);
}
