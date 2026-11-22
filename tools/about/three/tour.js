// Rendering only. Rust owns camera movement, framing, visibility and lifecycle.
import * as THREE from 'three';
import { reflectionEnvironment, disposeModel } from './scene-resources.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { MeshoptDecoder } from 'meshoptimizer/meshopt_decoder.module.js';

export async function createTour(canvas, assets, initial) {
  const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true, powerPreference: 'high-performance' });
  renderer.setPixelRatio(1);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.AgXToneMapping;
  renderer.shadowMap.enabled = false;
  renderer.setClearColor(0, 0);
  renderer.autoClear = false;
  const camera = new THREE.PerspectiveCamera();
  camera.matrixAutoUpdate = false;
  const cache = new Map(), pending = new Map(), failed = new Set();
  let disposed = false, lost = false, active = initial, drawn = [];
  const onLost = event => { event.preventDefault(); lost = true; canvas.style.display = 'none'; };
  canvas.addEventListener('webglcontextlost', onLost);
  const notify = () => canvas.dispatchEvent(new Event('tourassetready'));
  const stats = () => {
    canvas.dataset.residentModels = String(cache.size);
    canvas.dataset.pendingModels = String(pending.size);
    canvas.dataset.failedModels = [...failed].join(',');
  };
  const evict = () => {
    // Current stop and its two neighbors only; never keep twelve high-res atlases.
    for (const [index, item] of cache) {
      if (Math.abs(index - active) > 1) { disposeScene(item); cache.delete(index); }
    }
    stats();
  };
  async function ensure(index) {
    if (disposed || lost || index < 0 || index >= assets.length || failed.has(index)) return;
    if (cache.has(index)) return cache.get(index);
    if (pending.has(index)) return pending.get(index).promise;
    // At most two downloads/decodes at a time, even after rapid chapter jumps.
    if (pending.size >= 2) return;
    const controller = new AbortController();
    const task = (async () => {
      let root, environment;
      const timeout = setTimeout(() => controller.abort(), 20000);
      try {
        const asset = assets[index];
        const responses = await Promise.all([fetch(asset.model, { signal: controller.signal }), fetch(asset.lighting, { signal: controller.signal })]);
        if (responses.some(r => !r.ok)) throw new Error(`Scene ${index} assets unavailable`);
        const [bytes, lighting] = await Promise.all([responses[0].arrayBuffer(), responses[1].json()]);
        clearTimeout(timeout);
        const gltf = await new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).parseAsync(bytes, new URL('.', new URL(asset.model, document.baseURI)).href);
        root = gltf.scene;
        if (disposed || lost || Math.abs(index - active) > 1) { disposeModel(root); return; }
        const scene = new THREE.Scene(); scene.add(root);
        const materials = new Map();
        root.traverse(object => {
          if (!object.isMesh) return;
          for (const material of Array.isArray(object.material) ? object.material : [object.material]) {
            if (material.isMeshBasicMaterial) material.toneMapped = false;
            if (material.transmission > 0) {
              // The transmitted room is already display-baked. Applying AgX
              // again washes out its colors behind the glass.
              material.toneMapped = false;
            }
            materials.set(material, { opacity: material.opacity, transparent: material.transparent, depthWrite: material.depthWrite });
          }
        });
        environment = reflectionEnvironment(renderer, root, lighting);
        scene.environment = environment.texture;
        root.matrixAutoUpdate = false;
        const item = { scene, root, environment, materials, exposure: 2 ** lighting.exposure };
        cache.set(index, item); evict();
        return item;
      } catch (error) {
        if (root) disposeModel(root);
        environment?.dispose();
        if (!disposed && (error.name !== 'AbortError' || Math.abs(index - active) <= 1)) { failed.add(index); console.warn('Baked tour scene uses legacy fallback', index, error); }
      } finally {
        clearTimeout(timeout);
        pending.delete(index); stats();
        if (!disposed && !lost) notify();
      }
    })();
    pending.set(index, { promise: task, controller }); stats();
    return task;
  }
  canvas.dataset.renderer = 'three';
  canvas.dataset.bakedShadows = 'true';
  canvas.dataset.animations = '0';
  const rotation = new THREE.Quaternion(), scale = new THREE.Vector3(), position = new THREE.Vector3();
  const api = {
    begin(progress, width, height) {
      if (disposed || lost) return false;
      active = Math.max(0, Math.min(assets.length - 1, Math.round(progress)));
      // Cancel stale network requests; late GLTF decodes are disposed above.
      for (const [index, task] of pending) if (Math.abs(index - active) > 1) task.controller.abort();
      evict();
      if (canvas.width !== width || canvas.height !== height) renderer.setSize(width, height, false);
      renderer.clear(); drawn = []; canvas.dataset.drawnModels = '';
      // A cleared alpha canvas is transparent over the fallback. Keeping it
      // mounted avoids display/layout churn on every Rust camera frame.
      if (canvas.style.display !== 'block') canvas.style.display = 'block';
      void ensure(active);
      // Prefer the direction of travel, but warm both neighbors over successive frames.
      void ensure(active + (progress < active ? -1 : 1));
      if (pending.size < 2) void ensure(active + (progress < active ? 1 : -1));
      return true;
    },
    has(index) { return !disposed && !lost && cache.has(index); },
    draw(index, model, vp, view, visibility) {
      if (disposed || lost) return false;
      if (visibility <= 0) return true;
      const item = cache.get(index);
      if (!item) { void ensure(index); return false; }
      item.root.matrix.fromArray(model); item.root.matrixWorldNeedsUpdate = true;
      item.root.matrix.decompose(position, rotation, scale);
      item.scene.environmentRotation.setFromQuaternion(rotation);
      renderer.toneMappingExposure = item.exposure;
      for (const [material, original] of item.materials) {
        const transparent = original.transparent || visibility < 1;
        if (material.transparent !== transparent) { material.transparent = transparent; material.needsUpdate = true; }
        material.opacity = original.opacity * visibility;
        material.depthWrite = original.depthWrite && visibility > .99;
      }
      camera.matrixWorldInverse.fromArray(view);
      camera.matrixWorld.copy(camera.matrixWorldInverse).invert();
      camera.matrix.copy(camera.matrixWorld); camera.matrixWorldNeedsUpdate = true;
      camera.projectionMatrix.fromArray(vp).multiply(camera.matrixWorld);
      camera.projectionMatrixInverse.copy(camera.projectionMatrix).invert();
      camera.isOrthographicCamera = Math.abs(camera.projectionMatrix.elements[15] - 1) < 1e-6;
      camera.isPerspectiveCamera = !camera.isOrthographicCamera;
      renderer.render(item.scene, camera);
      drawn.push(index); canvas.dataset.drawnModels = drawn.join(',');
      canvas.dataset.calls = String(renderer.info.render.calls);
      canvas.dataset.triangles = String(renderer.info.render.triangles);
      return true;
    },
    hide() { canvas.style.display = 'none'; },
    dispose() {
      if (disposed) return;
      disposed = true; canvas.style.display = 'none';
      canvas.removeEventListener('webglcontextlost', onLost);
      for (const task of pending.values()) task.controller.abort();
      for (const item of cache.values()) disposeScene(item);
      cache.clear(); renderer.dispose();
    }
  };
  // Failure of one model must not disable Three for every other model.
  await ensure(initial);
  return api;
}

function disposeScene(item) { disposeModel(item.root); item.environment.dispose(); }
