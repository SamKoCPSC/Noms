import * as THREE from 'three';
import { groundHeight } from '../village/walking-world.js';
import {createPerimeter} from '../village/perimeter.js';

const bodyRadius = 0.3;
const adultEyeHeight = 1.65; // metres above the floor in the original room models
const groundPoint = ([x,,z]) => new THREE.Vector2(x,z);

export function createWalkSurface(layout, perimeter = createPerimeter(layout.walk.outline)) {
 const patios = layout.walk.entrances.map(({threshold,end,width,depth},i)=>{
  const b=groundPoint(end);
  const normal=b.clone().sub(groundPoint(threshold)).normalize();
  const tangent=new THREE.Vector2(normal.y,-normal.x);
  const center=b.clone().addScaledVector(normal,0.1-depth/2);
  const {position,yaw,halfSize,scale,width:roomWidth}=layout.walk.stops[i];
  return {center,normal,tangent,width,depth,level:end[1]-0.015,
          position,cos:Math.cos(yaw),sin:Math.sin(yaw),halfSize,scale,roomWidth,roomLevel:threshold[1]};
 });
 // Match the continuous stone surface used in the baked world.
 function sample(x,z) {
  const ground=groundHeight(layout.walk,x,z);
  let y=ground+0.19;
  for(const {center,tangent,normal,width,depth,level} of patios){
   const dx=x-center.x,dz=z-center.y;
   const u=Math.abs(dx*tangent.x+dz*tangent.y)-width/2;
   const v=Math.abs(dx*normal.x+dz*normal.y)-depth/2;
   const d=Math.hypot(Math.max(0,u),Math.max(0,v));
   y=THREE.MathUtils.lerp(level,y,THREE.MathUtils.smoothstep(d,0,2));
  }
  // Ease up onto each diorama's raised floor and back down at its edges.
  let eyeScale=1;
  for (const {position,cos,sin,halfSize,roomLevel,scale,roomWidth} of patios) {
   const ox=x-position[0], oz=z-position[2];
   const dx=Math.max(0,Math.abs(cos*ox-sin*oz)-halfSize[0]);
   const dz=Math.max(0,Math.abs(sin*ox+cos*oz)-halfSize[1]);
   const distance=Math.hypot(dx,dz);
   // Rooms were enlarged to fit the village. Ease back to outdoor scale over one room width.
   const influence=1-THREE.MathUtils.smoothstep(distance,0,roomWidth);
   eyeScale=Math.max(eyeScale,1+(scale-1)*influence);
   if(distance<1.5) y=THREE.MathUtils.lerp(roomLevel,y,THREE.MathUtils.smoothstep(distance,0,1.5));
  }
  return {floor:Math.max(ground+0.07,y),eyeHeight:adultEyeHeight*eyeScale};
 }
 return {constrain: (x,z) => perimeter.constrain(x,z,bodyRadius),sample};
}

export function createFirstPersonCamera(layout, perimeter) {
 const surface=createWalkSurface(layout, perimeter);
 let position=null, initialYaw=0, stepPhase=0;
 return (camera,walk)=>{
  if(!walk?.length){position=null;stepPhase=0;return;}
  camera.rotation.reorder('YXZ');
  if(!position){
   position=surface.constrain(camera.position.x,camera.position.z);
   initialYaw=camera.rotation.y;
  }
  const [weight,turn,pitch,side,forward,eyeOffset=0,stepLength=0,bobAmplitude=0]=walk;
  const yaw=initialYaw+turn, sin=Math.sin(yaw), cos=Math.cos(yaw);
  const next=surface.constrain(position[0]+side*cos-forward*sin,position[1]-side*sin-forward*cos);
  const [x,z]=next;
  if(bobAmplitude===0) stepPhase=0;
  else if(stepLength>0){
   // Use actual travel so pushing straight into a fence cannot generate footsteps.
   const travel=Math.hypot(x-position[0],z-position[1]);
   stepPhase=(stepPhase+travel/stepLength)%1;
  }
  position=next;
  const {floor,eyeHeight}=surface.sample(x,z);
  const bob=Math.sin(2*Math.PI*stepPhase)*bobAmplitude*(eyeHeight/adultEyeHeight);
  camera.position.lerp(new THREE.Vector3(x,floor+eyeHeight+eyeOffset+bob,z),weight);
  camera.rotation.y+=weight*Math.atan2(Math.sin(yaw-camera.rotation.y),Math.cos(yaw-camera.rotation.y));
  camera.rotation.x=THREE.MathUtils.lerp(camera.rotation.x,pitch,weight);
  camera.fov=THREE.MathUtils.lerp(camera.fov,65,weight);
  camera.view.offsetX*=1-weight;
  camera.view.offsetY*=1-weight;
  camera.updateProjectionMatrix();
  camera.updateMatrixWorld(true);
 };
}
