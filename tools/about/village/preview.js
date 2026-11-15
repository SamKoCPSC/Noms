import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { MeshoptDecoder } from 'meshoptimizer';

const $ = id => document.getElementById(id);
const canvas = document.querySelector('canvas');
let renderer;
try { renderer = new THREE.WebGLRenderer({canvas, antialias:true, logarithmicDepthBuffer:true}); }
catch { $('status').textContent='This preview needs a browser with WebGL.'; throw Error('WebGL unavailable'); }
renderer.setPixelRatio(Math.min(devicePixelRatio,1.5));
renderer.outputColorSpace=THREE.SRGBColorSpace; renderer.toneMapping=THREE.AgXToneMapping;
const scene=new THREE.Scene(); scene.background=new THREE.Color('#d6e6e9');
scene.fog=new THREE.Fog('#d6e6e9',190,390);
const sky=new THREE.Mesh(new THREE.SphereGeometry(480,24,12),new THREE.ShaderMaterial({side:THREE.BackSide,depthWrite:false,toneMapped:false,uniforms:{top:{value:new THREE.Color('#a4c9df')},bottom:{value:new THREE.Color('#ede8dc')}},vertexShader:'varying vec3 v; void main(){v=position;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',fragmentShader:'varying vec3 v;uniform vec3 top;uniform vec3 bottom;void main(){float h=smoothstep(-.05,.65,normalize(v).y);gl_FragColor=vec4(mix(bottom,top,h),1.);\n#include <tonemapping_fragment>\n#include <colorspace_fragment>\n}'}));scene.add(sky);
const camera=new THREE.PerspectiveCamera(43,1,.1,650);
const controls=new OrbitControls(camera,canvas);controls.enableDamping=true;controls.minDistance=5;controls.maxDistance=260;controls.maxPolarAngle=Math.PI*.48;
let dirty=true;controls.addEventListener('change',()=>{dirty=true;});
scene.add(new THREE.HemisphereLight('#e4f1ff','#b1a084',2.2));
const sun=new THREE.DirectionalLight('#fff2d9',2.8);sun.position.set(-60,100,50);scene.add(sun);
const pmrem=new THREE.PMREMGenerator(renderer), room=new RoomEnvironment();
scene.environment=pmrem.fromScene(room,.04).texture;room.dispose();pmrem.dispose();
const terrain=new THREE.Group();terrain.name='Village blockout terrain';scene.add(terrain);
const materials={};
function mat(color){return materials[color]??=new THREE.MeshStandardMaterial({color,roughness:.95});}
function box(name,p,size,color){const m=new THREE.Mesh(new THREE.BoxGeometry(...size),mat(color));m.name=name;m.position.fromArray(p);terrain.add(m);return m;}
function line(points,color,radius=.5){const c=new THREE.CatmullRomCurve3(points.map(p=>new THREE.Vector3(...p)));const m=new THREE.Mesh(new THREE.TubeGeometry(c,Math.max(40,points.length*12),radius,6,false),mat(color));terrain.add(m);return c;}
function label(text,x,y,z){const c=document.createElement('canvas');c.width=512;c.height=80;const ctx=c.getContext('2d');ctx.fillStyle='#faf6e9';ctx.beginPath();ctx.roundRect(0,0,512,80,24);ctx.fill();ctx.fillStyle='#3f4935';ctx.font='600 29px system-ui';ctx.textAlign='center';ctx.fillText(text,256,51);const tex=new THREE.CanvasTexture(c);tex.colorSpace=THREE.SRGBColorSpace;const sprite=new THREE.Sprite(new THREE.SpriteMaterial({map:tex,depthTest:true}));sprite.position.set(x,y,z);sprite.scale.set(8,1.25,1);scene.add(sprite);return sprite;}
function tree(x,y,z,size){const trunk=new THREE.Mesh(new THREE.CylinderGeometry(.2,.28,size*.4,6),mat('#79634a'));trunk.position.set(x,y+size*.2,z);terrain.add(trunk);for(let i=0;i<3;i++){const m=new THREE.Mesh(new THREE.ConeGeometry(size*(.32-i*.055),size*.55,5),mat(i%2?'#8f9f6c':'#80915f'));m.position.set(x,y+size*(.48+i*.22),z);terrain.add(m);}}
const layout=globalThis.__VILLAGE_LAYOUT__||await fetch('layout.json').then(r=>{if(!r.ok)throw Error('Layout unavailable');return r.json();});
const shape=new THREE.Shape();layout.ground_outline.forEach(([x,z],i)=>i?shape.lineTo(x,-z):shape.moveTo(x,-z));shape.closePath();
const land=new THREE.Mesh(new THREE.ExtrudeGeometry(shape,{depth:2,bevelEnabled:true,bevelSize:1,bevelThickness:.5,bevelSegments:2,steps:1}),mat('#a5ad87'));
land.rotation.x=-Math.PI/2;land.position.y=-3;terrain.add(land);
const river=layout.river.map(p=>[p[0],-.7,p[2]]);line(river,'#86b7c4',2.6);
// Backdrop deliberately stays simple during the layout review.
for(let i=0;i<10;i++){const hill=new THREE.Mesh(new THREE.SphereGeometry(1,10,6),mat(i%2?'#a4b5a3':'#b4c2ad'));hill.position.set(-110+i*26,-12,-125-(i%3)*10);hill.scale.set(28,21+(i%4)*6,24);terrain.add(hill);}
for(let i=0;i<16;i++)tree(-47+(i%2)*4,-.5,32-i*7,4+i%3);
for(let i=0;i<13;i++)tree(43+(i%2)*4,-.5,28-i*7,4+i%3);
const stops=[],labels=[],loader=new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
let loadingFailed=false;
try {
  for(let i=0;i<layout.stops.length;i++){
    const s=layout.stops[i];$('status').textContent=`${i+1} / 12 · ${s.title}`;
    const embedded=document.getElementById(`asset-${s.id}`);
    let gltf;
    if(embedded){const bytes=Uint8Array.from(atob(embedded.textContent.trim()),c=>c.charCodeAt(0));embedded.remove();gltf=await loader.parseAsync(bytes.buffer,'');}
    else gltf=await loader.loadAsync(s.model);
    const root=gltf.scene;
    root.traverse(o=>{if(o.isMesh)for(const m of Array.isArray(o.material)?o.material:[o.material]){if(m.isMeshBasicMaterial||m.transmission>0)m.toneMapped=false;}});
    root.updateMatrixWorld(true);const b=new THREE.Box3().setFromObject(root),size=b.getSize(new THREE.Vector3()),center=b.getCenter(new THREE.Vector3());
    const scale=s.width/Math.max(size.x,size.z);const anchor=new THREE.Group();anchor.name=s.id;anchor.position.fromArray(s.position);anchor.rotation.y=s.yaw;
    root.position.set(-center.x*scale,-b.min.y*scale,-center.z*scale);root.scale.setScalar(scale);anchor.add(root);scene.add(anchor);anchor.updateMatrixWorld(true);
    const worldBox=new THREE.Box3().setFromObject(anchor),worldSize=worldBox.getSize(new THREE.Vector3());
    const pad=box(`${s.id} terrace`,[s.position[0],(s.position[1]-1)/2-.08,s.position[2]],[size.x*scale+3,s.position[1]+1,size.z*scale+3],'#cebea2');pad.rotation.y=s.yaw;
    const cap=box(`${s.id} paving`,[s.position[0],s.position[1]-.14,s.position[2]],[size.x*scale+2.8,.22,size.z*scale+2.8],'#e3d6bc');cap.rotation.y=s.yaw;
    const direction=new THREE.Vector3(...s.lighting.reference_camera.eye).sub(center);
    direction.y=0;direction.normalize().applyAxisAngle(new THREE.Vector3(0,1,0),s.yaw);
    const target=new THREE.Vector3(s.position[0],s.position[1]+worldSize.y*.43,s.position[2]);
    const frontage=new THREE.Vector3(...s.position).addScaledVector(direction,s.width*.95);frontage.y+=.12;
    labels.push(label(`${String(i+1).padStart(2,'0')} · ${s.title}`,frontage.x,frontage.y+1.2,frontage.z));
    stops.push({...s,anchor,box:worldBox,size:worldSize,target,direction,frontage});
    const opt=document.createElement('option');opt.value=i;opt.textContent=`${i+1}. ${s.title}`;$('stops').append(opt);
    await new Promise(resolve=>setTimeout(resolve,0));
  }
} catch(error){loadingFailed=true;$('status').textContent='A model could not be loaded.';$('error').textContent=error.message;$('retry').hidden=false;$('retry').onclick=()=>location.reload();console.error(error);}
if(loadingFailed)throw Error('Incomplete village');
// Route the connecting paths around every plot, never through the room floors.
// A coarse deterministic grid is enough for a review blockout; final paths will
// replace these elevated links with sculpted terrain, ramps and stairs.
const step=1.5,key=(x,z)=>`${x},${z}`,obstacles=stops.map(s=>s.box.clone().expandByScalar(2.6));
function blocked(x,z){return obstacles.some(b=>x>b.min.x&&x<b.max.x&&z>b.min.z&&z<b.max.z);}
const neighbors=[[1,0],[-1,0],[0,1],[0,-1]],outside=new Set(),queue=[[-43,36]];
for(let cursor=0;cursor<queue.length;cursor++){const [x,z]=queue[cursor],k=key(x,z);if(outside.has(k)||x< -43||x>43||z< -65||z>36||blocked(x*step,z*step))continue;outside.add(k);for(const [dx,dz] of neighbors)queue.push([x+dx,z+dz]);}
function nearest(p){const x=Math.round(p.x/step),z=Math.round(p.z/step);for(let r=0;r<50;r++)for(let dx=-r;dx<=r;dx++)for(let dz=-r;dz<=r;dz++){if(Math.max(Math.abs(dx),Math.abs(dz))!==r)continue;if(outside.has(key(x+dx,z+dz)))return [x+dx,z+dz];}throw Error('No clear path frontage');}
function findPath(a,b){const start=nearest(a),end=nearest(b),goal=key(...end),open=[{x:start[0],z:start[1],g:0,f:0}],seen=new Map([[key(...start),0]]),parents=new Map();
 while(open.length){open.sort((a,b)=>b.f-a.f);const c=open.pop(),k=key(c.x,c.z);if(k===goal){let k2=k,points=[];while(k2){points.push(k2.split(',').map(Number));k2=parents.get(k2);}points.reverse();const simple=points.filter((p,i)=>!i||i===points.length-1||(p[0]-points[i-1][0]!==points[i+1][0]-p[0])||(p[1]-points[i-1][1]!==points[i+1][1]-p[1]));return simple.map((p,i)=>[p[0]*step,THREE.MathUtils.lerp(a.y,b.y,i/Math.max(1,simple.length-1)),p[1]*step]);}
 for(const [dx,dz] of [[1,0],[-1,0],[0,1],[0,-1]]){const x=c.x+dx,z=c.z+dz,nk=key(x,z),g=c.g+1;if(x< -43||x>43||z< -65||z>36||blocked(x*step,z*step)||(seen.has(nk)&&seen.get(nk)<=g))continue;seen.set(nk,g);parents.set(nk,k);open.push({x,z,g,f:g+Math.abs(x-end[0])+Math.abs(z-end[1])});}}
 throw Error('No clear path between locations');
}
const walkPaths=[];
function walkway(points){
  const c=new THREE.CatmullRomCurve3(points.map(p=>new THREE.Vector3(...p))),vertices=[],indices=[],samples=100;
  for(let j=0;j<=samples;j++){const p=c.getPoint(j/samples),t=c.getTangent(j/samples),n=new THREE.Vector3(-t.z,0,t.x).normalize();
    for(const [side,dy] of [[-1,.04],[1,.04],[-1,-.24],[1,-.24]]){const v=p.clone().addScaledVector(n,side*.85);v.y+=dy;vertices.push(...v.toArray());}
    if(j<samples){const a=j*4,b=a+4;indices.push(a,b,a+1,a+1,b,b+1,a+2,a+3,b+2,a+3,b+3,b+2,a,a+2,b,a+2,b+2,b,a+1,b+1,a+3,a+3,b+1,b+3);}
    if(j%12===0&&p.y>2)box('Bridge support',[p.x,(p.y-1)/2,p.z],[.55,p.y+1,.55],'#bdad91');
  }
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(vertices,3));g.setIndex(indices);g.computeVertexNormals();const deck=new THREE.Mesh(g,mat('#d3bc96'));terrain.add(deck);
  const l=new THREE.Line(new THREE.BufferGeometry().setFromPoints(c.getPoints(160).map(p=>p.add(new THREE.Vector3(0,.07,0)))),new THREE.LineDashedMaterial({color:'#ae613b',dashSize:1,gapSize:.65}));l.computeLineDistances();scene.add(l);
}
for(let i=1;i<stops.length;i++){const points=findPath(stops[i-1].frontage,stops[i].frontage);walkPaths.push(points);walkway(points);}

let playing=false,overviewMode=true,progress=0,hold=0,transition=null,last=performance.now();
const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
function pose(index){const s=stops[index];const aspect=camera.aspect;const radius=s.size.length()/2;
  const distance=radius/Math.sin(THREE.MathUtils.degToRad(43)/2)*Math.max(1,1/aspect)*.92;
  return {eye:s.target.clone().addScaledVector(s.direction,distance*.79).add(new THREE.Vector3(0,distance*.61,0)),target:s.target.clone()};}
function viewAt(p){const a=Math.min(11,Math.floor(p)),t=p-a;const pa=pose(a);if(a===11||t===0)return pa;
  const pb=pose(a+1),mid=pa.eye.clone().lerp(pb.eye,.5);mid.y=Math.max(pa.eye.y,pb.eye.y,stops[a].box.max.y,stops[a+1].box.max.y)+13;
  const curve=new THREE.CatmullRomCurve3([pa.eye,pa.eye.clone().lerp(mid,.45),mid,pb.eye.clone().lerp(mid,.45),pb.eye],false,'centripetal');
  const eased=t*t*(3-2*t);return {eye:curve.getPoint(eased),target:pa.target.clone().lerp(pb.target,eased)};}
function apply(p){camera.position.copy(p.eye);controls.target.copy(p.target);camera.lookAt(p.target);dirty=true;}
function setPlaying(value){playing=value;$('play').textContent=value?'Pause':'Play tour';}
function moveTo(p,duration=1600){transition={from:{eye:camera.position.clone(),target:controls.target.clone()},to:p,start:performance.now(),duration:reduced?0:duration};}
function updateLabel(){const i=Math.round(progress);$('title').textContent=overviewMode?'Village overview':`${i+1} / 12 · ${stops[i].title}`;$('stops').value=i;$('progress').value=progress;}
function go(index,instant=false){setPlaying(false);overviewMode=false;progress=THREE.MathUtils.clamp(index,0,11);hold=0;if(instant){transition=null;apply(viewAt(progress));}else moveTo(viewAt(progress));updateLabel();}
function overview(){setPlaying(false);overviewMode=true;transition=null;moveTo({eye:new THREE.Vector3(100,108,135).multiplyScalar(Math.max(1,.8/camera.aspect)),target:new THREE.Vector3(0,6,-22)},1800);updateLabel();}
$('play').onclick=()=>{if(playing){setPlaying(false);return;}if(overviewMode||progress>=11){go(0);hold=2;}overviewMode=false;setPlaying(true);updateLabel();};
$('overview').onclick=overview;$('next').onclick=()=>go(overviewMode?0:Math.floor(progress+1));$('previous').onclick=()=>go(Math.ceil(progress-1));
$('stops').onchange=event=>go(Number(event.target.value));$('progress').oninput=event=>go(Number(event.target.value),true);
controls.addEventListener('start',()=>{setPlaying(false);transition=null;});
window.addEventListener('keydown',event=>{if(/INPUT|SELECT|BUTTON/.test(document.activeElement.tagName))return;if(event.key==='ArrowRight')$('next').click();if(event.key==='ArrowLeft')$('previous').click();if(event.code==='Space'){event.preventDefault();$('play').click();}});
function resize(){renderer.setSize(innerWidth,innerHeight);camera.aspect=innerWidth/innerHeight;camera.updateProjectionMatrix();dirty=true;if(!overviewMode&&!transition)apply(viewAt(progress));}
window.addEventListener('resize',resize);resize();apply({eye:new THREE.Vector3(100,108,135).multiplyScalar(Math.max(1,.8/camera.aspect)),target:new THREE.Vector3(0,6,-22)});
$('loading').hidden=true;canvas.dataset.ready='true';updateLabel();
function frame(now){const dt=Math.min((now-last)/1000,.1);last=now;
  if(transition){const t=transition.duration?Math.min(1,(now-transition.start)/transition.duration):1,e=t*t*(3-2*t);apply({eye:transition.from.eye.clone().lerp(transition.to.eye,e),target:transition.from.target.clone().lerp(transition.to.target,e)});if(t===1)transition=null;}
  else if(playing){if(hold>0)hold-=dt;else{const prev=progress;progress=Math.min(11,progress+dt/8);if(Math.floor(progress)>Math.floor(prev)){progress=Math.floor(progress);hold=3;}apply(viewAt(progress));updateLabel();if(progress>=11)setPlaying(false);}}
  controls.update();if(dirty){renderer.render(scene,camera);dirty=false;}requestAnimationFrame(frame);
}requestAnimationFrame(frame);
window.village={go,overview,paths:()=>walkPaths,getState:()=>({ready:true,progress,playing,overviewMode,models:stops.length,eye:camera.position.toArray(),target:controls.target.toArray(),aspect:camera.aspect,triangles:renderer.info.render.triangles}),poses:()=>stops.map((s,i)=>({id:s.id,...Object.fromEntries(Object.entries(pose(i)).map(([k,v])=>[k,v.toArray()])),bounds:{min:s.box.min.toArray(),max:s.box.max.toArray()}}))};
