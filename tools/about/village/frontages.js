import * as THREE from 'three';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { groundHeight } from './walking-world.js';

const palettes = {
  stone: { paving: '#c9bca3', edge: '#a49b84', soil: '#6d5b43', foliage: '#6b8252' },
  brick: { paving: '#b47b5b', edge: '#955e45', soil: '#715343', foliage: '#75844f' },
  timber: { paving: '#aa8a5e', edge: '#7e6548', soil: '#685640', foliage: '#587653' },
  gravel: { paving: '#b6ad8c', edge: '#8e9576', soil: '#6d6043', foliage: '#789057' },
};

/** Build reversible exterior additions, leaving every source model untouched. */
export function addFrontages({ layout, stops, walking, terrain, mat }) {
  const additions = new THREE.Group(); additions.name = 'Village entrances and planting'; terrain.add(additions);
  const solids = [], reports = [], routePoints = walking.routes.flatMap(r => r.points.filter((_, i) => i % 3 === 0));
  const yAxis = new THREE.Vector3(0, 1, 0);
  const occupied = stops.map(s => s.box.clone().expandByScalar(.12));
  const approachObstacles = occupied.map(b => b.clone().expandByScalar(1.25));
  function clearProp(p, radius, own) {
    return routePoints.every(q => Math.hypot(q.x - p.x, q.z - p.z) > radius + 1.9)
      && occupied.every((b, i) => i === own || p.x + radius < b.min.x || p.x - radius > b.max.x || p.z + radius < b.min.z || p.z - radius > b.max.z);
  }
  function block(group, name, p, size, color, yaw = 0) {
    const mesh = new THREE.Mesh(new RoundedBoxGeometry(...size, 1, Math.min(.055, size[1] / 4)), mat(color));
    mesh.name = name; mesh.position.copy(p); mesh.rotation.y = yaw; group.add(mesh); solids.push(mesh); return mesh;
  }
  function shrub(group, p, color, seed) {
    for (let j = 0; j < 3; j++) {
      const mesh = new THREE.Mesh(new THREE.IcosahedronGeometry(.34 + j * .065, 1), mat(color));
      mesh.name = 'Low culinary herb planting'; mesh.position.copy(p).add(new THREE.Vector3(Math.sin(seed + j * 2) * .27, .32 + j * .06, Math.cos(seed + j * 2) * .22));
      mesh.scale.y = .8 + (j % 2) * .2; group.add(mesh); solids.push(mesh);
    }
  }
  for (let i = 0; i < stops.length; i++) {
    const s = stops[i], settings = s.entrance, palette = palettes[settings.surface];
    const group = new THREE.Group(); group.name = `${s.id} exterior arrival`; additions.add(group);
    const half = s.footprint.clone().multiplyScalar(.5);
    const direction = s.direction.clone().applyAxisAngle(yAxis, -s.yaw);
    const alongX = Math.abs(direction.x) / half.x > Math.abs(direction.z) / half.z;
    const normalLocal = alongX ? new THREE.Vector3(Math.sign(direction.x), 0, 0) : new THREE.Vector3(0, 0, Math.sign(direction.z));
    const extent = alongX ? half.x : half.z;
    const t = extent / Math.abs(alongX ? direction.x : direction.z);
    const entryLocal = direction.clone().multiplyScalar(t);
    if (alongX) entryLocal.z = THREE.MathUtils.clamp(entryLocal.z, -half.z + 2.5, half.z - 2.5);
    else entryLocal.x = THREE.MathUtils.clamp(entryLocal.x, -half.x + 2.5, half.x - 2.5);
    // Optional normalized position along the open edge avoids fixed features
    // such as the cabin stream without editing the source scene.
    if (settings.edgePosition !== undefined) {
      if (alongX) entryLocal.z = settings.edgePosition * (half.z - 2.5);
      else entryLocal.x = settings.edgePosition * (half.x - 2.5);
    }
    const normal = normalLocal.applyAxisAngle(yAxis, s.yaw);
    const tangent = new THREE.Vector3(normal.z, 0, -normal.x);
    const entry = entryLocal.applyAxisAngle(yAxis, s.yaw).add(new THREE.Vector3(...s.position));
    const yaw = Math.atan2(normal.x, normal.z);
    const at = (u, v, y) => entry.clone().addScaledVector(tangent, u).addScaledVector(normal, v).setY(y);
    // Sample an exposed floor just inside the open edge. Ignore furnishings and
    // wall tops; the highest low, upward-facing surface is the entry threshold.
    const sample = entry.clone().addScaledVector(normal, -.55); sample.y = s.box.max.y + 2;
    const ray = new THREE.Raycaster(sample, new THREE.Vector3(0, -1, 0));
    const hits = ray.intersectObject(s.anchor, true).filter(hit => hit.face
      && hit.face.normal.clone().transformDirection(hit.object.matrixWorld).y > .75
      && hit.point.y >= s.position[1] + .08 && hit.point.y <= s.position[1] + Math.min(1.8, s.size.y * .27));
    const base = groundHeight(layout, entry.x + normal.x * 3, entry.z + normal.z * 3) + .17;
    const floor = Math.max(base, hits.length ? Math.max(...hits.map(hit => hit.point.y)) : s.position[1] + settings.floorFallback);
    const count = floor - base > .22 ? Math.ceil((floor - base) / .24) : 0;
    const stairRun = count * .46, width = settings.width;
    const patioStart = stairRun + .55, patioDepth = settings.depth;
    const patioY = base + .02;
    const center = at(0, patioStart + patioDepth / 2, patioY - .09);
    block(group, 'Inset entrance forecourt', center, [width, .18, patioDepth], palette.edge, yaw);
    // Individual pavers/planks add scale without changing the room's materials.
    const columns = settings.surface === 'timber' ? 1 : Math.max(2, Math.round(width / .95));
    const rows = Math.max(3, Math.round(patioDepth / (settings.surface === 'timber' ? .34 : .8)));
    for (let row = 0; row < rows; row++) for (let col = 0; col < columns; col++) {
      const u = -width / 2 + (col + .5) * width / columns, v = patioStart + (row + .5) * patioDepth / rows;
      block(group, settings.surface === 'timber' ? 'Outdoor timber plank' : 'Entrance paving stone', at(u, v, patioY + .01), [width / columns - .035, .07, patioDepth / rows - .035], (row + col) % 5 ? palette.paving : palette.edge, yaw);
    }
    block(group, 'Doorway landing', at(0, .2, floor - .07), [3.1, .14, .7], palette.paving, yaw);
    for (let step = 0; step < count; step++) {
      const rise = (floor - base) * (step + 1) / count;
      block(group, 'Broad entrance step', at(0, .55 + stairRun - (step + .5) * .46, base + rise / 2), [3.1, rise, .48], palette.paving, yaw);
    }
    // Each branch begins at the actual walking-route stop, not an estimated path.
    const arrival = walking.poses[i].eye.clone(); arrival.y = groundHeight(layout, arrival.x, arrival.z) + .18;
    const end = at(0, patioStart + patioDepth - .1, patioY + .06);
    const control = end.clone().addScaledVector(normal, 2.5);
    const curve = new THREE.QuadraticBezierCurve3(arrival, control, end);
    const points = curve.getPoints(48);
    const intersects = points.some(p => approachObstacles.some((b, j) => j !== i && p.x > b.min.x && p.x < b.max.x && p.z > b.min.z && p.z < b.max.z));
    if (intersects) throw Error(`Entrance approach crosses another location: ${s.id}`);
    const vertices = [], indices = [];
    for (let j = 0; j < points.length; j++) {
      const p = points[j], heading = curve.getTangent(j / (points.length - 1)), side = new THREE.Vector3(-heading.z, 0, heading.x).normalize();
      for (const sign of [-1, 1]) {
        const v = p.clone().addScaledVector(side, sign * 1.25);
        v.y = THREE.MathUtils.lerp(groundHeight(layout, v.x, v.z) + .19, end.y, THREE.MathUtils.smoothstep(j / 48, .65, 1));
        vertices.push(...v.toArray());
      }
      if (j < points.length - 1) { const n = j * 2; indices.push(n, n + 1, n + 2, n + 1, n + 3, n + 2); }
    }
    const geometry = new THREE.BufferGeometry(); geometry.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3)); geometry.setIndex(indices); geometry.computeVertexNormals();
    const approach = new THREE.Mesh(geometry, mat(palette.paving)); approach.name = 'Path-to-entrance connection'; group.add(approach);
    // Low beds flank the opening, keeping the interior and camera corridor clear.
    let beds = 0;
    for (const sign of [-1, 1]) {
      const p = at(sign * (width / 2 + .72), patioStart + patioDepth * .48, 0);
      if (!clearProp(p, 1.3, i)) continue;
      const y = groundHeight(layout, p.x, p.z);
      block(group, 'Low stone planting border', p.clone().setY(y + .18), [1.25, .35, Math.min(3, patioDepth)], palette.edge, yaw);
      block(group, 'Recessed herb bed soil', p.clone().setY(y + .35), [1.07, .07, Math.min(2.8, patioDepth - .15)], palette.soil, yaw);
      for (let j = 0; j < 3; j++) shrub(group, p.clone().addScaledVector(normal, (j - 1) * .8).setY(y + .38), palette.foliage, i + j);
      beds++;
    }
    // Small base plantings soften the corners without surrounding every room
    // with a continuous hedge or hiding the furniture inside.
    let cornerPlantings = 0;
    for (const sign of [-1, 1]) {
      const p = at(sign * 3.5, .9, 0);
      const local = p.clone().sub(new THREE.Vector3(...s.position)).applyAxisAngle(yAxis, -s.yaw);
      if (Math.abs(local.x) > half.x + 1.5 || Math.abs(local.z) > half.z + 1.5 || !clearProp(p, .8, i)) continue;
      const soil = new THREE.Mesh(new THREE.CylinderGeometry(.64, .69, .07, 9), mat(palette.soil));
      soil.name = 'Inset corner planting pocket'; soil.position.copy(p).setY(groundHeight(layout, p.x, p.z) + .025); group.add(soil);
      p.y = groundHeight(layout, p.x, p.z) - .045; shrub(group, p, palette.foliage, i + sign); cornerPlantings++;
    }
    reports.push({ id: s.id, surface: settings.surface, floor_height: floor, sampled_floor: hits.length > 0, steps: count, planting_beds: beds, corner_plantings: cornerPlantings, approach_start: arrival.toArray(), approach_end: end.toArray(), threshold: entry.clone().setY(floor).toArray(), approach_clear_of_neighbors: !intersects });
  }
  additions.updateMatrixWorld(true);
  const bounds = solids.map(mesh => new THREE.Box3().setFromObject(mesh).expandByScalar(.2));
  function check() {
    const collisions = [];
    for (let i = 0; i < walking.routes.length; i++) for (let j = 0; j <= 200; j++) {
      const eye = walking.viewAt(i + j / 200).eye;
      if (bounds.some(b => b.containsPoint(eye))) collisions.push(i + j / 200);
    }
    return { entrances: reports, camera_collisions: collisions, added_meshes: solids.length };
  }
  return { check };
}
