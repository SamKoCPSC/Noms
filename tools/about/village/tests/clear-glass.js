import * as THREE from 'three';
import { configureClearGlass } from '../clear-glass.js';
const renderer=new THREE.WebGLRenderer({antialias:false,preserveDrawingBuffer:true});
renderer.setSize(512,512);renderer.transmissionResolutionScale=1;
renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.NoToneMapping;
document.body.appendChild(renderer.domElement);
const scene=new THREE.Scene();scene.background=new THREE.Color('black');
const camera=new THREE.PerspectiveCamera(60,1,.1,10);camera.position.z=3;
const bytes=new Uint8Array(256*256*4);
for(let y=0;y<256;y++)for(let x=0;x<256;x++){
 const i=(y*256+x)*4,v=Math.floor(x/8)%2?255:0;bytes.set([v,v,v,255],i);
}
const texture=new THREE.DataTexture(bytes,256,256);texture.magFilter=THREE.NearestFilter;texture.minFilter=THREE.NearestFilter;texture.needsUpdate=true;
const backdrop=new THREE.Mesh(new THREE.PlaneGeometry(3,3),new THREE.MeshBasicMaterial({map:texture,toneMapped:false}));scene.add(backdrop);
const base=new THREE.MeshPhysicalMaterial({color:'white',roughness:0,metalness:0,transmission:1,ior:1.45,thickness:0,toneMapped:false});
const glass=new THREE.Mesh(new THREE.PlaneGeometry(2,2),base);glass.position.z=.2;scene.add(glass);
function sample(){
 renderer.render(scene,camera);const gl=renderer.getContext(),pixels=new Uint8Array(512*4);
 gl.readPixels(0,256,512,1,gl.RGBA,gl.UNSIGNED_BYTE,pixels);
 const row=Array.from({length:200},(_,i)=>pixels[(i+156)*4]);
 return {softPixels:row.filter(v=>v>30&&v<220).length,range:[Math.min(...row),Math.max(...row)],row};
}
glass.visible=false;const uncovered=sample();glass.visible=true;const standard=sample();
const clear=base.clone();clear.userData.village_clear_glass=true;configureClearGlass(clear);glass.material=clear;
const polished=sample();
// A uniform dark backdrop isolates fog from stripe edge/refraction differences.
backdrop.material.map=null;backdrop.material.color.set('#222222');backdrop.material.needsUpdate=true;
scene.fog=new THREE.Fog('#aabbcc',1,8);
glass.visible=false;const fogBackground=sample();glass.visible=true;glass.material=clear;const fogClear=sample();glass.material=base;const fogStandard=sample();
const error=sample=>sample.row.reduce((n,v,i)=>n+Math.abs(v-fogBackground.row[i]),0)/sample.row.length;
glass.material=clear;renderer.render(scene,camera);
window.results={fogValues:{background:fogBackground.range,clear:fogClear.range,standard:fogStandard.range},fogError:{standard:error(fogStandard),polished:error(fogClear)},fogAppliedTwice:clear.fog,uncovered,standard,polished,physical:clear.isMeshPhysicalMaterial,transmission:clear.transmission,ior:clear.ior,opacity:clear.opacity,alphaBlended:clear.transparent};
