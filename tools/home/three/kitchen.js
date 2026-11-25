// Three.js owns rendering; the Dioxus component owns mounting and the pause control.
import * as THREE from 'three';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { MeshoptDecoder } from 'meshoptimizer/meshopt_decoder.module.js';
import { reflectionEnvironment, disposeModel } from '../../about/three/scene-resources.js';

// Fit the entire composition at narrower or wider aspect ratios.
function studioCamera(position, target, width, height) {
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, .1, 100);
  camera.position.set(...position);
  camera.lookAt(...target);
  return {camera, fit(aspect) {
    const halfHeight = Math.max(width / aspect, height) / 2;
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

function lightStudio(renderer, scene, root, sceneName) {
  // Studio reflections give polished props visible highlights.
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
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  root.traverse(mesh => {
    // The market flippers travel beyond their rest bounds during the handoff and hops.
    if (sceneName === 'pandacotta-market-stand' && mesh.isSkinnedMesh) mesh.frustumCulled = false;
    if (mesh.isMesh) mesh.castShadow = mesh.receiveShadow = true;
  });
  let key;
  if (sceneName === 'pandacotta-market-stand') key = lightMarket(renderer, scene);
  else if (sceneName === 'signature-recipe') key = lightPlating(renderer, scene);
  else key = lightDeskOrBanquet(renderer, scene, sceneName === 'pandacotta-banquet');
  return {dispose() {key.shadow.dispose(); environment.dispose();}};
}

function lightDeskOrBanquet(renderer, scene, banquet) {
  scene.environmentIntensity = .54;
  renderer.toneMapping = THREE.NeutralToneMapping;
  renderer.toneMappingExposure = .78;
  scene.add(new THREE.HemisphereLight(0xfffbf5, 0xd4cbbf, .55));
  const key = new THREE.DirectionalLight(0xfff7ee, 2.3);
  key.position.set(-3.8, 7, 5.5);
  key.target.position.set(0, 1.6, .8);
  scene.add(key, key.target);
  key.castShadow = true;
  Object.assign(key.shadow.camera, {left: banquet ? -6 : -4.2, right: banquet ? 6 : 4.2, top: 3.7, bottom: -3.7, near: .5, far: 18});
  key.shadow.mapSize.set(512, 512);
  key.shadow.normalBias = .015;
  key.shadow.intensity = .78;
  const fill = new THREE.DirectionalLight(0xe0edff, .8);
  fill.position.set(4, 4.5, 4);
  const rim = new THREE.DirectionalLight(0xffe3c6, 1.8);
  rim.position.set(1.4, 6, -3);
  const bounce = new THREE.DirectionalLight(0xfff7ed, .6);
  bounce.position.set(-.5, .6, 6);
  scene.add(fill, rim, bounce);
  return key;
}

function aimedLight(scene, targetY, color, intensity, x, y, z) {
  const source = new THREE.DirectionalLight(color, intensity);
  source.position.set(x, y, z);
  source.target.position.set(0, targetY, .35);
  scene.add(source, source.target);
  return source;
}

function lightMarket(renderer, scene) {
  renderer.toneMapping = THREE.NeutralToneMapping;
  renderer.toneMappingExposure = .81;
  scene.environmentIntensity = .5;
  scene.add(new THREE.HemisphereLight(0xfff6e9, 0xc5ab8d, .56));
  const key = aimedLight(scene, 1.6, 0xffead6, 2.25, -3.8, 7, 5.5);
  key.castShadow = true;
  Object.assign(key.shadow.camera, {left: -3.7, right: 3.7, top: 3.5, bottom: -3.5, near: .5, far: 18});
  key.shadow.camera.updateProjectionMatrix();
  key.shadow.mapSize.set(512, 512);
  key.shadow.normalBias = .012;
  key.shadow.bias = -.00008;
  aimedLight(scene, 1.6, 0xe2edff, .78, 3.2, 4.5, 4.5);
  aimedLight(scene, 1.6, 0xffe3c6, 1.3, 1.4, 6, -3);
  aimedLight(scene, 1.6, 0xfff2dd, .37, -.5, .6, 6);
  return key;
}

function lightPlating(renderer, scene) {
  renderer.toneMapping = THREE.AgXToneMapping;
  renderer.toneMappingExposure = .96;
  scene.environmentIntensity = .38;
  scene.add(new THREE.HemisphereLight(0xfff4e2, 0xaba197, .52));
  const key = aimedLight(scene, 1.8, 0xffebd6, 2.8, -3.8, 7, 5.5);
  key.castShadow = true;
  Object.assign(key.shadow.camera, {left: -3.5, right: 3.5, top: 3, bottom: -3, near: .5, far: 16});
  key.shadow.camera.updateProjectionMatrix();
  key.shadow.mapSize.set(512, 512);
  key.shadow.normalBias = .012;
  aimedLight(scene, 1.8, 0xddeaff, .87, 3, 4, 4);
  aimedLight(scene, 1.8, 0xffdfba, 1.7, 1.4, 6.5, -2.8);
  return key;
}

async function fetchAsset(url, signal) {
  const response = await fetch(url, {signal});
  if (!response.ok) throw new Error('Landing scene asset unavailable');
  return response;
}

export async function createScene(canvas, sceneName, modelUrl, lightingUrl, signal) {
  const isKitchen = sceneName === 'kitchen';
  const isBanquet = sceneName === 'pandacotta-banquet';
  const isMarket = sceneName === 'pandacotta-market-stand';
  const fadeOnLoop = isKitchen || isMarket;
  let renderer, root, environment, mixer, api;
  try {
    renderer = new THREE.WebGLRenderer({canvas, alpha:true, antialias:true, powerPreference:'low-power'});
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.AgXToneMapping;
    renderer.setClearColor(0, 0);
    const [bytes, lighting] = await Promise.all([
      fetchAsset(modelUrl, signal).then(r => r.arrayBuffer()),
      isKitchen ? fetchAsset(lightingUrl, signal).then(r => r.json()) : null,
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
    if (isKitchen) {
      environment = reflectionEnvironment(renderer, root, lighting);
      scene.environment = environment.texture;
      renderer.toneMappingExposure = 2 ** lighting.exposure;
    } else {
      environment = lightStudio(renderer, scene, root, sceneName);
    }
    mixer = new THREE.AnimationMixer(root);
    const clip = gltf.animations[0], duration=clip.duration;
    const action = mixer.clipAction(clip);
    if (fadeOnLoop) {
      action.setLoop(THREE.LoopOnce, 1);
      action.clampWhenFinished = true;
    } else {
      action.setLoop(THREE.LoopRepeat, Infinity);
    }
    action.play();
    // Show all three courses in the static pose. Begin the first reveal on play.
    let showingPosterPose = isBanquet;
    if (showingPosterPose) mixer.setTime(22.6);
    const cycle = fadeOnLoop ? duration + .4 : duration;
    let view;
    if (isKitchen) view = kitchenCamera(root);
    else if (sceneName === 'signature-recipe') view = studioCamera([.11, 5.88, 12.7], [0, 2.08, .35], 6.16235, 6.16235 * 876 / 1059);
    else if (sceneName === 'terrimisu-cookie-study') view = studioCamera([1.13, 5.6, 10.9], [0, 2.19, .47], 5.79, 5.06);
    else if (isMarket) view = studioCamera([.41, 5.13, 13.25], [.16, 2.05, .56], 7.36, 7.36 * 800 / 1152);
    else if (isBanquet) view = studioCamera([.13, 5.02, 14], [0, 2.57, -.3], 10.675, 6.1);
    else if (sceneName === 'owlfredo-kitchen-island') view = studioCamera([3.75, 5.66, 11.4], [0, 2.51, .72], 7.0, 6.45);
    else view = studioCamera([2.1, 8.35, 15], [0, 1.98, .82], 7.48, 7.48 * 5 / 6);
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
        if (fadeOnLoop) {
          // Fade across the reset when props or characters return to their starting positions.
          const t = elapsed % cycle;
          action.paused = false;
          mixer.setTime(THREE.MathUtils.clamp(t - .2, 0, duration));
          if (!paused && elapsed >= .2) opacity = THREE.MathUtils.clamp(Math.min(t / .2, (cycle - t) / .4), 0, 1);
        } else {
          mixer.update(advance);
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
      setPaused(value){
        if (!value && showingPosterPose) { mixer.setTime(0); showingPosterPose=false; }
        paused=value;previous=0;dirty=true;wake();
      },
      dispose(){
        if(disposed)return;disposed=true;cancelAnimationFrame(raf);
        observer.disconnect();intersection.disconnect();document.removeEventListener('visibilitychange',visibility);canvas.removeEventListener('webglcontextlost',lost);
        mixer.stopAllAction();mixer.uncacheRoot(root);disposeModel(root);environment?.dispose();renderer.dispose();renderer.forceContextLoss();
        signal.removeEventListener('abort', api.dispose);
        canvas.dataset.disposed='true';
      }
    };
    signal.addEventListener('abort',api.dispose,{once:true});
    canvas.dataset.renderer='three';canvas.dataset.duration=String(duration);canvas.dataset.bakedShadows=String(isKitchen);
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

// The drawer reuses this bundle's Three.js engine and GLB decoder.
export {mountDrawerModels} from '../../drawer/renderer.js';
