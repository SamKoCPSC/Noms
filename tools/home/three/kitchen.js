// Three.js owns rendering; the Dioxus component owns mounting and the pause control.
import * as THREE from 'three';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { MeshoptDecoder } from 'meshoptimizer/meshopt_decoder.module.js';
import { reflectionEnvironment, disposeModel } from '../../about/three/scene-resources.js';

// Match the orthographic framing of the desk poster exported from Blender.
function deskCamera() {
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, .1, 100);
  camera.position.set(2.1, 8.35, 15);
  camera.lookAt(0, 1.98, .82);
  return {camera, fit(aspect) {
    const halfHeight = Math.max(7.48 / (2 * aspect), 7.48 * 5 / 12);
    camera.top = halfHeight;
    camera.bottom = -halfHeight;
    camera.right = halfHeight * aspect;
    camera.left = -camera.right;
    camera.updateProjectionMatrix();
  }};
}

function kitchenCamera(root) {
  const camera = new THREE.PerspectiveCamera(35,1,.1,100);
  const target = new THREE.Vector3(0,2.2,0);
  // Turn toward the front so Penguini faces the visitor, while showing the open side.
  const azimuth = THREE.MathUtils.degToRad(-10), polar = THREE.MathUtils.degToRad(62);
  const direction = new THREE.Vector3(Math.sin(azimuth)*Math.sin(polar),Math.cos(polar),Math.cos(azimuth)*Math.sin(polar));
  const bounds = new THREE.Box3().setFromObject(root);
  const corners=[];
  for(const x of [bounds.min.x,bounds.max.x]) for(const y of [bounds.min.y,bounds.max.y]) for(const z of [bounds.min.z,bounds.max.z]) corners.push(new THREE.Vector3(x,y,z));
  return {camera, fit(aspect) {
    camera.aspect=aspect;
    camera.position.copy(target).add(direction); camera.lookAt(target); camera.updateMatrixWorld();
    const inverse = camera.quaternion.clone().invert();
    const tan=Math.tan(THREE.MathUtils.degToRad(camera.fov/2));
    let distance=0;
    for(const p of corners) {
      const local=p.clone().sub(target).applyQuaternion(inverse);
      distance=Math.max(distance,local.z+Math.max(Math.abs(local.x)/(tan*camera.aspect),Math.abs(local.y)/tan)*1.02);
    }
    camera.position.copy(target).addScaledVector(direction,distance); camera.updateProjectionMatrix();
  }};
}

function lightDesk(renderer, scene, root) {
  // Soft studio reflections keep the metallic laptops and glazed mugs readable.
  const studio = new RoomEnvironment();
  const generator = new THREE.PMREMGenerator(renderer);
  let environment;
  try {
    environment = generator.fromScene(studio, .04);
  } finally {
    generator.dispose();
    studio.dispose();
  }
  scene.environment = environment.texture;
  scene.environmentIntensity = .64;
  renderer.toneMapping = THREE.NeutralToneMapping;
  renderer.toneMappingExposure = .78;
  scene.add(new THREE.HemisphereLight(0xfffbf5, 0xd4cbbf, .55));
  const key = new THREE.DirectionalLight(0xfff7ee, 2.3);
  key.position.set(-3.8, 7, 5.5);
  key.target.position.set(0, 1.6, .8);
  scene.add(key, key.target);
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  key.castShadow = true;
  Object.assign(key.shadow.camera, {left: -4.2, right: 4.2, top: 3.7, bottom: -3.7, near: .5, far: 18});
  key.shadow.mapSize.set(512, 512);
  key.shadow.normalBias = .015;
  key.shadow.intensity = .5;
  root.traverse(mesh => {
    if (mesh.isMesh) mesh.castShadow = mesh.receiveShadow = true;
  });
  const fill = new THREE.DirectionalLight(0xe0edff, 1.2);
  fill.position.set(4, 4.5, 4);
  const rim = new THREE.DirectionalLight(0xffe3c6, 1.8);
  rim.position.set(1.4, 6, -3);
  const bounce = new THREE.DirectionalLight(0xfff7ed, .9);
  bounce.position.set(-.5, .6, 6);
  scene.add(fill, rim, bounce);
  return {dispose() {key.shadow.dispose(); environment.dispose();}};
}

async function fetchAsset(url, signal) {
  const response = await fetch(url, {signal});
  if (!response.ok) throw new Error('Landing scene asset unavailable');
  return response;
}

export async function createScene(canvas, sceneName, modelUrl, lightingUrl, signal) {
  const isDesk = sceneName === 'recipe-desk';
  let renderer, root, environment, mixer, api;
  try {
    renderer = new THREE.WebGLRenderer({canvas, alpha:true, antialias:true, powerPreference:'low-power'});
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.AgXToneMapping;
    renderer.setClearColor(0, 0);
    const [bytes, lighting] = await Promise.all([
      fetchAsset(modelUrl, signal).then(r => r.arrayBuffer()),
      isDesk ? null : fetchAsset(lightingUrl, signal).then(r => r.json()),
    ]);
    signal.throwIfAborted();
    const gltf = await new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).parseAsync(bytes, new URL('.', new URL(modelUrl, document.baseURI)).href);
    root = gltf.scene;
    signal.throwIfAborted();
    if (!canvas.isConnected) throw new Error('Landing scene unmounted');
    if (gltf.animations.length !== 1) throw new Error('Expected one landing animation');
    const scene = new THREE.Scene(); scene.add(root);
    root.traverse(o=>{
      if (!o.isMesh) return;
      for (const m of Array.isArray(o.material) ? o.material : [o.material]) {
        if (m.isMeshBasicMaterial) m.toneMapped=false;
      }
    });
    if (isDesk) {
      environment = lightDesk(renderer, scene, root);
    } else {
      environment = reflectionEnvironment(renderer, root, lighting);
      scene.environment = environment.texture;
      renderer.toneMappingExposure = 2 ** lighting.exposure;
    }
    mixer = new THREE.AnimationMixer(root);
    const clip = gltf.animations[0], duration=clip.duration;
    const action = mixer.clipAction(clip);
    if (isDesk) {
      action.setLoop(THREE.LoopRepeat, Infinity);
    } else {
      action.setLoop(THREE.LoopOnce, 1);
      action.clampWhenFinished = true;
    }
    action.play();
    const cycle = isDesk ? duration : duration + .4;
    const view = isDesk ? deskCamera() : kitchenCamera(root);
    const camera = view.camera;
    let disposed=false, paused=true, inView=true, raf=0, previous=0, elapsed=0, width=0, height=0, dirty=true;
    const host=canvas.closest('figure');
    const resize=()=>{dirty=true;wake();};
    const observer=new ResizeObserver(resize); observer.observe(canvas);
    const intersection=new IntersectionObserver(entries=>{inView=entries[0].isIntersecting;previous=0;wake();}); intersection.observe(canvas);
    function fit() {
      const w=Math.max(1,canvas.clientWidth), h=Math.max(1,canvas.clientHeight);
      if(width===w && height===h) return;
      width=w;height=h;
      renderer.setPixelRatio(Math.min(devicePixelRatio||1,1.5)); renderer.setSize(w,h,false);
      view.fit(w/h);
    }
    function draw(stamp) {
      raf=0;
      if(disposed || !canvas.isConnected || document.hidden || !inView) {previous=0;return;}
      const dt=previous?Math.max(0,(stamp-previous)/1000):0; previous=stamp;
      if(!paused || dirty) {
        const advance = paused ? 0 : dt;
        elapsed += advance;
        let opacity = 1;
        if (isDesk) {
          mixer.update(advance);
        } else {
          // Hold and fade the kitchen across the ingredient reset.
          const t = elapsed % cycle;
          action.paused = false;
          mixer.setTime(THREE.MathUtils.clamp(t - .2, 0, duration));
          if (!paused && elapsed >= .2) opacity = THREE.MathUtils.clamp(Math.min(t / .2, (cycle - t) / .4), 0, 1);
        }
        fit(); renderer.render(scene,camera);
        canvas.style.opacity=String(opacity);
        canvas.dataset.time=action.time.toFixed(3); canvas.dataset.cycles=String(Math.floor(elapsed/cycle));
        canvas.dataset.playing=String(!paused); dirty=false;
      }
      if(!paused) raf=requestAnimationFrame(draw);
    }
    function wake(){if(!disposed && !raf && !document.hidden && inView)raf=requestAnimationFrame(draw);}
    const visibility=()=>{previous=0;wake();}; document.addEventListener('visibilitychange',visibility);
    const lost=e=>{e.preventDefault();host.dataset.state='fallback';api.dispose();};canvas.addEventListener('webglcontextlost',lost);
    api={
      setPaused(value){paused=value;previous=0;dirty=true;wake();},
      dispose(){
        if(disposed)return;disposed=true;cancelAnimationFrame(raf);
        observer.disconnect();intersection.disconnect();document.removeEventListener('visibilitychange',visibility);canvas.removeEventListener('webglcontextlost',lost);
        mixer.stopAllAction();mixer.uncacheRoot(root);disposeModel(root);environment?.dispose();renderer.dispose();renderer.forceContextLoss();
        signal.removeEventListener('abort', api.dispose);
        canvas.dataset.disposed='true';
      }
    };
    signal.addEventListener('abort',api.dispose,{once:true});
    canvas.dataset.renderer='three';canvas.dataset.duration=String(duration);canvas.dataset.bakedShadows=String(!isDesk);
    fit();mixer.update(0);renderer.render(scene,camera);
    host.dataset.state='ready';
    return api;
  } catch(error) {
    if (api) {
      api.dispose();
    } else {
      mixer?.stopAllAction();
      if (root) disposeModel(root);
      environment?.dispose();
      renderer?.dispose();
      renderer?.forceContextLoss();
    }
    throw error;
  }
}
