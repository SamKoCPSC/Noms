import * as THREE from 'three';

export function groundHeight(layout, x, z) {
  const base = .18 - z * .014 + Math.sin(x * .045) * Math.cos(z * .035) * .3;
  // Level the ground around each unchanged room; blend into the wider landscape.
  let value = base;
  for (const stop of layout.stops) {
    const distance = Math.hypot(x - stop.position[0], z - stop.position[2]);
    const radius = stop.width * .76;
    const blend = 1 - THREE.MathUtils.smoothstep(distance, radius, radius + 7);
    value = THREE.MathUtils.lerp(value, stop.position[1] - .15, blend);
  }
  return value;
}

export function makeWalkingRoutes(layout, stops) {
  const step = 1.5, bounds = [-63, 63, -94, 55];
  const obstacles = stops.map(s => s.box.clone().expandByScalar(3.8));
  const physical = stops.map(s => s.box.clone().expandByScalar(1.8));
  const blocked = (x, z, boxes = obstacles) => boxes.some(b => x > b.min.x && x < b.max.x && z > b.min.z && z < b.max.z);
  const key = (x, z) => `${x},${z}`;
  const inBounds = (x, z) => x >= bounds[0] && x <= bounds[1] && z >= bounds[2] && z <= bounds[3];
  function nearest(p) {
    const x = Math.round(p.x / step), z = Math.round(p.z / step);
    for (let r = 0; r < 40; r++) {
      const candidates = [];
      for (let dx = -r; dx <= r; dx++) for (let dz = -r; dz <= r; dz++) {
        if (Math.max(Math.abs(dx), Math.abs(dz)) !== r) continue;
        if (inBounds(x + dx, z + dz) && !blocked((x + dx) * step, (z + dz) * step)) candidates.push([x + dx, z + dz]);
      }
      candidates.sort((a, b) => Math.hypot(a[0] * step - p.x, a[1] * step - p.z) - Math.hypot(b[0] * step - p.x, b[1] * step - p.z));
      if (candidates.length) return candidates[0];
    }
    throw Error('No accessible arrival point');
  }
  function clear(a, b) {
    const n = Math.ceil(Math.hypot(a[0] - b[0], a[1] - b[1]) * 3);
    for (let i = 0; i <= n; i++) {
      const t = n ? i / n : 0;
      if (blocked(THREE.MathUtils.lerp(a[0], b[0], t) * step, THREE.MathUtils.lerp(a[1], b[1], t) * step)) return false;
    }
    return true;
  }
  const arrivals = stops.map(stop => nearest(stop.frontage));
  function route(start, end) {
    const goal = key(...end), open = [{ p: start, g: 0, f: 0 }], costs = new Map([[key(...start), 0]]), parents = new Map();
    while (open.length) {
      open.sort((a, b) => b.f - a.f);
      const { p, g } = open.pop(), k = key(...p);
      if (g !== costs.get(k)) continue;
      if (k === goal) {
        let at = k, points = [];
        while (at) { points.push(at.split(',').map(Number)); at = parents.get(at); }
        points.reverse();
        const simplified = [points[0]];
        for (let i = 0; i < points.length - 1;) {
          let next = points.length - 1;
          while (next > i + 1 && !clear(points[i], points[next])) next--;
          simplified.push(points[next]); i = next;
        }
        const vectors = simplified.map(([x, z]) => new THREE.Vector3(x * step, 0, z * step));
        let curve = new THREE.CatmullRomCurve3(vectors, false, 'centripetal');
        // If the broad spline overshoots a plot, round the verified polyline's
        // corners locally instead. This avoids both clipping and abrupt turns.
        if (curve.getPoints(600).some(p => blocked(p.x, p.z, physical))) {
          for (const radius of [4, 2, 1, .5]) {
            curve = new THREE.CurvePath();
            let last = vectors[0];
            for (let i = 1; i < vectors.length - 1; i++) {
              const corner = vectors[i], before = vectors[i - 1], after = vectors[i + 1];
              const distance = Math.min(radius, corner.distanceTo(before) * .25, corner.distanceTo(after) * .25);
              const entry = corner.clone().addScaledVector(before.clone().sub(corner).normalize(), distance);
              const exit = corner.clone().addScaledVector(after.clone().sub(corner).normalize(), distance);
              curve.add(new THREE.LineCurve3(last, entry));
              curve.add(new THREE.QuadraticBezierCurve3(entry, corner, exit));
              last = exit;
            }
            curve.add(new THREE.LineCurve3(last, vectors.at(-1)));
            if (!curve.getPoints(600).some(p => blocked(p.x, p.z, physical))) break;
          }
        }
        curve.arcLengthDivisions = 1200;
        const points3 = Array.from({ length: 401 }, (_, i) => {
          const p = curve.getPointAt(i / 400); p.y = groundHeight(layout, p.x, p.z) + .055; return p;
        });
        return { curve, points: points3, length: curve.getLength() };
      }
      for (const [dx, dz] of [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [1, -1], [-1, 1], [-1, -1]]) {
        const next = [p[0] + dx, p[1] + dz], nk = key(...next), ng = g + Math.hypot(dx, dz);
        if (!inBounds(...next) || !clear(p, next) || (costs.has(nk) && costs.get(nk) <= ng)) continue;
        costs.set(nk, ng); parents.set(nk, k);
        open.push({ p: next, g: ng, f: ng + Math.hypot(next[0] - end[0], next[1] - end[1]) });
      }
    }
    throw Error('No outdoor path between locations');
  }
  const routes = arrivals.slice(1).map((end, i) => route(arrivals[i], end));
  const poses = arrivals.map(([x, z], i) => ({
    eye: new THREE.Vector3(x * step, groundHeight(layout, x * step, z * step) + layout.walk.eye_height, z * step),
    target: new THREE.Vector3(stops[i].position[0], stops[i].position[1] + THREE.MathUtils.clamp(stops[i].size.y * .32, 2.2, 3.2), stops[i].position[2]),
  }));
  function viewAt(progress) {
    const index = Math.min(stops.length - 1, Math.floor(progress)), t = progress - index;
    if (index === stops.length - 1 || t === 0) return { eye: poses[index].eye.clone(), target: poses[index].target.clone() };
    const r = routes[index], p = r.curve.getPointAt(t);
    p.y = groundHeight(layout, p.x, p.z) + layout.walk.eye_height;
    const ahead = r.curve.getPointAt(Math.min(1, t + 5 / r.length));
    ahead.y = groundHeight(layout, ahead.x, ahead.z) + layout.walk.eye_height - .15;
    const target = ahead;
    const leaving = 1 - THREE.MathUtils.smoothstep(t, 0, .18);
    const arriving = THREE.MathUtils.smoothstep(t, .78, 1);
    target.lerp(poses[index].target, leaving).lerp(poses[index + 1].target, arriving);
    return { eye: p, target };
  }
  return { routes, poses, viewAt, blocked: (x, z) => blocked(x, z, physical) };
}
