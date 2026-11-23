import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { groundHeight } from './walking-world.js';

/** Seeded, low-poly outdoor detail with open paths, entrances and arrival views. */
export function addLandscape({ layout, stops, walking, terrain, mat, riverCurve, reserved = [] }) {
  const group = new THREE.Group(); group.name = 'Village landscape planting';
  const existing = [...reserved];
  terrain.updateMatrixWorld(true);
  terrain.traverse(o => {
    if (o.isMesh && (o.name.includes('entrance') || o.name.includes('forecourt') || o.name.includes('planting') || o.parent.name === 'Pathside wooden bench')) {
      existing.push(new THREE.Box3().setFromObject(o));
    }
  });
  terrain.add(group);
  let seed = 21731;
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
  const choose = values => values[Math.floor(random() * values.length)];
  const paths = walking.routes.flatMap(r => r.points.filter((_, i) => i % 2 === 0));
  const river = riverCurve.getPoints(300), placed = [], counts = {};
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const overlaps = (p, radius, b) => p.x + radius > b.min.x && p.x - radius < b.max.x && p.z + radius > b.min.z && p.z - radius < b.max.z;
  function sightDistance(p, pose) {
    const a = new THREE.Vector2(pose.eye.x, pose.eye.z), b = new THREE.Vector2(pose.target.x, pose.target.z), v = b.clone().sub(a);
    const t = THREE.MathUtils.clamp(new THREE.Vector2(p.x, p.z).sub(a).dot(v) / v.lengthSq(), 0, 1);
    return a.addScaledVector(v, t).distanceTo(new THREE.Vector2(p.x, p.z));
  }
  function clear(p, radius) {
    return stops.every(s => !overlaps(p, radius + 1.1, s.box))
      && existing.every(b => !overlaps(p, radius + .5, b))
      && paths.every(q => distance(p, q) > radius + 3)
      && river.every(q => distance(p, q) > radius + 3.4)
      && walking.poses.every(pose => sightDistance(p, pose) > radius + 2)
      && placed.every(q => distance(p, q.p) > radius + q.radius + .5);
  }
  function mesh(parent, name, geometry, color, position, scale = [1, 1, 1]) {
    const o = new THREE.Mesh(geometry, mat(color)); o.name = name; o.position.set(...position); o.scale.set(...scale); parent.add(o); return o;
  }
  function plant(type, p, size) {
    const radius = type === 'tree' ? size * .4 : size;
    if (!clear(p, radius)) return false;
    const patch = new THREE.Group(); patch.name = `Landscape ${type}`; patch.position.set(p.x, groundHeight(layout, p.x, p.z), p.z); patch.rotation.y = random() * Math.PI * 2; group.add(patch);
    placed.push({ p: p.clone(), radius }); counts[type] = (counts[type] || 0) + 1;
    if (type === 'tree') {
      const leaves = choose([['#3e702c', '#639132', '#86a944'], ['#487b35', '#76a43b', '#9db84d'], ['#286b46', '#3f8852', '#6ba15b']]);
      mesh(patch, 'Faceted tree trunk', new THREE.CylinderGeometry(.15, .27, size * .62, 7), '#805032', [0, size * .31, 0]);
      if (random() < .48) {
        for (let j = 0; j < 3; j++) {
          const g = new THREE.ConeGeometry(size * (.35 - j * .075), size * .48, 7, 2);
          const a = g.attributes.position;
          for (let k = 0; k < a.count; k++) { const y = a.getY(k), f = 1 + .08 * Math.sin(a.getX(k) * 4 + a.getZ(k) * 3 + y); a.setX(k, a.getX(k) * f); a.setZ(k, a.getZ(k) * f); }
          g.computeVertexNormals();mesh(patch, 'Folded evergreen canopy', g, leaves[j], [0, size * (.43 + j * .22), 0]);
        }
      } else {
        for (let j = 0; j < 4; j++) {
          const angle = j * 2.4;
          mesh(patch, 'Rounded leafy canopy', new THREE.IcosahedronGeometry(size * .23, 1), leaves[j % 3], [Math.sin(angle) * size * .14, size * (.66 + (j % 2) * .16), Math.cos(angle) * size * .14], [1, 1.15, 1]);
        }
      }
    } else if (type === 'boulder') {
      const g = new THREE.IcosahedronGeometry(1, 1), a = g.attributes.position;
      for (let j = 0; j < a.count; j++) {const f = 1 + .14 * Math.sin(a.getX(j) * 8 + a.getZ(j) * 6);a.setXYZ(j, a.getX(j) * f, a.getY(j) * f, a.getZ(j) * f);}
      g.computeVertexNormals();mesh(patch, 'Weathered landscape rock', g, choose(['#777d69', '#93957c', '#a6a18a', '#737b78']), [0, size * .28, 0], [size * .9, size * .58, size * .75]);
      mesh(patch, 'Small companion stone', new THREE.IcosahedronGeometry(size * .28, 0), '#a8a084', [size * .6, size * .12, size * .35], [1, .7, 1]);
    } else if (type === 'bush') {
      for (let j = 0; j < 3; j++) mesh(patch, 'Leafy low shrub', new THREE.IcosahedronGeometry(size * .55, 1), ['#3e783b', '#62943c', '#80ab49'][j], [(j - 1) * size * .35, size * (.35 + (j % 2) * .16), Math.sin(j * 3) * size * .2], [1, .85, 1]);
    } else {
      for (let j = 0; j < 6; j++) {
        const x = (random() - .5) * size, z = (random() - .5) * size, h = .3 + random() * .4;
        const blade = mesh(patch, 'Meadow grass blade', new THREE.ConeGeometry(.07, h, 3), choose(['#508035', '#739b39', '#a1b855']), [x, h / 2, z]);blade.rotation.z = (random() - .5) * .5;
        if (j % 2 === 0) mesh(patch, 'Small meadow flower', new THREE.IcosahedronGeometry(.11, 0), choose(['#e4b741', '#e9d49c', '#a287ba']), [x, h, z], [1, .55, 1]);
      }
    }
    return true;
  }
  // Groves and mixed understory in the spaces between locations, not a grid.
  for (const [type, desired, lo, hi] of [['tree', 66, 4.5, 8], ['bush', 95, .7, 1.45], ['boulder', 68, .6, 1.45], ['meadow', 95, .6, 1.1]]) {
    for (let attempt = 0; attempt < 2200 && (counts[type] || 0) < desired; attempt++) {
      const p = new THREE.Vector3(-88 + random() * 178, 0, -143 + random() * 207);
      plant(type, p, lo + random() * (hi - lo));
    }
  }
  // A deliberate line of irregular bank stones and reeds makes the river legible.
  for (let i = 1; i < 70; i++) {
    const t = i / 70, p = riverCurve.getPointAt(t), d = riverCurve.getTangentAt(t), side = new THREE.Vector3(-d.z, 0, d.x).normalize();
    p.addScaledVector(side, (i % 2 ? 1 : -1) * (5 + random() * 2));
    plant(i % 3 === 0 ? 'boulder' : 'meadow', p, .6 + random() * .4);
  }
  group.updateMatrixWorld(true);
  const bounds = [];group.traverse(o => { if (o.isMesh) bounds.push(new THREE.Box3().setFromObject(o).expandByScalar(.2)); });
  // Batch the small details by material/name so more planting does not mean
  // thousands of extra draw calls. Keep the individual bounds for route checks.
  const batches = new Map();
  group.traverse(o => {
    if (!o.isMesh) return;
    const key = o.name + o.material.color.getHexString();
    if (!batches.has(key)) batches.set(key, { name: o.name, material: o.material, geometries: [] });
    batches.get(key).geometries.push(o.geometry.clone().applyMatrix4(o.matrixWorld));
    o.geometry.dispose();
  });
  group.clear();
  for (const batch of batches.values()) {
    const geometry = mergeGeometries(batch.geometries);
    const o = new THREE.Mesh(geometry, batch.material); o.name = batch.name; group.add(o);
    batch.geometries.forEach(g => g.dispose());
  }
  return { check: () => {
    const collisions = [];
    for (let i = 0; i < walking.routes.length; i++) for (let j = 0; j <= 200; j++) if (bounds.some(b => b.containsPoint(walking.viewAt(i + j / 200).eye))) collisions.push(i + j / 200);
    return { seed: 21731, counts, camera_collisions: collisions, clear_arrival_views: placed.every(q => walking.poses.every(pose => sightDistance(q.p, pose) > q.radius + 2)), detail_parts: bounds.length, render_meshes: batches.size };
  } };
}
