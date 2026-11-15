// Rendering only. Rust owns scrolling, projection, framing, visibility and lifecycle.
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';

export async function createKitchen(canvas, modelUrl, lightingUrl) {
  const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true, powerPreference: 'high-performance' });
  renderer.setPixelRatio(1); // Rust supplies the bounded drawing-buffer dimensions.
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.AgXToneMapping;
  renderer.shadowMap.enabled = false;
  renderer.setClearColor(0, 0);
  let root, environment;
  try {
    const responses = await Promise.all([fetch(modelUrl), fetch(lightingUrl)]);
    if (responses.some(r => !r.ok)) throw new Error('Kitchen assets unavailable');
    const [bytes, lighting] = await Promise.all([responses[0].arrayBuffer(), responses[1].json()]);
    const gltf = await new GLTFLoader().parseAsync(bytes, new URL('.', new URL(modelUrl, document.baseURI)).href);
    root = gltf.scene;
    renderer.toneMappingExposure = 2 ** lighting.exposure;
    const scene = new THREE.Scene();
    scene.add(root);
    const live = [];
    root.traverse(object => {
      if (!object.isMesh) return;
      for (const material of Array.isArray(object.material) ? object.material : [object.material]) {
        // Blender already applied its exact AgX look/exposure to diffuse light maps.
        // Decode sRGB for sampling, but do not light or tone-map those pixels twice.
        if (material.isMeshBasicMaterial) material.toneMapped = false;
        else live.push(object);
      }
    });
    // One static reflection environment from the baked room and original softboxes.
    // No shadow maps, animation mixers, per-frame captures or independent RAF loop.
    const studio = new THREE.Scene();
    studio.background = new THREE.Color(...lighting.world_color).multiplyScalar(lighting.world_strength);
    const captureRoot = root.clone(true);
    captureRoot.traverse(o => { if (o.isMesh && !o.material.isMeshBasicMaterial) o.visible = false; });
    captureRoot.position.y = -1.5;
    studio.add(captureRoot);
    const basis = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), -Math.PI / 2);
    const panels = [];
    for (const light of lighting.lights) {
      const color = new THREE.Color(...light.color).multiplyScalar(light.energy / (Math.PI * light.size ** 2));
      const panel = new THREE.Mesh(new THREE.PlaneGeometry(light.size, light.size), new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide }));
      panel.position.fromArray(light.position); panel.position.y -= 1.5;
      panel.quaternion.copy(basis).multiply(new THREE.Quaternion().setFromEuler(new THREE.Euler(...light.rotation)));
      studio.add(panel); panels.push(panel);
    }
    const pmrem = new THREE.PMREMGenerator(renderer);
    environment = pmrem.fromScene(studio, 0.02, 0.1, 40);
    pmrem.dispose();
    panels.forEach(p => { p.geometry.dispose(); p.material.dispose(); });
    scene.environment = environment.texture;
    const camera = new THREE.PerspectiveCamera();
    camera.matrixAutoUpdate = false;
    root.matrixAutoUpdate = false;
    const rotation = new THREE.Quaternion(), scale = new THREE.Vector3(), position = new THREE.Vector3();
    let disposed = false, lost = false;
    const onLost = event => { event.preventDefault(); lost = true; canvas.style.display = 'none'; };
    canvas.addEventListener('webglcontextlost', onLost);
    canvas.dataset.renderer = 'three';
    canvas.dataset.bakedShadows = 'true';
    canvas.dataset.animations = String(gltf.animations.length);
    canvas.dataset.liveMaterials = String(new Set(live).size);
    return {
      draw(model, vp, view, width, height, visibility) {
        if (disposed || lost) return false;
        if (visibility <= 0) { canvas.style.display = 'none'; return true; }
        if (canvas.width !== width || canvas.height !== height) renderer.setSize(width, height, false);
        root.matrix.fromArray(model); root.matrixWorldNeedsUpdate = true;
        root.matrix.decompose(position, rotation, scale);
        scene.environmentRotation.setFromQuaternion(rotation);
        // Preserve the actual view basis as well as the clip transform. PBR
        // transmission needs a pure projection matrix, not a combined VP.
        camera.matrixWorldInverse.fromArray(view);
        camera.matrixWorld.copy(camera.matrixWorldInverse).invert();
        camera.matrix.copy(camera.matrixWorld); camera.matrixWorldNeedsUpdate = true;
        camera.projectionMatrix.fromArray(vp).multiply(camera.matrixWorld);
        camera.projectionMatrixInverse.copy(camera.projectionMatrix).invert();
        camera.isOrthographicCamera = Math.abs(camera.projectionMatrix.elements[15] - 1) < 1e-6;
        camera.isPerspectiveCamera = !camera.isOrthographicCamera;
        canvas.style.display = 'block'; canvas.style.opacity = String(visibility);
        renderer.render(scene, camera);
        canvas.dataset.calls = String(renderer.info.render.calls);
        canvas.dataset.triangles = String(renderer.info.render.triangles);
        return true;
      },
      hide() { canvas.style.display = 'none'; },
      dispose() {
        if (disposed) return;
        disposed = true; canvas.style.display = 'none';
        canvas.removeEventListener('webglcontextlost', onLost);
        disposeModel(root); environment.dispose(); renderer.dispose();
      }
    };
  } catch (error) {
    if (root) disposeModel(root);
    environment?.dispose(); renderer.dispose(); canvas.style.display = 'none';
    throw error;
  }
}
function disposeModel(root) {
  const geometries = new Set(), materials = new Set(), textures = new Set();
  root.traverse(o => {
    if (!o.isMesh) return;
    geometries.add(o.geometry);
    for (const material of Array.isArray(o.material) ? o.material : [o.material]) {
      materials.add(material);
      Object.values(material).forEach(v => { if (v?.isTexture) textures.add(v); });
    }
  });
  textures.forEach(t => { t.dispose(); t.source?.data?.close?.(); });
  materials.forEach(m => m.dispose()); geometries.forEach(g => g.dispose());
}
