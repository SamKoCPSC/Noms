import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { MeshoptDecoder } from 'meshoptimizer';
import { groundHeight, makeWalkingRoutes } from './walking-world.js';
import { addFrontages } from './frontages.js';
import { addLandscape } from './landscape.js';
import { addNeighborhood } from './neighborhood.js';

const $ = id => document.getElementById(id);
const canvas = document.querySelector('canvas');
let renderer;
try { renderer = new THREE.WebGLRenderer({canvas, antialias:true, logarithmicDepthBuffer:true}); }
catch { $('status').textContent='This preview needs a browser with WebGL.'; throw Error('WebGL unavailable'); }
renderer.setPixelRatio(Math.min(devicePixelRatio,1.5));
renderer.outputColorSpace=THREE.SRGBColorSpace; renderer.toneMapping=THREE.AgXToneMapping;
const scene=new THREE.Scene(); scene.background=new THREE.Color('#c6e0ed');
scene.fog=new THREE.Fog('#c6e0ed',100,300);
const sky=new THREE.Mesh(new THREE.SphereGeometry(480,24,12),new THREE.ShaderMaterial({side:THREE.BackSide,depthWrite:false,toneMapped:false,uniforms:{top:{value:new THREE.Color('#70b6e1')},bottom:{value:new THREE.Color('#e4edcf')}},vertexShader:'varying vec3 v; void main(){v=position;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',fragmentShader:'varying vec3 v;uniform vec3 top;uniform vec3 bottom;void main(){float h=smoothstep(-.05,.65,normalize(v).y);gl_FragColor=vec4(mix(bottom,top,h),1.);\n#include <tonemapping_fragment>\n#include <colorspace_fragment>\n}'}));scene.add(sky);
const camera=new THREE.PerspectiveCamera(60,1,.08,650);
const controls=new OrbitControls(camera,canvas);controls.enableDamping=true;controls.minDistance=.5;controls.maxDistance=260;controls.maxPolarAngle=Math.PI*.9;
let dirty=true;controls.addEventListener('change',()=>{dirty=true;});
scene.add(new THREE.HemisphereLight('#e4f1ff','#869956',.8));
const sun=new THREE.DirectionalLight('#fff3dc',1.1);sun.position.set(-60,100,50);scene.add(sun);
const pmrem=new THREE.PMREMGenerator(renderer), room=new RoomEnvironment();
scene.environment=pmrem.fromScene(room,.04).texture;room.dispose();pmrem.dispose();
const terrain=new THREE.Group();terrain.name='Village blockout terrain';scene.add(terrain);
const materials={};
function mat(color){return materials[color]??=new THREE.MeshStandardMaterial({color,roughness:.95,envMapIntensity:.2,toneMapped:false});}
function box(name,p,size,color){const m=new THREE.Mesh(new THREE.BoxGeometry(...size),mat(color));m.name=name;m.position.fromArray(p);terrain.add(m);return m;}
function line(points,color,radius=.5){const c=new THREE.CatmullRomCurve3(points.map(p=>new THREE.Vector3(...p)));const m=new THREE.Mesh(new THREE.TubeGeometry(c,Math.max(40,points.length*12),radius,6,false),mat(color));terrain.add(m);return c;}
function label(text,x,y,z){const c=document.createElement('canvas');c.width=512;c.height=80;const ctx=c.getContext('2d');ctx.fillStyle='#faf6e9';ctx.beginPath();ctx.roundRect(0,0,512,80,24);ctx.fill();ctx.fillStyle='#3f4935';ctx.font='600 29px system-ui';ctx.textAlign='center';ctx.fillText(text,256,51);const tex=new THREE.CanvasTexture(c);tex.colorSpace=THREE.SRGBColorSpace;const sprite=new THREE.Sprite(new THREE.SpriteMaterial({map:tex,depthTest:true}));sprite.position.set(x,y,z);sprite.scale.set(8,1.25,1);scene.add(sprite);return sprite;}
function tree(x,y,z,size){const trunk=new THREE.Mesh(new THREE.CylinderGeometry(.2,.28,size*.4,6),mat('#805132'));trunk.position.set(x,y+size*.2,z);terrain.add(trunk);for(let i=0;i<3;i++){const m=new THREE.Mesh(new THREE.ConeGeometry(size*(.32-i*.055),size*.55,5),mat(i%2?'#7fa638':'#4e7e30'));m.position.set(x,y+size*(.48+i*.22),z);terrain.add(m);}}
const layout=globalThis.__VILLAGE_LAYOUT__||await fetch('layout.json').then(r=>{if(!r.ok)throw Error('Layout unavailable');return r.json();});
// Continuous gently sloping ground replaces the raised display plinths.
const groundGeometry=new THREE.PlaneGeometry(440,460,220,230);groundGeometry.rotateX(-Math.PI/2);groundGeometry.translate(0,0,-30);
const gp=groundGeometry.attributes.position;
for(let i=0;i<gp.count;i++)gp.setY(i,groundHeight(layout,gp.getX(i),gp.getZ(i)));
groundGeometry.computeVertexNormals();terrain.add(new THREE.Mesh(groundGeometry,mat('#56842e')));
function ribbon(points,width,color,height=.16){
 const vertices=[],indices=[];
 for(let i=0;i<points.length;i++){const p=points[i],before=points[Math.max(0,i-1)],after=points[Math.min(points.length-1,i+1)],n=new THREE.Vector3(-(after.z-before.z),0,after.x-before.x).normalize();
  for(const side of [-1,1]){const v=p.clone().addScaledVector(n,side*width/2);v.y=groundHeight(layout,v.x,v.z)+height;vertices.push(...v.toArray());}
  if(i<points.length-1){const a=i*2;indices.push(a,a+1,a+2,a+1,a+3,a+2);}
 }
 const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(vertices,3));geometry.setIndex(indices);geometry.computeVertexNormals();const mesh=new THREE.Mesh(geometry,mat(color));terrain.add(mesh);return mesh;
}
const riverCurve=new THREE.CatmullRomCurve3(layout.river.map(p=>new THREE.Vector3(...p)));
ribbon(riverCurve.getPoints(260),7,'#c1a773').name='River banks';
ribbon(riverCurve.getPoints(260).map(p=>p.add(new THREE.Vector3(0,.02,0))),4.8,'#358d9c',.19).name='River surface';
for(let i=0;i<12;i++){const hill=new THREE.Mesh(new THREE.SphereGeometry(1,12,8),mat(i%2?'#749266':'#93aa76'));hill.name='Distant hillside';hill.position.set(-180+i*35,-10,-200-(i%3)*18);hill.scale.set(45,27+(i%4)*9,35);terrain.add(hill);}
const stops=[],labels=[],loader=new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
let loadingFailed=false;
try {
  for(let i=0;i<layout.stops.length;i++){
    const s=layout.stops[i];$('status').textContent=`${i+1} / 12 · ${s.title}`;
    const embedded=document.getElementById(`asset-${s.id}`);
    let gltf;
    if(embedded){const bytes=Uint8Array.from(atob(embedded.textContent.trim()),c=>c.charCodeAt(0));embedded.remove();gltf=await loader.parseAsync(bytes.buffer,'');}
    else if(globalThis.__VILLAGE_LAYOUT__)throw Error('This HTML could not access its model files. Use OPEN-VILLAGE-MOBILE.html from the single-file mobile ZIP.');
    else gltf=await loader.loadAsync(s.model);
    const root=gltf.scene;
    root.traverse(o=>{if(o.isMesh)for(const m of Array.isArray(o.material)?o.material:[o.material]){if(m.isMeshBasicMaterial||m.transmission>0)m.toneMapped=false;}});
    root.updateMatrixWorld(true);const b=new THREE.Box3().setFromObject(root),size=b.getSize(new THREE.Vector3()),center=b.getCenter(new THREE.Vector3());
    const scale=s.width/Math.max(size.x,size.z);const anchor=new THREE.Group();anchor.name=s.id;anchor.position.fromArray(s.position);anchor.rotation.y=s.yaw;
    root.position.set(-center.x*scale,-b.min.y*scale,-center.z*scale);root.scale.setScalar(scale);anchor.add(root);scene.add(anchor);anchor.updateMatrixWorld(true);
    const worldBox=new THREE.Box3().setFromObject(anchor),worldSize=worldBox.getSize(new THREE.Vector3());
    const cap=box(`${s.id} threshold`,[s.position[0],s.position[1]-.12,s.position[2]],[size.x*scale+1,.18,size.z*scale+1],'#c5b899');cap.rotation.y=s.yaw;
    const direction=new THREE.Vector3(...s.lighting.reference_camera.eye).sub(center);
    direction.y=0;direction.normalize().applyAxisAngle(new THREE.Vector3(0,1,0),s.yaw);
    const target=new THREE.Vector3(s.position[0],s.position[1]+worldSize.y*.43,s.position[2]);
    const frontage=new THREE.Vector3(...s.position).addScaledVector(direction,s.width*1.2);frontage.y+=.12;

    stops.push({...s,anchor,box:worldBox,size:worldSize,footprint:new THREE.Vector3(size.x*scale,0,size.z*scale),target,direction,frontage});
    const opt=document.createElement('option');opt.value=i;opt.textContent=`${i+1}. ${s.title}`;$('stops').append(opt);
    await new Promise(resolve=>setTimeout(resolve,0));
  }
} catch(error){loadingFailed=true;$('status').textContent='A model could not be loaded.';$('error').textContent=error.message;$('retry').hidden=false;$('retry').onclick=()=>location.reload();console.error(error);}
if(loadingFailed)throw Error('Incomplete village');
const walking=makeWalkingRoutes(layout,stops);
const walkPaths=walking.routes.map(r=>r.points.map(p=>p.toArray()));
for(const route of walking.routes)ribbon(route.points,layout.walk.path_width,'#bf9a65').name='Ground-level walking path';
const frontages=addFrontages({layout,stops,walking,terrain,mat});
const neighborhood=addNeighborhood({layout,stops,walking,terrain,mat,riverCurve});
// Give the journey outdoor scale cues at the side of the paths, keeping the
// camera corridor and the original model footprints clear.
const routeSamples=walking.routes.flatMap(r=>r.points.filter((_,i)=>i%4===0));
function clearProp(p){return !walking.blocked(p.x,p.z)&&neighborhood.reservations.every(b=>p.x+3<b.min.x||p.x-3>b.max.x||p.z+3<b.min.z||p.z-3>b.max.z)&&routeSamples.every(q=>Math.hypot(q.x-p.x,q.z-p.z)>3.1);}
for(let i=0;i<walking.routes.length;i++){
 const route=walking.routes[i];
 for(let j=1;j<5;j++){
  const t=j/5,p=route.curve.getPointAt(t),tangent=route.curve.getTangentAt(t),normal=new THREE.Vector3(-tangent.z,0,tangent.x).normalize();
  p.addScaledVector(normal,(j%2?1:-1)*5.5);p.y=groundHeight(layout,p.x,p.z);
  if(!clearProp(p))continue;
  if(j%2){tree(p.x,p.y,p.z,4.5+(i%3)*.6);}
  else {const bench=new THREE.Group();bench.name='Pathside wooden bench';bench.scale.setScalar(1.6);bench.position.copy(p);bench.rotation.y=Math.atan2(tangent.x,tangent.z);terrain.add(bench);
   const part=(position,size,color)=>{const m=new THREE.Mesh(new THREE.BoxGeometry(...size),mat(color));m.position.set(...position);bench.add(m);};
   part([0,.55,0],[2.2,.15,.6],'#917045');part([0,1,-.27],[2.2,.75,.12],'#917045');for(const x of [-.85,.85])part([x,.25,0],[.12,.5,.45],'#555d4b');
  }
 }
}
for(let i=0;i<46;i++){const x=(i%2?-78:80)+Math.sin(i*13.3)*11,z=65-Math.floor(i/2)*9+Math.cos(i*7.1)*4;if(clearProp(new THREE.Vector3(x,0,z)))tree(x,groundHeight(layout,x,z),z,5+i%4);}

const landscape=addLandscape({layout,stops,walking,terrain,mat,riverCurve,reserved:neighborhood.reservations});

let playing=false,overviewMode=false,progress=0,hold=0,transition=null,last=performance.now();
const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
const pose=index=>walking.poses[index];
const viewAt=p=>walking.viewAt(p);
function apply(p){camera.position.copy(p.eye);controls.target.copy(p.target);camera.lookAt(p.target);dirty=true;}
function setPlaying(value){playing=value;$('play').textContent=value?'Pause':'Walk tour';}
function updateLabel(){const i=Math.round(progress);$('title').textContent=overviewMode?'Village overview':`${i+1} / 12 · ${stops[i].title}`;$('stops').value=i;$('progress').value=progress;}
function go(index,instant=false){
 setPlaying(false);scene.fog.near=100;scene.fog.far=300;camera.fov=camera.aspect<.8?82:60;camera.updateProjectionMatrix();const target=THREE.MathUtils.clamp(index,0,11),wasOverview=overviewMode;overviewMode=false;hold=0;
 // Adjacent section controls follow the same outdoor route as playback.
 // Distant jumps and return from overview use a brief fade instead of flying.
 if(!instant&&!reduced&&!wasOverview&&Math.abs(target-progress)<=1.01&&target!==progress){transition={from:progress,to:target,start:performance.now(),duration:Math.max(5000,walking.routes[Math.min(10,Math.floor(Math.min(progress,target)))].length/layout.walk.speed*1000)};}
 else {transition=null;progress=target;apply(viewAt(progress));if(!instant&&!reduced)canvas.animate([{opacity:.2},{opacity:1}],{duration:450});}
 updateLabel();
}
function overview(){setPlaying(false);overviewMode=true;transition=null;scene.fog.near=400;scene.fog.far=900;camera.fov=43;camera.updateProjectionMatrix();apply({eye:new THREE.Vector3(85,110,115).multiplyScalar(Math.max(1,.9/camera.aspect)),target:new THREE.Vector3(0,0,-25)});updateLabel();}
$('play').onclick=()=>{if(playing){setPlaying(false);return;}if(overviewMode||progress>=11)go(0,true);overviewMode=false;transition=null;setPlaying(true);updateLabel();};
$('overview').onclick=overview;$('next').onclick=()=>go(overviewMode?0:Math.floor(progress+1));$('previous').onclick=()=>go(Math.ceil(progress-1));
$('stops').onchange=event=>go(Number(event.target.value));$('progress').oninput=event=>go(Number(event.target.value),true);
controls.addEventListener('start',()=>{setPlaying(false);transition=null;});
window.addEventListener('keydown',event=>{if(/INPUT|SELECT|BUTTON/.test(document.activeElement.tagName))return;if(event.key==='ArrowRight')$('next').click();if(event.key==='ArrowLeft')$('previous').click();if(event.code==='Space'){event.preventDefault();$('play').click();}});
function resize(){renderer.setSize(innerWidth,innerHeight);camera.aspect=innerWidth/innerHeight;camera.fov=overviewMode?43:(camera.aspect<.8?82:60);camera.updateProjectionMatrix();dirty=true;if(!overviewMode)apply(viewAt(progress));}
window.addEventListener('resize',resize);resize();apply(viewAt(0));
$('loading').hidden=true;canvas.dataset.ready='true';setPlaying(false);updateLabel();
function frame(now){const dt=Math.min((now-last)/1000,.25);last=now;
 if(transition){const t=Math.min(1,(now-transition.start)/transition.duration),e=t*t*(3-2*t);progress=THREE.MathUtils.lerp(transition.from,transition.to,e);apply(viewAt(progress));updateLabel();if(t===1)transition=null;}
 else if(playing){if(hold>0)hold-=dt;else{const prev=progress,i=Math.min(10,Math.floor(progress));progress=Math.min(11,progress+dt*layout.walk.speed/walking.routes[i].length);if(Math.floor(progress)>Math.floor(prev)){progress=Math.floor(progress);hold=2.5;}apply(viewAt(progress));updateLabel();if(progress>=11)setPlaying(false);}}
 controls.update();if(dirty){renderer.render(scene,camera);dirty=false;}requestAnimationFrame(frame);
}requestAnimationFrame(frame);
window.village={go,overview,frontages:()=>frontages.check(),landscape:()=>landscape.check(),neighborhood:()=>neighborhood.check(),paths:()=>walkPaths,getState:()=>({ready:true,progress,playing,overviewMode,models:stops.length,eye:camera.position.toArray(),target:controls.target.toArray(),aspect:camera.aspect,triangles:renderer.info.render.triangles}),poses:()=>stops.map((s,i)=>({id:s.id,...Object.fromEntries(Object.entries(pose(i)).map(([k,v])=>[k,v.toArray()])),bounds:{min:s.box.min.toArray(),max:s.box.max.toArray()}})),
 routeSamples:()=>walking.routes.map((r,i)=>Array.from({length:201},(_,j)=>{const p=viewAt(i+j/200);return {progress:i+j/200,eye:p.eye.toArray(),target:p.target.toArray(),ground:groundHeight(layout,p.eye.x,p.eye.z),blocked:walking.blocked(p.eye.x,p.eye.z)};})),
 environment:()=>{terrain.updateMatrixWorld(true);const result=[];terrain.traverse(o=>{if(!o.isMesh)return;const p=o.geometry.attributes.position;const vertices=Array.from({length:p.count},(_,i)=>new THREE.Vector3().fromBufferAttribute(p,i).applyMatrix4(o.matrixWorld).toArray());result.push({name:o.name||'Landscape detail',vertices,indices:o.geometry.index?Array.from(o.geometry.index.array):Array.from({length:p.count},(_,i)=>i),color:o.material.color.toArray()});});return result;}
};
