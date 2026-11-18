// The pilot uses the paved kitchen–library path, its two entrance branches
// and the forecourts. The inset keeps the player's body clear of the edges.
import * as THREE from 'three';
import { groundHeight } from '../village/walking-world.js';

const bodyRadius = 0.3;
const eyeHeight = 1.65;
const groundPoint = ([x,,z]) => new THREE.Vector2(x,z);

export function createWalkSurface(layout) {
 const segments = [], patios = [];
 function addPath(points, width) {
  for (let i=1; i<points.length; i++) {
   const a=points[i-1], delta=points[i].clone().sub(a);
   segments.push({a,delta,lengthSq:delta.lengthSq(),radius:width/2-bodyRadius});
  }
 }
 addPath(layout.routes[0].map(sample=>groundPoint(sample.eye)),layout.walk.pathWidth);
 layout.walk.entrances.forEach(({threshold,end,width,depth},i)=>{
  const b=groundPoint(end);
  const normal=b.clone().sub(groundPoint(threshold)).normalize();
  const tangent=new THREE.Vector2(normal.y,-normal.x);
  const arrival=groundPoint(layout.stops[i].pose.eye);
  const control=b.clone().addScaledVector(normal,2.5);
  addPath(new THREE.QuadraticBezierCurve(arrival,control,b).getPoints(24),2.5);
  const center=b.clone().addScaledVector(normal,0.1-depth/2);
  patios.push({center,normal,tangent,width,depth,level:end[1]-0.015});
 });
 function constrain(x,z) {
  let best=Infinity, outX=x, outZ=z;
  for (const {a,delta,lengthSq,radius} of segments) {
   const t=THREE.MathUtils.clamp(((x-a.x)*delta.x+(z-a.y)*delta.y)/lengthSq,0,1);
   const cx=a.x+t*delta.x, cz=a.y+t*delta.y;
   const distance=Math.hypot(x-cx,z-cz), excess=distance-radius;
   if (excess<=0) return [x,z];
   if (excess<best) {
    best=excess;
    outX=cx+(x-cx)*radius/distance;
    outZ=cz+(z-cz)*radius/distance;
   }
  }
  for (const {center,normal,tangent,width,depth} of patios) {
   const dx=x-center.x, dz=z-center.y;
   const u=THREE.MathUtils.clamp(dx*tangent.x+dz*tangent.y,-width/2+bodyRadius,width/2-bodyRadius);
   const v=THREE.MathUtils.clamp(dx*normal.x+dz*normal.y,-depth/2+bodyRadius,depth/2-bodyRadius);
   const cx=center.x+tangent.x*u+normal.x*v, cz=center.y+tangent.y*u+normal.y*v;
   const distance=Math.hypot(cx-x,cz-z);
   if(distance<best){best=distance;outX=cx;outZ=cz;}
  }
  return [outX,outZ];
 }
 // Match the continuous stone surface used in the baked world.
 function floor(x,z) {
  const ground=groundHeight(layout.walk,x,z);
  let y=ground+0.19;
  for(const {center,tangent,normal,width,depth,level} of patios){
   const dx=x-center.x,dz=z-center.y;
   const u=Math.abs(dx*tangent.x+dz*tangent.y)-width/2;
   const v=Math.abs(dx*normal.x+dz*normal.y)-depth/2;
   const d=Math.hypot(Math.max(0,u),Math.max(0,v));
   y=THREE.MathUtils.lerp(level,y,THREE.MathUtils.smoothstep(d,0,2));
  }
  return Math.max(ground+0.07,y);
 }
 return {constrain,floor};
}

export function createFirstPersonCamera(layout) {
 const surface=createWalkSurface(layout);
 let position=null, initialYaw=0;
 return (camera,walk)=>{
  if(!walk?.length){position=null;return;}
  camera.rotation.reorder('YXZ');
  if(!position){
   position=surface.constrain(camera.position.x,camera.position.z);
   initialYaw=camera.rotation.y;
  }
  const [weight,turn,pitch,side,forward]=walk;
  const yaw=initialYaw+turn, sin=Math.sin(yaw), cos=Math.cos(yaw);
  position=surface.constrain(position[0]+side*cos-forward*sin,position[1]-side*sin-forward*cos);
  const [x,z]=position;
  camera.position.lerp(new THREE.Vector3(x,surface.floor(x,z)+eyeHeight,z),weight);
  camera.rotation.y+=weight*Math.atan2(Math.sin(yaw-camera.rotation.y),Math.cos(yaw-camera.rotation.y));
  camera.rotation.x=THREE.MathUtils.lerp(camera.rotation.x,pitch,weight);
  camera.fov=THREE.MathUtils.lerp(camera.fov,65,weight);
  camera.view.offsetX*=1-weight;
  camera.view.offsetY*=1-weight;
  camera.updateProjectionMatrix();
  camera.updateMatrixWorld(true);
 };
}
