import * as THREE from 'three';
import { MarchingCubes } from 'three/addons/objects/MarchingCubes.js';
import { mergeGeometries, mergeVertices } from 'three/addons/utils/BufferGeometryUtils.js';

const smooth = THREE.MathUtils.smoothstep;
const tau = Math.PI * 2;
const saddle = new THREE.Color('#403a35'),
  flank = new THREE.Color('#938270');
const charcoal = new THREE.Color('#2c2825'),
  cream = new THREE.Color('#ded4c0');
const mask = (distance, width) => Math.exp(-((distance / width) ** 2));

// Tile short, slightly slanted guard hairs over the larger painted markings.
function furTexture() {
  const size = 128,
    pixels = new Uint8Array(size * size * 4).fill(230);
  let seed = 127;
  function random() {
    seed = (Math.imul(1664525, seed) + 1013904223) >>> 0;
    return seed / 4294967296;
  }
  for (let i = 0; i < 5200; i++) {
    const x = Math.floor(random() * size),
      y = Math.floor(random() * size);
    const tone = 160 + random() * 95,
      drift = random() - 0.5;
    for (let k = 0; k < 5; k++) {
      const j = 4 * (((y + k) % size) * size + ((x + Math.round(drift * k) + size) % size));
      pixels.fill((pixels[j] + tone) / 2, j, j + 3);
    }
  }
  for (let i = 3; i < pixels.length; i += 4) pixels[i] = 255;
  pixels.fill(255, 0, 4);
  const map = new THREE.DataTexture(pixels, size, size);
  map.wrapS = map.wrapT = THREE.RepeatWrapping;
  map.generateMipmaps = true;
  map.minFilter = THREE.LinearMipmapLinearFilter;
  map.magFilter = THREE.LinearFilter;
  map.needsUpdate = true;
  return map;
}

export function makeTabby() {
  const material = new THREE.MeshStandardMaterial({
    map: furTexture(),
    vertexColors: true,
    roughness: 1,
    side: THREE.DoubleSide
  });
  const pieces = [];
  function add(g, tint, fur = true) {
    const pos = g.attributes.position,
      uv = g.attributes.uv;
    const colors = new Float32Array(pos.count * 3);
    for (let i = 0; i < pos.count; i++) {
      const c = typeof tint === 'function' ? tint(pos.getX(i), pos.getY(i), pos.getZ(i)) : tint;
      c.toArray(colors, i * 3);
      if (fur) uv.setXY(i, uv.getX(i) * 3, uv.getY(i) * 1.5);
      else uv.setXY(i, 0.5 / 128, 0.5 / 128);
    }
    g.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    pieces.push(g);
    return g;
  }
  function skin(transform, tint, segments = 40, rings = 28) {
    const g = add(new THREE.SphereGeometry(1, segments, rings), tint);
    const p = g.attributes.position;
    for (let i = 0; i < p.count; i++) p.setXYZ(i, ...transform(p.getX(i), p.getY(i), p.getZ(i)));
    g.computeVertexNormals();
  }
  function oval(x, y, z, rx, ry, rz, tint) {
    add(new THREE.SphereGeometry(1, 20, 14), tint)
      .scale(rx, ry, rz)
      .translate(x, y, z);
  }
  function strand(points, radius, tint, fur = false, tipScale = 1) {
    const line = new THREE.CatmullRomCurve3(points.map((p) => new THREE.Vector3(...p)));
    const tube = new THREE.TubeGeometry(line, 16, radius, 8, false);
    const position = tube.attributes.position,
      uv = tube.attributes.uv;
    const center = new THREE.Vector3(),
      vertex = new THREE.Vector3();
    for (let i = 0; i < position.count; i++) {
      const t = uv.getX(i);
      line.getPointAt(t, center);
      vertex.fromBufferAttribute(position, i).lerp(center, (1 - tipScale) * t);
      position.setXYZ(i, vertex.x, vertex.y, vertex.z);
    }
    return add(tube, tint, fur);
  }
  const headX = 0.064,
    headY = 0.531,
    headZ = 0.223;
  const faceZ = (x, y) =>
    headZ + 0.13 * Math.sqrt(Math.max(0, 1 - ((x - headX) / 0.15) ** 2 - ((y - headY) / 0.127) ** 2));

  // Blend the chest, hips, muzzle and limbs into one continuous coat. Only visit
  // the small grid box around each shape while building the signed distance field.
  const resolution = 100,
    halfSize = 0.54,
    centerY = 0.42;
  const cellSize = (halfSize * 2) / resolution;
  const surface = new MarchingCubes(resolution, material, false, false, 60000);
  surface.isolation = 0;
  surface.field.fill(-0.08);
  function ellipsoid(x, y, z, rx, ry, rz, blend, leanZ = 0) {
    const extent = [rx + blend, ry + blend, rz + blend + Math.abs(leanZ)];
    const center = [x, y - centerY, z];
    const lo = center.map((v, k) => Math.max(1, Math.floor((v - extent[k] + halfSize) / cellSize)));
    const hi = center.map((v, k) =>
      Math.min(resolution - 2, Math.ceil((v + extent[k] + halfSize) / cellSize))
    );
    for (let iz = lo[2]; iz <= hi[2]; iz++)
      for (let iy = lo[1]; iy <= hi[1]; iy++) {
        const dy = (iy * cellSize - halfSize - center[1]) / ry;
        const dz = (iz * cellSize - halfSize - z - leanZ * dy) / rz;
        for (let ix = lo[0]; ix <= hi[0]; ix++) {
          const dx = (ix * cellSize - halfSize - x) / rx;
          const r = Math.hypot(dx, dy, dz);
          const distance =
            r === 0 ? Math.min(rx, ry, rz) : ((1 - r) * r) / Math.hypot(dx / rx, dy / ry, dz / rz);
          const i = ix + resolution * (iy + resolution * iz);
          const prev = surface.field[i],
            h = Math.max(0, blend - Math.abs(distance - prev));
          surface.field[i] = Math.max(prev, distance) + (h * h) / (4 * blend);
        }
      }
  }
  ellipsoid(-0.046, 0.369, -0.207, 0.177, 0.199, 0.224, 0.026);
  ellipsoid(0.002, 0.369, -0.012, 0.163, 0.165, 0.243, 0.059);
  ellipsoid(0.049, 0.368, 0.139, 0.139, 0.183, 0.159, 0.046, 0.028);
  ellipsoid(headX, 0.453, 0.186, 0.117, 0.143, 0.111, 0.037, 0.05);
  ellipsoid(headX, headY, headZ, 0.15, 0.127, 0.13, 0.021);
  // The bridge projects between the eyes and tapers down towards the nose leather.
  ellipsoid(headX, 0.512, 0.34, 0.028, 0.055, 0.026, 0.012, -0.023);
  for (const side of [-1, 1]) {
    ellipsoid(headX + side * 0.029, 0.483, 0.339, 0.039, 0.034, 0.038, 0.013);

    for (const back of [false, true]) {
      const x = side * (back ? 0.105 : 0.081) + (back ? -0.055 : 0.046);
      const z = (back ? -0.276 : 0.156) + side * (back ? 0.032 : -0.018);
      if (back) {
        ellipsoid(x, 0.279, z + 0.012, 0.075, 0.143, 0.1, 0.038, 0.018);
        ellipsoid(x, 0.175, z - 0.008, 0.047, 0.092, 0.048, 0.025, 0.052);
      } else {
        ellipsoid(x, 0.269, z - 0.008, 0.053, 0.136, 0.058, 0.036, -0.007);
      }
      ellipsoid(
        x,
        back ? 0.099 : 0.129,
        z - (back ? 0.038 : 0),
        0.034,
        back ? 0.08 : 0.114,
        0.036,
        0.02,
        back ? -0.018 : -0.012
      );
      ellipsoid(x, 0.032, z + (back ? -0.005 : 0.023), 0.043, 0.03, 0.057, 0.019);
      for (const toe of [-1, 0, 1]) {
        ellipsoid(
          x + toe * 0.019,
          0.022,
          z + (back ? 0.025 : 0.056) - Math.abs(toe) * 0.004,
          0.017,
          0.018,
          0.021,
          0.008
        );
      }
    }
  }
  surface.update();
  const body = new THREE.BufferGeometry();
  for (const name of ['position', 'normal']) {
    const data = surface.geometry.attributes[name].array.slice(0, surface.count * 3);
    body.setAttribute(name, new THREE.BufferAttribute(data, 3));
  }
  surface.geometry.dispose();
  body.scale(halfSize, halfSize, halfSize).translate(0, centerY, 0);
  const bodyUV = new THREE.Float32BufferAttribute(new Float32Array(surface.count * 2), 2);
  body.setAttribute('uv', bodyUV);
  const pos = body.attributes.position;
  for (let i = 0; i < pos.count; i++)
    bodyUV.setXY(i, pos.getZ(i) * 2.2 + pos.getX(i) * 1.4, pos.getY(i) * 2.8 + 0.18 * pos.getX(i));
  body.setIndex(Array.from({ length: surface.count }, (_, i) => i));
  add(body, (x, y, z) => {
    const fx = (x - headX) / 0.15,
      fy = (y - headY) / 0.127,
      ax = Math.abs(fx);
    const crown = smooth(y, 0.26, 0.53);
    const c = flank.clone().lerp(saddle, 0.95 * crown);
    // Uneven mackerel stripes break and fork over the flanks and wrap down the legs.
    const rib = Math.cos(z * 66 + 1.75 * Math.sin(10 * y + 8 * z) + 4.3 * y + 0.8 * Math.sin(z * 21 + x * 4));
    const branch = Math.cos(z * 111 + 15 * y + 1.3 * Math.sin(13 * y));
    const legBand = Math.cos(101 * y + 13 * z + 1.1 * Math.sin(18 * x + 11 * y));
    const toBody = smooth(y, 0.18, 0.35);
    const tiger = THREE.MathUtils.lerp(
      smooth(legBand, 0.21, 0.86),
      Math.max(smooth(rib, 0.14, 0.88), 0.44 * smooth(branch, 0.42, 0.9)),
      toBody
    );
    c.lerp(charcoal, 0.88 * tiger).lerp(cream, 0.78 * (1 - smooth(y, 0.027, 0.057)));
    // A pale chest with narrow, broken necklaces beneath the chin.
    const bib = smooth(z, 0.175, 0.274) * (1 - smooth(Math.abs(x - headX), 0.05, 0.125));
    c.lerp(cream, 0.89 * bib * (1 - 0.69 * smooth(Math.cos(117 * y - 8 * Math.abs(fx)), 0.44, 0.92)));
    const face = smooth(y, 0.399, 0.493) * smooth(z, 0.105, 0.203);
    const faceColor = flank.clone().lerp(saddle, 0.9 * smooth(fy, 0.47, 0.95));
    const eyes = mask((ax - 0.46) / 0.32, 1) * mask((fy - 0.17) / 0.22, 1);
    faceColor.lerp(cream, 0.84 * eyes);
    const forehead =
      Math.max(
        mask(ax - 0.09 - 0.35 * Math.abs(fy - 0.52), 0.065),
        mask(ax - 0.47 - 0.11 * Math.sin(4 * fy), 0.07)
      ) * smooth(fy, 0.2, 0.45);
    const cheek =
      Math.max(mask(fy + 0.14 - 0.5 * (ax - 0.5), 0.074), mask(fy + 0.38 - 0.37 * (ax - 0.5), 0.08)) *
      smooth(ax, 0.38, 0.82);
    const tear = mask(ax - 0.23 - 0.43 * (fy + 0.1), 0.068) * mask(fy + 0.03, 0.23);
    faceColor.lerp(charcoal, Math.max(forehead, cheek, tear) * 0.96);
    const bridge = mask(ax, 0.22) * mask(fy + 0.02, 0.42);
    faceColor.lerp(saddle, 0.45 * bridge);
    const noseSides = mask(ax - 0.18 - 0.25 * (fy + 0.15), 0.072) * mask(fy + 0.01, 0.26);
    faceColor.lerp(charcoal, 0.65 * noseSides);
    c.lerp(faceColor, face);
    const muzzle = smooth(z, 0.307, 0.348) * (1 - smooth(ax, 0.38, 0.63)) * (1 - smooth(y, 0.489, 0.518));
    const chin = smooth(z, 0.246, 0.325) * smooth(y, 0.4, 0.45) * (1 - smooth(y, 0.454, 0.486));
    const ruff = mask(y - 0.426 - 0.048 * ax, 0.024) * smooth(z, 0.19, 0.285);
    const speckle = Math.sin(192 * x + 63 * y + 27 * Math.sin(18 * z)) * Math.sin(164 * z + 41 * y);
    return c.lerp(cream, Math.max(muzzle, chin)).multiplyScalar((1 - 0.59 * ruff) * (0.96 + 0.13 * speckle));
  });
  // Trace facial details onto the sculpt rather than guessing their depth on the muzzle.
  body.computeBoundingSphere();
  const ray = new THREE.Raycaster();
  const coatMesh = new THREE.Mesh(body, material);
  function muzzlePoint(dx, y) {
    ray.set(new THREE.Vector3(headX + dx, y, 0.49), new THREE.Vector3(0, 0, -1));
    const hit = ray.intersectObject(coatMesh, false)[0];
    return [headX + dx, y, hit.point.z + 0.0009];
  }
  const innerEar = new THREE.Color('#937b6b');
  for (const side of [-1, 1]) {
    skin(
      (x, y, z) => {
        const t = (y + 1) * 0.5,
          w = 1.13 - 0.92 * t;
        return [
          headX + side * (0.082 + 0.054 * t) + x * 0.076 * w,
          0.572 + t * 0.155,
          headZ - 0.015 - 0.026 * t + (z - 0.65 * smooth(z, 0.38, 1)) * 0.044 * w
        ];
      },
      (x, y, z) => saddle.clone().lerp(innerEar, smooth(z, 0.56, 0.96) * (1 - smooth(y, 0.58, 0.93))),
      32,
      24
    );

    // An almond-shaped eyelid sits directly on the curved face, with no exposed sclera.
    const eye = new THREE.PlaneGeometry(1, 1, 32, 18),
      p = eye.attributes.position;
    const iris = new THREE.Color('#928673'),
      ink = new THREE.Color('#211f1d');
    add(
      eye,
      (x, y) => {
        const rim = Math.max(smooth(Math.abs(y), 0.31, 0.47), smooth(Math.abs(x), 0.34, 0.48));
        const pupil = 1 - smooth(Math.hypot(x * 2.8, y * 1.35), 0.7, 0.81);
        return iris
          .clone()
          .lerp(ink, Math.max(rim, pupil))
          .lerp(cream, 0.72 * mask(Math.hypot(x + 0.14, y - 0.16), 0.047));
      },
      false
    );
    for (let i = 0; i < p.count; i++) {
      const u = p.getX(i) * 2,
        v = p.getY(i) * 2;
      const px = headX + side * 0.068 + u * 0.035;
      const py = headY + 0.023 + side * u * 0.007 + v * 0.019 * (1 - u * u);
      p.setXYZ(i, px, py, faceZ(px, py) + 0.0018 + 0.005 * (1 - u * u) * (1 - v * v));
    }
    eye.computeVertexNormals();

    strand(
      [muzzlePoint(0, 0.477), muzzlePoint(side * 0.014, 0.473), muzzlePoint(side * 0.029, 0.474)],
      0.0009,
      charcoal
    );
    for (const [dx, dy] of [
      [0.029, 0.494],
      [0.04, 0.491],
      [0.049, 0.486],
      [0.034, 0.484]
    ]) {
      const [x, y, z] = muzzlePoint(side * dx, dy);
      oval(x, y, z, 0.0015, 0.0012, 0.0009, saddle);
    }
    for (let w = 0; w < 4; w++) {
      const start = muzzlePoint(side * (0.044 + 0.003 * w), 0.483 + 0.004 * w);
      strand(
        [
          start,
          [headX + side * 0.093, 0.491 + 0.008 * w, 0.37],
          [headX + side * (0.174 + 0.006 * w), 0.46 + 0.016 * w, 0.339]
        ],
        0.00062,
        new THREE.Color('#bbaf9b'),
        false,
        0.15
      );
    }
  }
  skin(
    (x, y, z) => [headX + x * (0.015 + 0.007 * y), 0.506 + y * 0.012, 0.37 + z * 0.008],
    new THREE.Color('#70564e'),
    24,
    16
  );
  strand([muzzlePoint(0, 0.501), muzzlePoint(0, 0.488), muzzlePoint(0, 0.477)], 0.0018, charcoal);

  strand(
    [
      [-0.082, 0.386, -0.353],
      [-0.161, 0.279, -0.423],
      [-0.225, 0.141, -0.365],
      [-0.242, 0.103, -0.237],
      [-0.248, 0.13, -0.108]
    ],
    0.031,
    (x, y, z) => saddle.clone().lerp(charcoal, smooth(Math.cos(z * 69 + y * 15), 0.05, 0.83)),
    true,
    0.55
  );

  oval(-0.248, 0.13, -0.108, 0.017, 0.017, 0.017, charcoal);
  const merged = mergeGeometries(pieces);
  merged.normalizeNormals();
  const geometry = mergeVertices(merged);
  merged.dispose();
  for (const piece of pieces) piece.dispose();
  const cat = new THREE.Mesh(geometry, material);
  // Pitch the stance to meet the slight incline between the paws on the limestone slab.
  geometry.rotateX(-0.09);
  cat.name = 'Tabby';
  return cat;
}
