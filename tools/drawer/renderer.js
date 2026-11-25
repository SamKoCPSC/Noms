import * as THREE from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {MeshoptDecoder} from 'meshoptimizer/meshopt_decoder.module.js';
import {disposeModel} from '../about/three/scene-resources.js';

// Share one WebGL context across the drawer. Individual rows keep their last
// frame while idle, so scrolling and navigation do not require a render loop.
export async function mountDrawerModels(drawer, signal) {
  signal.throwIfAborted();
  const renderer = new THREE.WebGLRenderer({alpha: true, antialias: true});
  renderer.setClearColor(0, 0);
  const loader = new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
  const live = new Set();
  signal.addEventListener('abort', () => {
    for (const icon of live) {
      cancelAnimationFrame(icon.frame);
      disposeModel(icon.model);
    }
    renderer.dispose();
    renderer.forceContextLoss();
  }, {once: true});
  renderer.domElement.addEventListener('webglcontextlost', () => {
    for (const icon of live) delete icon.slot.dataset.ready;
  }, {signal});

  async function load(slot) {
    const response = await fetch(slot.dataset.model, {signal});
    if (!response.ok) throw new Error(`Drawer model HTTP ${response.status}`);
    const bytes = await response.arrayBuffer();
    signal.throwIfAborted();
    const {scene: model} = await loader.parseAsync(bytes, '');
    if (signal.aborted) { disposeModel(model); return; }
    const icon = {slot, model, frame: 0};
    live.add(icon);
    const canvas = slot.querySelector('canvas');
    const context = canvas.getContext('2d');
    if (!context) return;
    const scene = new THREE.Scene();
    scene.add(model);
    const bounds = new THREE.Box3().setFromObject(model, true);
    const center = bounds.getCenter(new THREE.Vector3());
    const aspect = canvas.width / canvas.height;
    const start = slot.dataset.icon === 'introduction' ? -.23 : .26;
    const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, .1, 30);
    function position(angle) {
      camera.position.set(Math.sin(angle), .23, Math.cos(angle)).multiplyScalar(8).add(center);
      camera.lookAt(center);
      camera.updateMatrixWorld();
    }
    position(start);
    const projected = new THREE.Box3().makeEmpty();
    model.traverse(part => {
      if (!part.isMesh) return;
      const transform = new THREE.Matrix4().multiplyMatrices(camera.matrixWorldInverse, part.matrixWorld);
      const box = part.geometry.boundingBox;
      if (box) projected.union(box.clone().applyMatrix4(transform));
    });
    const size = projected.getSize(new THREE.Vector3());
    const half = Math.max(size.x / aspect, size.y) * .53;
    camera.left = -half * aspect; camera.right = half * aspect;
    camera.top = half; camera.bottom = -half; camera.updateProjectionMatrix();

    function draw(angle) {
      position(angle);
      renderer.setSize(canvas.width, canvas.height, false);
      renderer.render(scene, camera);
      context.clearRect(0, 0, canvas.width, canvas.height);
      context.drawImage(renderer.domElement, 0, 0);
    }
    draw(start);
    slot.dataset.ready = 'true';
    const row = slot.closest('a');
    if (!row) return;
    const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
    let angle = start;
    function turn() {
      const target = start + (!motion.matches && row.matches(':hover, :focus-visible') ? .28 : 0);
      cancelAnimationFrame(icon.frame);
      const from = angle;
      const began = performance.now();
      function step(now) {
        const t = motion.matches ? 1 : Math.min(1, (now - began) / 160);
        angle = from + (target - from) * (1 - (1 - t) ** 2);
        draw(angle);
        if (t < 1) icon.frame = requestAnimationFrame(step);
      }
      if (angle !== target) icon.frame = requestAnimationFrame(step);
    }
    for (const event of ['pointerenter', 'pointerleave', 'focus', 'blur']) {
      row.addEventListener(event, turn, {signal});
    }
    motion.addEventListener('change', turn, {signal});
  }

  await Promise.all(Array.from(drawer.querySelectorAll('[data-model]'), async slot => {
    try { await load(slot); }
    catch (error) { if (!signal.aborted) console.warn('Using drawer model poster', error); }
  }));
}
