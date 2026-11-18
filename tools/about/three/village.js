// Three.js renders a single baked world. Rust owns navigation, timing, scrolling,
// text, loading/fallback decisions and the lifetime of this adapter.
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { MeshoptDecoder } from 'meshoptimizer/meshopt_decoder.module.js';
import { configureClearGlass } from '../village/clear-glass.js';
import { createWalkingTurn, frameCamera } from './village-camera.js';

import {createFence, createPerimeter} from '../village/perimeter.js';
import { createFirstPersonCamera } from './village-walk.js';

export function createVillage(canvas,catalog) {
 const renderer=new THREE.WebGLRenderer({canvas,antialias:true,logarithmicDepthBuffer:true,powerPreference:'high-performance'});
 renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.AgXToneMapping;renderer.shadowMap.enabled=false;renderer.transmissionResolutionScale=1;
 renderer.setPixelRatio(Math.min(window.devicePixelRatio||1,1.5));
 const camera=new THREE.PerspectiveCamera(60,1,.08,650),scene=new THREE.Scene();scene.background=new THREE.Color('#c6e0ed');scene.fog=new THREE.Fog('#c6e0ed',100,300);
 const sky=new THREE.Mesh(new THREE.SphereGeometry(480,24,12),new THREE.ShaderMaterial({side:THREE.BackSide,depthWrite:false,toneMapped:false,uniforms:{top:{value:new THREE.Color('#70b6e1')},bottom:{value:new THREE.Color('#e4edcf')}},vertexShader:'varying vec3 v; void main(){v=position;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',fragmentShader:'varying vec3 v;uniform vec3 top;uniform vec3 bottom;void main(){float h=smoothstep(-.05,.65,normalize(v).y);gl_FragColor=vec4(mix(bottom,top,h),1.);\n#include <colorspace_fragment>\n}'}));scene.add(sky);
 let disposed=false,layout=null,loaded=0,environment=null;
 const controllers=new Set(),roots=[];
 const notify=()=>{canvas.dataset.loadedModels=String(loaded);canvas.dispatchEvent(new Event('villageassetready'));};
 async function fetchAsset(url,json=false){
  if(disposed)throw new DOMException('Disposed','AbortError');
  const controller=new AbortController();controllers.add(controller);const timer=setTimeout(()=>controller.abort(),60000);
  try{const response=await fetch(url,{signal:controller.signal});if(!response.ok)throw Error('Village asset unavailable');return await (json?response.json():response.arrayBuffer());}
  finally{clearTimeout(timer);controllers.delete(controller);}
 }
 const loader=new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
 function disposeRoot(root){const geometries=new Set(),materials=new Set(),textures=new Set();root.traverse(o=>{if(o.isInstancedMesh)o.dispose();if(o.geometry)geometries.add(o.geometry);for(const m of o.material?(Array.isArray(o.material)?o.material:[o.material]):[]){materials.add(m);for(const v of Object.values(m))if(v?.isTexture)textures.add(v);}});geometries.forEach(g=>g.dispose());textures.forEach(t=>{t.source?.data?.close?.();t.dispose();});materials.forEach(m=>m.dispose());}
 let firstPerson;
 const walkingTurn = createWalkingTurn();
 const api={
  ready:null,
  draw(progress,width,height,slot,openingDistance=0,look,walk){
   if(disposed||!layout||loaded!==catalog.models.length+1)return false;
   if(renderer.domElement.clientWidth!==width||renderer.domElement.clientHeight!==height||canvas.dataset.cssSize!==`${width},${height}`){renderer.setSize(width,height,false);canvas.dataset.cssSize=`${width},${height}`;}
   const {backwards, offset} = walkingTurn(layout, progress);
   frameCamera(camera,layout,progress,width,height,slot,backwards,openingDistance,look,offset);
   firstPerson(camera,walk);
   renderer.render(scene,camera);
   canvas.dataset.progress=String(progress);canvas.dataset.renderer='three-village';canvas.dataset.ready='true';canvas.dataset.calls=String(renderer.info.render.calls);
   canvas.dataset.cameraEye=JSON.stringify(camera.position.toArray());canvas.dataset.projection=JSON.stringify(camera.projectionMatrix.elements);
   const index=Math.round(progress),projected=[];
   for(const vertex of layout.stops[index].framing.extrema){const p=new THREE.Vector3(...vertex).project(camera);projected.push([(p.x+1)*width/2,(1-p.y)*height/2]);}
   canvas.dataset.modelBounds=JSON.stringify([Math.min(...projected.map(p=>p[0])),Math.min(...projected.map(p=>p[1])),Math.max(...projected.map(p=>p[0])),Math.max(...projected.map(p=>p[1]))]);
   return true;
  },
  dispose(){if(disposed)return;disposed=true;controllers.forEach(c=>c.abort());roots.forEach(disposeRoot);roots.length=0;disposeRoot(sky);environment?.dispose();renderer.dispose();canvas.dataset.ready='false';}
 };
 canvas.dataset.renderer='three-village';canvas.dataset.animations='0';canvas.dataset.bakedShadows='true';canvas.dataset.totalModels=String(catalog.models.length+1);canvas.dataset.ready='false';notify();
 api.ready=(async()=>{
  try{
   layout=await fetchAsset(catalog.layout,true);if(disposed)throw new DOMException('Disposed','AbortError');
   if(layout.stops.length!==catalog.models.length)throw Error('Village/story mismatch');
   const perimeter=createPerimeter(layout.walk.outline);
   firstPerson=createFirstPersonCamera(layout,perimeter);
   const fence=createFence(perimeter,layout.walk);
   scene.add(fence);roots.push(fence);
   const settings=layout.lighting;renderer.toneMappingExposure=2**settings.exposure;
   scene.add(new THREE.HemisphereLight(new THREE.Color(...settings.sky_color),'#869956',settings.sky_strength));
   const sun=new THREE.DirectionalLight(new THREE.Color(...settings.sun_color),settings.sun_energy);sun.position.fromArray(settings.sun_direction_y_up);scene.add(sun);
   const outdoor=new THREE.Scene();outdoor.background=new THREE.Color(...settings.sky_color).multiplyScalar(settings.sky_strength);
   const ground=new THREE.Mesh(new THREE.PlaneGeometry(400,400),new THREE.MeshBasicMaterial({color:'#56842e'}));ground.rotation.x=-Math.PI/2;ground.position.y=-15;outdoor.add(ground);
   const generator=new THREE.PMREMGenerator(renderer);environment=generator.fromScene(outdoor,.04);scene.environment=environment.texture;generator.dispose();disposeRoot(ground);
   const urls=[catalog.world,...catalog.models];let next=0;
   await Promise.all([0,1].map(async()=>{while(next<urls.length){const url=urls[next++],bytes=await fetchAsset(url);if(disposed)return;const gltf=await loader.parseAsync(bytes,new URL('.',new URL(url,document.baseURI)).href);if(disposed){disposeRoot(gltf.scene);return;}
    gltf.scene.traverse(o=>{if(o.isMesh)for(const m of Array.isArray(o.material)?o.material:[o.material]){if(m.isMeshBasicMaterial||m.transmission>0)m.toneMapped=false;configureClearGlass(m);}});
    roots.push(gltf.scene);scene.add(gltf.scene);loaded++;notify();
   }}));
   if(disposed)throw new DOMException('Disposed','AbortError');
   canvas.dataset.residentModels=String(catalog.models.length);return true;
  }catch(error){api.dispose();throw error;}
 })();
 // Attach a handler immediately; Rust also awaits the promise across wasm-bindgen.
 api.ready.catch(()=>{});
 return api;
}
