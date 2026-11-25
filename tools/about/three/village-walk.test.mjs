import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import * as THREE from 'three';
import {frameCamera} from './village-camera.js';
import {createFirstPersonCamera,createWalkSurface} from './village-walk.js';

import {createFence,createPerimeter} from '../village/perimeter.js';
import {groundHeight} from '../village/walking-world.js';

const layout=JSON.parse(readFileSync('assets/about/village/layout.json'));
const surface=createWalkSurface(layout);
const emptyMap={gap:(_x,_z,bottom)=>({bottom,top:Infinity})};

const outline=JSON.parse(readFileSync('assets/models/terraced-village/layout.json')).ground_outline;
const xs=outline.map(p=>p[0]), zs=outline.map(p=>p[1]);
const west=Math.min(...xs), east=Math.max(...xs), north=Math.min(...zs), south=Math.max(...zs);

test('all twelve stops and route samples lie inside the walking perimeter',()=>{
 for(const route of layout.routes) for(const {eye:[x,,z]} of route) {
  assert.deepEqual(surface.constrain(x,z),[x,z]);
 }
 for(const [i,{position:[x,,z]}] of layout.walk.stops.entries()) {
  assert.deepEqual(surface.constrain(x,z),[x,z]);
  assert.ok(Math.abs(surface.sample(x,z).floor-layout.walk.entrances[i].threshold[1])<1e-5);
 }
 for(let x=west+1;x<east;x+=7) for(let z=north+1;z<south;z+=7) {
  assert.deepEqual(surface.constrain(x,z),[x,z]);
  assert.ok(Number.isFinite(surface.sample(x,z).floor));
 }
});

test('standing eye level follows the authored room scale and eases down to meadow scale',()=>{
 const anchors=JSON.parse(readFileSync('assets/models/terraced-village/daylight/source/anchors.json'));
 for(const [i,{position:[x,,z]}] of layout.walk.stops.entries()){
  const scale=anchors[layout.stops[i].id].scale;
  const {eyeHeight}=surface.sample(x,z);
  assert.ok(Math.abs(eyeHeight/scale-1.65)<1e-6,layout.stops[i].id);
 }
 const kitchen=layout.walk.stops[0];
 const [x,,z]=kitchen.position;
 const outward=d=>surface.sample(x+d*Math.sin(kitchen.yaw),z+d*Math.cos(kitchen.yaw)).eyeHeight;
 const edge=kitchen.halfSize[1], width=kitchen.width;
 assert.ok(outward(edge)>outward(edge+width/2));
 assert.ok(outward(edge+width/2)>outward(edge+width));
 assert.ok(Math.abs(outward(edge+width)-1.65)<1e-6);
});

test('even a large sprint step stays inside the posts and can slide along every side',()=>{
 const inset=0.4;
 for(const farX of [-10000,10000]) for(const farZ of [-10000,10000]) {
  const x=farX<0?west+inset:east-inset, z=farZ<0?north+inset:south-inset;
  assert.deepEqual(surface.constrain(farX,farZ),[x,z]);
  assert.deepEqual(surface.constrain(farX,17),[x,17]);
  assert.deepEqual(surface.constrain(14,farZ),[14,z]);
 }
});

test('the visible rails join each neighboring post, including the last corner, on the ground outline',()=>{
 assert.deepEqual(layout.walk.outline,outline);
 const [posts,rails]=createFence(createPerimeter(outline),layout.walk).children;
 assert.equal(rails.count,posts.count*2);
 const at=(mesh,i)=>{const matrix=new THREE.Matrix4();mesh.getMatrixAt(i,matrix);return matrix;};
 const base=i=>new THREE.Vector3().setFromMatrixPosition(at(posts,i));
 for(let i=0;i<posts.count;i++) {
  const point=base(i),next=base((i+1)%posts.count);
  assert.ok(point.x===west||point.x===east||point.z===north||point.z===south);
  assert.ok(Math.abs(point.y-groundHeight(layout.walk,point.x,point.z))<1e-5);
  assert.ok(point.distanceTo(next)<3.21 && point.distanceTo(next)>0);
  for(let level=0;level<2;level++){
   const rail=at(rails,i*2+level), lift=new THREE.Vector3(0,0.52+0.58*level,0);
   assert.ok(new THREE.Vector3(-0.5,0,0).applyMatrix4(rail).distanceTo(point.clone().add(lift))<1e-4);
   assert.ok(new THREE.Vector3(0.5,0,0).applyMatrix4(rail).distanceTo(next.clone().add(lift))<1e-4);
  }
 }
});

test('the walk uses eye height and camera-relative travel, and restores the guided view exactly',()=>{
 const camera=new THREE.PerspectiveCamera(), walk=createFirstPersonCamera(layout,undefined,emptyMap);
 function draw(input){
  frameCamera(camera,layout,.48,393,852,[8,450,377,350]);
  walk(camera,input);
 }
 draw([]);
 const savedEye=camera.position.clone(),savedProjection=camera.projectionMatrix.clone(),savedRotation=camera.quaternion.clone();
 draw([0,0,0,0,0]);
 assert.ok(camera.position.equals(savedEye));
 draw([1,Math.PI/2,0,0,0]);
 const eye=camera.position.clone(), forward=camera.getWorldDirection(new THREE.Vector3());
 const standing=surface.sample(eye.x,eye.z);
 assert.ok(Math.abs(eye.y-standing.floor-standing.eyeHeight)<1e-6);
 assert.ok(Math.abs(forward.y)<1e-6);
 draw([1,Math.PI/2,0,0,.1]);
 assert.ok(camera.position.clone().sub(eye).dot(forward)>.099);
 draw([]);
 assert.ok(camera.position.equals(savedEye));
 assert.ok(camera.projectionMatrix.equals(savedProjection));
 assert.ok(camera.quaternion.angleTo(savedRotation)<1e-6);
 draw([1,0,0,0,0]);
 assert.ok(Math.abs(camera.position.x-eye.x)<1e-6);
});

test('footsteps follow grounded distance and settle without tilting the view',()=>{
 const camera=new THREE.PerspectiveCamera(), walk=createFirstPersonCamera(layout,undefined,emptyMap);
 function draw(forward,stepLength,amplitude,lift=0){
  frameCamera(camera,layout,0,900,600,[350,0,550,600]);
  walk(camera,[1,0,0,0,forward,lift,stepLength,amplitude]);
  const {x,y,z}=camera.position;
  const {floor,eyeHeight}=surface.sample(x,z);
  // Compare in authored metres at every point on the path.
  return (y-floor-eyeHeight-lift)*1.65/eyeHeight;
 }
 const close=(actual,expected)=>assert.ok(Math.abs(actual-expected)<1e-5);
 close(draw(0,0,0),0);
 const rotation=camera.quaternion.clone();
 close(draw(.625,2.5,.03),.03);   // quarter of a walking step
 close(draw(1.25,2.5,.03),-.03);  // the opposite point in the same step
 close(draw(.5,0,.01,.5),-.01);   // in the air, the last step fades in place
 close(draw(.5,0,0,1),0);
 close(draw(0,0,0),0);
 assert.ok(camera.quaternion.angleTo(rotation)<1e-6);
 const fence=draw(10000,2.5,.03);
 close(draw(5,2.5,.03),fence);
});
