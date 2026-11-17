// Three.js owns rendering; the Dioxus component owns mounting and the pause control.
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { MeshoptDecoder } from 'meshoptimizer/meshopt_decoder.module.js';
import { reflectionEnvironment, disposeModel } from '../../about/three/scene-resources.js';

export async function createKitchen(canvas, modelUrl, lightingUrl, signal) {
  let renderer, root, environment, mixer;
  try {
    renderer = new THREE.WebGLRenderer({canvas, alpha:true, antialias:true, powerPreference:'low-power'});
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.AgXToneMapping;
    renderer.setClearColor(0, 0);
    // Match the About tour: baked static shadows, no real-time shadow maps.
    renderer.shadowMap.enabled = false;
    const responses = await Promise.all([modelUrl, lightingUrl].map(url => fetch(url, {signal})));
    if (responses.some(r=>!r.ok)) throw new Error('Kitchen assets unavailable');
    const [bytes, lighting] = await Promise.all([responses[0].arrayBuffer(), responses[1].json()]);
    signal.throwIfAborted();
    const gltf = await new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).parseAsync(bytes, new URL('.', new URL(modelUrl, document.baseURI)).href);
    root = gltf.scene;
    signal.throwIfAborted();
    if (!canvas.isConnected) throw new Error('Kitchen unmounted');
    if (gltf.animations.length !== 1) throw new Error('Expected one complete kitchen sequence');
    const scene = new THREE.Scene(); scene.add(root);
    root.traverse(o=>{
      if (!o.isMesh) return;
      for (const m of Array.isArray(o.material) ? o.material : [o.material]) {
        if (m.isMeshBasicMaterial) m.toneMapped=false;
      }
    });
    environment = reflectionEnvironment(renderer, root, lighting);
    scene.environment = environment.texture;
    renderer.toneMappingExposure = 2 ** lighting.exposure;
    mixer = new THREE.AnimationMixer(root);
    const clip = gltf.animations[0], duration=clip.duration;
    const action = mixer.clipAction(clip); action.setLoop(THREE.LoopOnce, 1); action.clampWhenFinished=true; action.play();
    const camera = new THREE.PerspectiveCamera(35,1,.1,100);
    const target = new THREE.Vector3(0,2.2,0);
    // Turn toward the front so Penguini faces the visitor, while showing the open side.
    const azimuth = THREE.MathUtils.degToRad(-10), polar = THREE.MathUtils.degToRad(62);
    const direction = new THREE.Vector3(Math.sin(azimuth)*Math.sin(polar),Math.cos(polar),Math.cos(azimuth)*Math.sin(polar));
    const bounds = new THREE.Box3().setFromObject(root);
    const corners=[];
    for(const x of [bounds.min.x,bounds.max.x]) for(const y of [bounds.min.y,bounds.max.y]) for(const z of [bounds.min.z,bounds.max.z]) corners.push(new THREE.Vector3(x,y,z));
    let disposed=false, paused=true, inView=true, raf=0, previous=0, elapsed=0, width=0, height=0, dirty=true;
    const host=canvas.closest('.landing-kitchen');
    const resize=()=>{dirty=true;wake();};
    const observer=new ResizeObserver(resize); observer.observe(canvas);
    const intersection=new IntersectionObserver(entries=>{inView=entries[0].isIntersecting;previous=0;wake();}); intersection.observe(canvas);
    function fit() {
      const w=Math.max(1,canvas.clientWidth), h=Math.max(1,canvas.clientHeight);
      if(width===w && height===h) return;
      width=w;height=h;
      renderer.setPixelRatio(Math.min(devicePixelRatio||1,1.5)); renderer.setSize(w,h,false);
      camera.aspect=w/h;
      camera.position.copy(target).add(direction); camera.lookAt(target); camera.updateMatrixWorld();
      const inverse = camera.quaternion.clone().invert();
      const tan=Math.tan(THREE.MathUtils.degToRad(camera.fov/2));
      let distance=0;
      for(const p of corners) {
        const local=p.clone().sub(target).applyQuaternion(inverse);
        distance=Math.max(distance,local.z+Math.max(Math.abs(local.x)/(tan*camera.aspect),Math.abs(local.y)/tan)*1.02);
      }
      camera.position.copy(target).addScaledVector(direction,distance); camera.updateProjectionMatrix();
    }
    function draw(stamp) {
      raf=0;
      if(disposed || !canvas.isConnected || document.hidden || !inView) {previous=0;return;}
      const dt=previous?Math.max(0,(stamp-previous)/1000):0; previous=stamp;
      if(!paused) elapsed+=dt;
      // Fade across the ingredient reset while keeping the kitchen layout fixed.
      const cycle=duration+.4, t=elapsed%cycle;
      const opacity=t<.2?t/.2:t>duration?Math.max(0,(cycle-t)/.4):1;
      const pose=Math.min(duration,Math.max(0,t-.2));
      if(!paused || dirty) {
        fit(); action.paused=false; mixer.setTime(pose); renderer.render(scene,camera);
        canvas.style.opacity=String(paused || elapsed<.2 ? 1 : opacity);
        canvas.dataset.time=pose.toFixed(3); canvas.dataset.cycles=String(Math.floor(elapsed/cycle));
        canvas.dataset.playing=String(!paused); dirty=false;
      }
      if(!paused) raf=requestAnimationFrame(draw);
    }
    function wake(){if(!disposed && !raf && !document.hidden && inView)raf=requestAnimationFrame(draw);}
    const visibility=()=>{previous=0;wake();}; document.addEventListener('visibilitychange',visibility);
    const lost=e=>{e.preventDefault();host.dataset.state='fallback';api.dispose();};canvas.addEventListener('webglcontextlost',lost);
    const api={
      setPaused(value){paused=value;previous=0;dirty=true;wake();},
      dispose(){
        if(disposed)return;disposed=true;cancelAnimationFrame(raf);
        observer.disconnect();intersection.disconnect();document.removeEventListener('visibilitychange',visibility);canvas.removeEventListener('webglcontextlost',lost);
        mixer.stopAllAction();mixer.uncacheRoot(root);disposeModel(root);environment.dispose();renderer.dispose();renderer.forceContextLoss();
        signal.removeEventListener('abort', api.dispose);
        canvas.dataset.disposed='true';
      }
    };
    signal.addEventListener('abort',api.dispose,{once:true});
    canvas.dataset.renderer='three';canvas.dataset.duration=String(duration);canvas.dataset.bakedShadows='true';
    fit();mixer.setTime(0);renderer.render(scene,camera);
    host.dataset.state='ready';
    return api;
  } catch(error) {
    mixer?.stopAllAction(); if(root)disposeModel(root);environment?.dispose();renderer?.dispose();
    throw error;
  }
}
