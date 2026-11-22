import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { MeshoptDecoder } from 'meshoptimizer';

const $=id=>document.getElementById(id),canvas=document.querySelector('canvas');
try {
const meta=globalThis.__STUDY__ || await fetch('study.json').then(r=>{if(!r.ok)throw Error('Study unavailable');return r.json();});
const renderer=new THREE.WebGLRenderer({canvas,antialias:true,logarithmicDepthBuffer:true});
renderer.setPixelRatio(Math.min(devicePixelRatio,1.5));renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.AgXToneMapping;
const scene=new THREE.Scene(),camera=new THREE.PerspectiveCamera(60,1,.08,650);
const controls=new OrbitControls(camera,canvas);controls.enableDamping=true;controls.minDistance=2;controls.maxDistance=95;controls.maxPolarAngle=Math.PI*.52;
let dirty=true,mode='after',motion=null;
controls.addEventListener('change',()=>dirty=true);controls.addEventListener('start',()=>stopWalk());
const sky=new THREE.Mesh(new THREE.SphereGeometry(480,24,12),new THREE.ShaderMaterial({side:THREE.BackSide,depthWrite:false,toneMapped:false,
uniforms:{top:{value:new THREE.Color('#70b6e1')},bottom:{value:new THREE.Color('#e4edcf')}},vertexShader:'varying vec3 v;void main(){v=position;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',fragmentShader:'varying vec3 v;uniform vec3 top;uniform vec3 bottom;void main(){float h=smoothstep(-.05,.65,normalize(v).y);gl_FragColor=vec4(mix(bottom,top,h),1.);\n#include <colorspace_fragment>\n}'}));scene.add(sky);
const oldLights=new THREE.Group(),newLights=new THREE.Group();scene.add(oldLights,newLights);
oldLights.add(new THREE.HemisphereLight('#e4f1ff','#869956',.8));const oldSun=new THREE.DirectionalLight('#fff3dc',1.1);oldSun.position.set(-60,100,50);oldLights.add(oldSun);
newLights.add(new THREE.HemisphereLight(new THREE.Color(...meta.lighting.sky_color),'#56842e',meta.lighting.sky_strength));
const sun=new THREE.DirectionalLight(new THREE.Color(...meta.lighting.sun_color),meta.lighting.sun_energy);sun.position.fromArray(meta.lighting.sun_direction_y_up);newLights.add(sun);
const pmrem=new THREE.PMREMGenerator(renderer),room=new RoomEnvironment(),oldEnv=pmrem.fromScene(room,.04);room.dispose();
// Live glass/metal use an outdoor reflection field. Opaque colors and shadows
// are display-baked and therefore never receive a second lighting/tone-map pass.
const envScene=new THREE.Scene();envScene.background=new THREE.Color(...meta.lighting.sky_color).multiplyScalar(meta.lighting.sky_strength);
const envGround=new THREE.Mesh(new THREE.PlaneGeometry(400,400),new THREE.MeshBasicMaterial({color:'#56842e'}));envGround.rotation.x=-Math.PI/2;envGround.position.y=-15;envScene.add(envGround);
const newEnv=pmrem.fromScene(envScene,.04);envGround.geometry.dispose();envGround.material.dispose();pmrem.dispose();
const loader=new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
async function load(id){
  const embedded=$('asset-'+id);let gltf;
  if(embedded){const bytes=Uint8Array.from(atob(embedded.textContent.trim()),c=>c.charCodeAt(0));embedded.remove();gltf=await loader.parseAsync(bytes.buffer,'');}
  else if(globalThis.__STUDY__)throw Error('Embedded model missing: '+id);
  else gltf=await loader.loadAsync(id+'.glb');
  return gltf.scene;
}
const before=new THREE.Group();scene.add(before);
$('status').textContent='Loading the current lighting…';
const landscape=await load('before');before.add(landscape);
landscape.traverse(o=>{if(o.isMesh)for(const m of Array.isArray(o.material)?o.material:[o.material]){m.toneMapped=false;m.roughness=1;m.metalness=0;m.envMapIntensity=.025;if('specularIntensity'in m)m.specularIntensity=.08;}});
const kitchen=await load('kitchen');kitchen.updateMatrixWorld(true);
const b=new THREE.Box3().setFromObject(kitchen),size=b.getSize(new THREE.Vector3()),center=b.getCenter(new THREE.Vector3());
const expected=meta.source_bounds;
for(let i=0;i<3;i++)if(Math.abs(b.min.getComponent(i)-expected.min[i])>.002||Math.abs(b.max.getComponent(i)-expected.max[i])>.002)throw Error('Kitchen anchor changed; rebuild the study');
const anchor=new THREE.Group(),transform=meta.kitchen_transform;anchor.position.fromArray(transform.position);anchor.rotation.y=transform.yaw;
kitchen.position.set(-center.x*transform.scale,-b.min.y*transform.scale,-center.z*transform.scale);kitchen.scale.setScalar(transform.scale);anchor.add(kitchen);before.add(anchor);
$('status').textContent='Loading the shared sunlight…';const after=await load('after');scene.add(after);
for(const group of [kitchen,after])group.traverse(o=>{if(o.isMesh)for(const m of Array.isArray(o.material)?o.material:[o.material]){if(m.isMeshBasicMaterial||m.transmission>0)m.toneMapped=false;}});
const poses={arrival:meta.camera,path:{eye:[-38,4.2,53],target:[-50,3,32]},wide:{eye:[-20,24,65],target:[-48,1.5,32]}};
function apply(p){camera.position.fromArray(p.eye);controls.target.fromArray(p.target);camera.lookAt(controls.target);dirty=true;}
function switchMode(next){mode=next;before.visible=next==='before';after.visible=!before.visible;oldLights.visible=before.visible;newLights.visible=after.visible;scene.environment=before.visible?oldEnv.texture:newEnv.texture;renderer.toneMappingExposure=before.visible?1:2**meta.lighting.exposure;for(const id of ['before','after'])$(id).setAttribute('aria-pressed',String(id===next));$('caption').textContent=before.visible?'Original kitchen lighting beside the current matte landscape.':'One sun and sky; baked shadows connect the kitchen to its surroundings.';dirty=true;}
function stopWalk(){motion=null;$('walk').textContent='Walk a little';}
function setView(name){stopWalk();$('view').value=name;apply(poses[name]);}
$('before').onclick=()=>switchMode('before');$('after').onclick=()=>switchMode('after');$('view').onchange=e=>setView(e.target.value);
$('walk').onclick=()=>{if(motion){stopWalk();return;}setView('arrival');if(matchMedia('(prefers-reduced-motion: reduce)').matches){apply(meta.walk[meta.walk.length-1]);return;}motion={start:performance.now()};$('walk').textContent='Pause';};
function resize(){renderer.setSize(innerWidth,innerHeight);camera.aspect=innerWidth/innerHeight;camera.fov=camera.aspect<.8?82:60;camera.updateProjectionMatrix();dirty=true;}
window.addEventListener('resize',resize);resize();setView('arrival');switchMode('after');
function frame(now){if(motion){const t=Math.min(1,(now-motion.start)/12000)*(meta.walk.length-1),i=Math.min(meta.walk.length-2,Math.floor(t)),f=t-i,a=meta.walk[i],b=meta.walk[i+1];apply({eye:a.eye.map((v,j)=>THREE.MathUtils.lerp(v,b.eye[j],f)),target:a.target.map((v,j)=>THREE.MathUtils.lerp(v,b.target[j],f))});if(t>=meta.walk.length-1)stopWalk();}controls.update();if(dirty){renderer.render(scene,camera);dirty=false;}requestAnimationFrame(frame);}requestAnimationFrame(frame);
window.lightingStudy={setMode:switchMode,setView,state:()=>({mode,eye:camera.position.toArray(),target:controls.target.toArray(),aspect:camera.aspect,shadows:renderer.shadowMap.enabled,triangles:renderer.info.render.triangles,walking:!!motion})};
canvas.dataset.ready='true';$('loading').hidden=true;
}catch(error){$('status').textContent='The lighting study could not load.';$('error').textContent=error.message;$('retry').hidden=false;$('retry').onclick=()=>location.reload();console.error(error);}
