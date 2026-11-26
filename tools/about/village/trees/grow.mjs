// Build tree shapes in local coordinates, then fit each to its authored planting.
import { Vector3, Color } from 'three';
import { EXTMeshGPUInstancing, KHRMaterialsUnlit } from '@gltf-transform/extensions';
import { readFile } from 'node:fs/promises';

const v = (x, y, z) => new Vector3(x, y, z);
const sun = v(-0.6, 1, 0.5).normalize(),
  TAU = Math.PI * 2;
const polar = (angle, radius, rise = 0) => v(Math.cos(angle) * radius, rise, Math.sin(angle) * radius);
const colors = ['#93715a', '#826959', '#a47e5a', '#856f50'].map((hex) => new Color(hex));
const species = ['cherry', 'fir', 'maple', 'oak'];

function geometry() {
  return { positions: [], colors: [], indices: [], uv: [] };
}
function vertex(g, point, color, uv) {
  g.positions.push(point.x, point.y, point.z);
  g.colors.push(...color.toArray().map((c) => Math.max(0, Math.min(255, Math.round(c * 255)))));
  if (uv) g.uv.push(...uv);
}

function grow(tree, kind, bark, foliage, rand) {
  const { x, y, z, height: h, radius: r } = tree;
  const base = v(x, y, z),
    phase = rand() * TAU;
  const at = (a, d, t) => base.clone().add(polar(a, d * r, t * h));
  const barkColor = colors[kind];
  // Transport the ring around each bend. Long ridges carry the light down the trunk.
  function limb(spine, widths, sides = 7) {
    const first = bark.positions.length / 3,
      twist = rand() * TAU;
    const ridges = Array.from({ length: sides }, () => 0.82 + rand() * 0.32);
    let across;
    spine.forEach((p, j) => {
      const tangent = spine[Math.min(j + 1, spine.length - 1)]
        .clone()
        .sub(spine[Math.max(0, j - 1)])
        .normalize();
      across ??= Math.abs(tangent.x) < 0.8 ? v(1, 0, 0) : v(0, 0, 1);
      across.addScaledVector(tangent, -tangent.dot(across)).normalize();
      const side = tangent.clone().cross(across);
      for (let k = 0; k < sides; k++) {
        const a = twist + (k * TAU) / sides;
        const n = across.clone().multiplyScalar(Math.cos(a)).addScaledVector(side, Math.sin(a));
        const radius = widths[j] * ridges[k] * (j === 0 && sides === 12 && k % 3 === 0 ? 1.38 : 1);
        const tint = barkColor.clone().multiplyScalar((0.48 + 0.85 * Math.max(0, n.dot(sun))) * ridges[k]);
        vertex(bark, p.clone().addScaledVector(n, radius), tint);
        if (j) {
          const a0 = first + (j - 1) * sides + k,
            b0 = first + (j - 1) * sides + ((k + 1) % sides);
          bark.indices.push(a0, b0, b0 + sides, a0, b0 + sides, a0 + sides);
        }
      }
    });
  }
  // A folded sprig has a lifted midrib and a drooping (or raised) tip. Changing
  // pitch and roll between sprays leaves real windows through the crown.
  function spray(origin, angle, length, width, rise, bow, roll, shade) {
    const right = polar(angle + Math.PI / 2, Math.cos(roll), Math.sin(roll));
    const corner = foliage.positions.length / 3;
    const outward = polar(angle, 1),
      u = (kind % 2) * 0.5,
      vv = Math.floor(kind / 2) * 0.5;
    const lit = 0.22 * Math.cos(angle - 2.45);
    // One raised center and four corners give each cutout a folded silhouette.
    for (const [t, s] of [
      [0, -1],
      [0, 1],
      [0.52, 0],
      [1, -1],
      [1, 1]
    ]) {
      const p = origin
        .clone()
        .addScaledVector(outward, length * t)
        .addScaledVector(right, s * width * 0.5);
      p.y += rise * t + bow * Math.sin(t * Math.PI) - Math.abs(s) * width * 0.14;
      const light = Math.max(0.14, Math.min(1.32, shade + lit + 0.27 * t - 0.09 * Math.abs(s)));
      vertex(foliage, p, new Color(light, light * (0.97 + 0.03 * t), light * (1.08 - 0.12 * t)), [
        u + 0.004 + (s + 1) * 0.246,
        vv + 0.004 + (1 - t) * 0.492
      ]);
    }
    for (const i of [0, 2, 1, 0, 3, 2, 2, 3, 4, 1, 2, 4]) foliage.indices.push(corner + i);
  }

  if (kind === 1) {
    const lean = 0.1 + rand() * 0.09;
    const spine = [0, 0.12, 0.4, 0.68, 0.88, 0.985].map((t) => at(phase, lean * t * t, t));
    limb(
      spine,
      [0.05, 0.033, 0.027, 0.017, 0.009, 0.002].map((n) => n * h),
      9
    );
    // Overlapping but broken whorls. Each bough sweeps down from the stem,
    // then lifts at the tip and carries feathered sprays on alternate sides.
    for (let tier = 0; tier < 9; tier++) {
      const t = 0.26 + tier * 0.082,
        span = r * (1.13 - t) * 1.21;
      const count = tier < 5 ? 8 : 6;
      for (let j = 0; j < count; j++) {
        const a = phase + tier * 2.399 + ((j + rand() * 0.42) * TAU) / count;
        const len = span * (0.8 + rand() * 0.26);
        const start = at(phase, lean * t * t, t + rand() * 0.035);
        const mid = start.clone().add(polar(a, len * 0.49, -len * 0.31));
        const tip = start.clone().add(polar(a, len, -len * 0.24));
        limb(
          [start, mid, tip],
          [0.048, 0.031, 0.004].map((n) => n * len),
          4
        );
        for (let k = 0; k < 3; k++)
          for (const side of [-1, 1]) {
            const f = 0.12 + k * 0.25;
            const root = start.clone().lerp(tip, f);
            root.y -= Math.sin(f * Math.PI) * len * 0.19;
            const width = len * (0.51 - 0.21 * f);
            spray(
              root,
              a + side * (0.72 - 0.26 * f),
              len * (0.56 - 0.19 * f),
              width,
              -len * (0.22 - 0.08 * f),
              len * 0.09,
              side * (0.2 + rand() * 0.35),
              0.31 + t * 0.49 + rand() * 0.15
            );
          }
        spray(mid, a, len * 0.64, len * 0.37, -0.02 * len, 0.1 * len, (rand() - 0.5) * 0.5, 0.68 + t * 0.28);
      }
    }
    for (let i = 0; i < 6; i++) spray(spine[4], phase + i, 0.09 * r, 0.15 * r, 0.11 * h, 0.015 * h, i, 0.83);
    return;
  }

  const lean = 0.19 + rand() * 0.16;
  const trunk = [
    at(phase, 0, -0.005),
    at(phase + 0.65, 0.07, 0.08),
    at(phase + 0.5, lean, 0.3),
    at(phase + 0.08, lean * 0.85, 0.48),
    at(phase - 0.6, lean * 0.95, 0.65),
    at(phase - 0.7, lean * 1.16, 0.83)
  ];
  limb(
    trunk,
    [0.064, 0.044, 0.035, 0.027, 0.018, 0.006].map((n) => n * h),
    12
  );
  function tuft(center, a, scale) {
    const shade = 0.24 + (0.68 * (center.y - y)) / h + (0.28 * (center.x - x) * sun.x) / r;
    for (let layer = 0; layer < 2; layer++) {
      for (let j = 0; j < 6; j++) {
        const az = a + ((j + rand() * 0.45) * TAU) / 6 + layer * 0.7;
        const root = center.clone().add(polar(az, -0.06 * r, layer * 0.035 * h));
        spray(
          root,
          az,
          (0.35 + 0.12 * rand()) * r * scale,
          (0.5 + 0.13 * rand()) * r * scale,
          (layer ? 0.12 + rand() * 0.12 : -0.22 + rand() * 0.27) * r,
          (0.13 + 0.08 * rand()) * r,
          (rand() - 0.5) * 1.5,
          shade - 0.2 + layer * 0.24 + rand() * 0.16
        );
      }
    }
    for (let i = 0; i < 3; i++)
      spray(
        center,
        a + i * 2.1,
        0.2 * r * scale,
        0.39 * r * scale,
        0.3 * r * scale,
        0.05 * r,
        (rand() - 0.5) * 1.1,
        shade + 0.15
      );
  }
  // Fork into low, spreading scaffold limbs and progressively narrower upper
  // branches. Secondary twigs place the leaf clusters at different depths.
  for (let j = 0; j < 10; j++) {
    const t = j / 10,
      a = phase + j * 2.399 + (rand() - 0.5) * 0.52;
    const joint = trunk[2].clone().lerp(trunk[5], kind === 0 ? 0.12 + 0.76 * t : t);
    const reach = r * (kind === 0 ? 1.06 - 0.52 * t : 0.75 - 0.39 * t) * (0.83 + rand() * 0.26);
    const mid = joint.clone().add(polar(a + 0.23, reach * 0.52, h * (0.1 + 0.03 * rand())));
    const end = mid.clone().add(polar(a - 0.14, reach * 0.51, h * (0.13 - 0.03 * t)));
    limb(
      [joint, mid, end],
      [0.029 - 0.014 * t, 0.02 - 0.009 * t, 0.005].map((n) => n * h),
      7
    );
    for (const side of [-1, 1]) {
      const fork = mid.clone().lerp(end, rand() * 0.67);
      const elbow = fork.clone().add(polar(a + side * 0.96, reach * 0.23, h * 0.045));
      const tip = elbow
        .clone()
        .add(polar(a + side * 1.1, reach * (0.2 + rand() * 0.14), h * (0.055 + rand() * 0.038)));
      limb(
        [fork, elbow, tip],
        [0.012, 0.008, 0.002].map((n) => n * h),
        5
      );
      tuft(tip, a + side, 1 - 0.16 * t);
    }
    tuft(end, a, 1.03 - 0.16 * t);
  }
}

function randomSequence(seed) {
  return () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 4294967296;
  };
}

function chooseSpecies(tree, random) {
  if (tree.fir && random() < 0.66) return 1;
  const roll = random();
  if (roll < 0.3) return 0;
  if (roll < 0.56) return 2;
  return 3;
}

// Keep solid wood in spatial batches for both frustum culling and collision baking.
function placeBark(batch, source, tree, sx, sy, yaw) {
  const start = batch.positions.length / 3;
  const cos = Math.cos(yaw),
    sin = Math.sin(yaw);
  for (let i = 0; i < source.positions.length; i += 3) {
    const x = source.positions[i] * sx,
      z = source.positions[i + 2] * sx;
    batch.positions.push(
      tree.x + x * cos + z * sin,
      tree.y + source.positions[i + 1] * sy,
      tree.z - x * sin + z * cos
    );
  }
  batch.colors.push(...source.colors);
  for (const index of source.indices) batch.indices.push(start + index);
}

export async function addTreeMeshes(doc, trees) {
  const unlit = doc.createExtension(KHRMaterialsUnlit);
  const instancing = doc.createExtension(EXTMeshGPUInstancing).setRequired(true);
  const barkMat = doc.createMaterial('Painted bark').setExtension('KHR_materials_unlit', unlit.createUnlit());
  const image = await readFile(new URL('./sprigs.png', import.meta.url));
  const texture = doc.createTexture('Painted village sprigs').setImage(image).setMimeType('image/png');
  const leafMat = doc
    .createMaterial('Painted leaf sprays')
    .setBaseColorTexture(texture)
    .setAlphaMode('MASK')
    .setAlphaCutoff(0.4)
    .setDoubleSided(true)
    .setExtension('KHR_materials_unlit', unlit.createUnlit());
  const buffer = doc.getRoot().listBuffers()[0],
    scene = doc.getRoot().getDefaultScene();

  function accessor(type, array, normalized = false) {
    return doc.createAccessor().setBuffer(buffer).setType(type).setArray(array).setNormalized(normalized);
  }

  function meshNode(label, g, material) {
    const IndexArray = g.positions.length / 3 < 65536 ? Uint16Array : Uint32Array;
    const primitive = doc
      .createPrimitive()
      .setMaterial(material)
      .setAttribute('POSITION', accessor('VEC3', new Float32Array(g.positions)))
      .setAttribute('COLOR_0', accessor('VEC3', new Uint8Array(g.colors), true))
      .setIndices(accessor('SCALAR', new IndexArray(g.indices)));
    if (g.uv.length) {
      primitive.setAttribute(
        'TEXCOORD_0',
        accessor(
          'VEC2',
          Uint16Array.from(g.uv, (n) => Math.round(n * 65535)),
          true
        )
      );
    }
    const node = doc.createNode(label).setMesh(doc.createMesh(label).addPrimitive(primitive));
    scene.addChild(node);
    return node;
  }

  const random = randomSequence(117943);
  const variants = new Map(),
    wood = new Map();
  for (const tree of trees) {
    const kind = chooseSpecies(tree, random);
    const variant = Math.floor(random() * 4);
    const key = species[kind] + ' ' + variant;
    if (!variants.has(key)) {
      const bark = geometry(),
        foliage = geometry();
      grow(
        { x: 0, y: 0, z: 0, height: 6, radius: 2 },
        kind,
        bark,
        foliage,
        randomSequence(197309 + 8791 * kind + 98171 * variant)
      );
      variants.set(key, { bark, foliage, translations: [], rotations: [], scales: [] });
    }
    const shape = variants.get(key);
    const spread = [1.3, 1.02, 1.16, 1.24][kind];
    const sx = (tree.radius / 2) * spread,
      sy = tree.height / 6,
      yaw = (random() - 0.5) * 0.9;
    shape.translations.push(tree.x, tree.y, tree.z);
    shape.scales.push(sx, sy, sx);
    shape.rotations.push(0, Math.sin(yaw / 2), 0, Math.cos(yaw / 2));
    const cell = Math.floor(tree.x / 48) + ',' + Math.floor(tree.z / 48);
    if (!wood.has(cell)) wood.set(cell, geometry());
    placeBark(wood.get(cell), shape.bark, tree, sx, sy, yaw);
  }

  for (const [key, g] of wood) meshNode('Village tree bark ' + key, g, barkMat);
  for (const [key, shape] of variants) {
    const instances = instancing
      .createInstancedMesh()
      .setAttribute('TRANSLATION', accessor('VEC3', new Float32Array(shape.translations)))
      .setAttribute('ROTATION', accessor('VEC4', new Float32Array(shape.rotations)))
      .setAttribute('SCALE', accessor('VEC3', new Float32Array(shape.scales)));
    meshNode('Village tree foliage ' + key, shape.foliage, leafMat).setExtension(
      'EXT_mesh_gpu_instancing',
      instances
    );
  }
}
