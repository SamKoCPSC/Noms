// Recover planted trees from the editable, Y-up landscape before its daylight meshes were batched.
import { readFile } from 'node:fs/promises';

const base = 'assets/models/terraced-village';
export async function treePlantings() {
  const source = JSON.parse(await readFile(base + '/source/world-environment.json'));
  const { original_foliage_bounds: crowns } = JSON.parse(
    await readFile(base + '/daylight/source/vegetation-verification.json')
  );
  const trees = [];
  for (const mesh of source) {
    const batched = mesh.name === 'Faceted tree trunk';
    // The earlier six-sided fir trunks are individually authored as Landscape detail.
    if (
      !batched &&
      !(
        mesh.name === 'Landscape detail' &&
        mesh.vertices.length === 26 &&
        Math.abs(mesh.color[0] - 0.21586) < 0.00001
      )
    )
      continue;
    const verticesPerTrunk = batched ? 30 : 26;
    for (let i = 0; i < mesh.vertices.length; i += verticesPerTrunk) {
      const verts = mesh.vertices.slice(i, i + verticesPerTrunk);
      const y = Math.min(...verts.map((v) => v[1]));
      // Center of the bottom cap (seam/cap vertices repeat around the ring).
      const bottom = [...new Map(verts.filter((v) => v[1] === y).map((v) => [v.join(','), v])).values()];
      const mean = (axis) => bottom.reduce((n, v) => n + v[axis], 0) / bottom.length;
      trees.push({ x: mean(0), y, z: mean(2), height: 0, radius: 0, fir: false });
    }
  }
  for (const { source_index, min, max } of crowns) {
    const name = source[source_index].name;
    if (!['Rounded leafy canopy', 'Folded evergreen canopy', 'Landscape detail'].includes(name)) continue;
    const x = (min[0] + max[0]) / 2,
      z = (min[2] + max[2]) / 2;
    const closest = trees.reduce((best, t) =>
      Math.hypot(t.x - x, t.z - z) < Math.hypot(best.x - x, best.z - z) ? t : best
    );
    if (Math.hypot(closest.x - x, closest.z - z) > 2.1) throw Error('Unmatched village crown');
    closest.height = Math.max(closest.height, max[1] - closest.y);
    closest.radius = Math.max(
      closest.radius,
      max[0] - closest.x,
      closest.x - min[0],
      max[2] - closest.z,
      closest.z - min[2]
    );
    closest.fir = name !== 'Rounded leafy canopy';
  }
  if (trees.some((t) => t.height <= 0) || trees.length !== 116) throw Error('Incomplete village planting');
  return trees;
}
