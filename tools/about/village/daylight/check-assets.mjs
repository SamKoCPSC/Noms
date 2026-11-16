// Verify complete bakes, world-space bounds and consistent display lighting.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder } from 'meshoptimizer';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const base = 'assets/models/terraced-village/daylight';
const config = JSON.parse(await readFile(`${base}/lighting.json`));
const layout = JSON.parse(await readFile('assets/models/terraced-village/layout.json'));
await MeshoptDecoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.decoder': MeshoptDecoder });
const reports = [];
for (const id of [...layout.stops.map(s => s.id), 'world']) {
  const bytes = await readFile(`${base}/models/${id}.glb`);
  const receipt = JSON.parse(await readFile(`${base}/source/${id}-bake.json`));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), receipt.packed_sha256, id);
  const doc = await io.readBinary(new Uint8Array(bytes));
  const root = doc.getRoot();
  assert.equal(root.listScenes().length, 1);
  assert.equal(root.listAnimations().length, 0);
  const triangles = root.listMeshes().flatMap(m => m.listPrimitives())
    .reduce((sum, p) => sum + (p.getIndices()?.getCount() ?? p.getAttribute('POSITION').getCount()) / 3, 0);
  assert.equal(triangles, receipt.triangles, id);
  const unlit = root.listMaterials().filter(m => m.getExtension('KHR_materials_unlit'));
  assert.equal(unlit.length, id === 'world' ? 5 : 2, id);
  const textures = unlit.map(m => m.getBaseColorTexture()).filter(Boolean);
  assert.equal(textures.length, id === 'world' ? 4 : 2);
  for (const tex of textures) assert.deepEqual(tex.getSize(), [2048, 2048], id);
  if (id === 'world') {
    if(layout.stone_paths){
      const bytes=await readFile(layout.stone_paths.geometry);
      const hash=createHash('sha256').update(bytes).digest('hex');
      assert.equal(receipt.ground.stone_path_geometry_sha256,hash);
      assert.equal(receipt.landscape.stone_path_geometry_sha256,hash);
      const paths=JSON.parse(await readFile(layout.stone_paths.verification));
      assert.equal(paths.sha256,hash);
    }
    if(layout.vegetation){
      const hash=createHash('sha256').update(await readFile(layout.vegetation.geometry)).digest('hex');
      assert.equal(receipt.ground.vegetation_geometry_sha256,hash);
      assert.equal(receipt.landscape.vegetation_geometry_sha256,hash);
      const plants=JSON.parse(await readFile(layout.vegetation.verification));
      assert.equal(plants.sha256,hash);
      assert(plants.foliage_within_original_bounds && plants.paths_rooms_river_props_excluded);
      assert.equal(plants.groundcover_path_intrusions,0);
      assert(plants.finite_vertices_colors_and_unit_normals && plants.evergreen_undersides_closed && plants.petal_tops_face_sky);
      assert(plants.minimum_grass_center_path_clearance>plants.grass_envelope_radius);
    }
    if(layout.signage){
      const hash=createHash('sha256').update(await readFile(layout.signage.geometry)).digest('hex');
      assert.equal(receipt.ground.signage_geometry_sha256,hash);
      assert.equal(receipt.landscape.signage_geometry_sha256,hash);
      const signs=JSON.parse(await readFile(layout.signage.verification));
      assert.equal(signs.sha256,hash);
      assert(signs.street_positions_preserved && signs.non_sign_source_meshes_preserved && signs.noticeboard_positions_preserved && signs.direction_arrows_preserved);
      assert.deepEqual(signs.names,Object.fromEntries(layout.stops.map(s=>[s.id,s.title])));
      assert(signs.sign_mounts.every(m=>Object.entries(m).filter(([k])=>k.endsWith('clearance')).every(([,v])=>v>0)));
    }
    assert.deepEqual(receipt.ground.lighting, config);
    assert.deepEqual(receipt.landscape.lighting, config);
    assert(receipt.ground.full_world_occluders && receipt.landscape.full_world_occluders);
    assert(root.listMeshes().some(m => m.listPrimitives().some(p => p.getAttribute('COLOR_0'))));
  } else {
    assert.deepEqual(receipt.lighting, config, id);
    assert(receipt.world_coordinates && receipt.penguini, id);
    for (const [path, hash] of Object.entries(receipt.source_hashes)) {
      assert.equal(createHash('sha256').update(await readFile(path)).digest('hex'), hash, path);
    }
  }
  reports.push({ id, bytes: bytes.length, triangles, baked_materials: unlit.length,
    texture_dimensions: textures.map(t => t.getSize()), sha256: receipt.packed_sha256 });
}
await writeFile(`${base}/source/asset-verification.json`, JSON.stringify({
  models: 12, shared_world: true, original_source_hashes_verified: true,
  consistent_lighting: true, diffuse_materials_display_baked: true,
  realtime_shadow_maps_required: false, assets: reports,
}, null, 2) + '\n');
console.log('DAYLIGHT_ASSETS_CHECK_PASS', reports.length);
