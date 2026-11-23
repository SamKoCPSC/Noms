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

const outline=JSON.parse(readFileSync('assets/models/terraced-village/layout.json')).ground_outline;
const xs=outline.map(p=>p[0]), zs=outline.map(p=>p[1]);
const west=Math.min(...xs), east=Math.max(...xs), north=Math.min(...zs), south=Math.max(...zs);

test('all twelve stops, route samples and the open meadow are freely walkable',()=>{
 for(const route of layout.routes) for(const {eye:[x,,z]} of route) {
  assert.deepEqual(surface.constrain(x,z),[x,z]);
 }
 for(const [i,{position:[x,,z]}] of layout.walk.stops.entries()) {
  assert.deepEqual(surface.constrain(x,z),[x,z]);
  assert.ok(Math.abs(surface.floor(x,z)-layout.walk.entrances[i].threshold[1])<1e-5);
 }
 for(let x=west+1;x<east;x+=7) for(let z=north+1;z<south;z+=7) {
  assert.deepEqual(surface.constrain(x,z),[x,z]);
  assert.ok(Number.isFinite(surface.floor(x,z)));
 }
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
 const camera=new THREE.PerspectiveCamera(), walk=createFirstPersonCamera(layout);
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
 assert.ok(Math.abs(eye.y-surface.floor(eye.x,eye.z)-1.65)<1e-6);
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
