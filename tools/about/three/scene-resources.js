// Shared display-baked lighting and GPU resource cleanup for Three.js scenes.
import * as THREE from 'three';

export function reflectionEnvironment(renderer, root, lighting) {
  const studio = new THREE.Scene();
  studio.background = new THREE.Color(...lighting.world_color).multiplyScalar(lighting.world_strength);
  const captureRoot = root.clone(true);
  captureRoot.traverse(o => { if (o.isMesh && !o.material.isMeshBasicMaterial) o.visible = false; });
  captureRoot.position.y = -1.5; studio.add(captureRoot);
  const basis = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), -Math.PI / 2);
  const panels = [];
  for (const light of lighting.lights) {
    const size = Math.max(.1, light.size);
    const color = new THREE.Color(...light.color).multiplyScalar(light.energy / (Math.PI * size ** 2));
    const panel = new THREE.Mesh(new THREE.PlaneGeometry(size, size), new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide }));
    panel.position.fromArray(light.position); panel.position.y -= 1.5;
    panel.quaternion.copy(basis).multiply(new THREE.Quaternion().setFromEuler(new THREE.Euler(...light.rotation)));
    studio.add(panel); panels.push(panel);
  }
  const pmrem = new THREE.PMREMGenerator(renderer);
  try { return pmrem.fromScene(studio, .02, .1, 100); }
  finally { pmrem.dispose(); panels.forEach(p => { p.geometry.dispose(); p.material.dispose(); }); }
}
export function disposeModel(root) {
  const geometries = new Set(), materials = new Set(), textures = new Set(), skeletons = new Set();
  root.traverse(o => {
    if (!o.isMesh) return;
    geometries.add(o.geometry);
    if (o.skeleton) skeletons.add(o.skeleton);
    for (const material of Array.isArray(o.material) ? o.material : [o.material]) {
      materials.add(material);
      Object.values(material).forEach(v => { if (v?.isTexture) textures.add(v); });
    }
  });
  skeletons.forEach(s => s.dispose());
  textures.forEach(t => { t.dispose(); t.source?.data?.close?.(); });
  materials.forEach(m => m.dispose()); geometries.forEach(g => g.dispose());
}
