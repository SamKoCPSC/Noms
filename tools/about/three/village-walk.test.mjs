import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import * as THREE from 'three';
import {frameCamera} from './village-camera.js';
import {createFirstPersonCamera,createWalkSurface} from './village-walk.js';

const layout=JSON.parse(readFileSync('assets/about/village/layout.json'));
const surface=createWalkSurface(layout);

test('the complete first leg and both entrance approaches are connected at pedestrian height',()=>{
 for(const {eye:[x,,z]} of layout.routes[0]){
  assert.deepEqual(surface.constrain(x,z),[x,z]);
  assert.ok(Number.isFinite(surface.floor(x,z)));
 }
 for(const [i,{threshold,end,depth}] of layout.walk.entrances.entries()){
  const xy=([x,,z])=>new THREE.Vector2(x,z);
  const b=xy(end),normal=b.clone().sub(xy(threshold)).normalize();
  const branch=new THREE.QuadraticBezierCurve(xy(layout.stops[i].pose.eye),b.clone().addScaledVector(normal,2.5),b);
  for(const point of [...branch.getPoints(48),b.clone().addScaledVector(normal,-depth/2)]){
   assert.ok(point.distanceTo(new THREE.Vector2(...surface.constrain(point.x,point.y)))<1e-5);
  }
  assert.ok(Math.abs(surface.floor(b.x,b.y)-end[1])<0.15);
 }
});

test('walking diagonally against the path edge slides along it and stays outside the rooms',()=>{
 let [x,,z]=layout.routes[0][22].eye;
 const startX=x;
 for(let i=0;i<80;i++) [x,z]=surface.constrain(x-.05,z+.05);
 assert.ok(x<startX-2);
 assert.ok(z<47);
 for(const {box} of layout.stops.slice(0,2)){
  const [cx,cz]=surface.constrain((box.min[0]+box.max[0])/2,(box.min[2]+box.max[2])/2);
  assert.ok(cx<box.min[0]||cx>box.max[0]||cz<box.min[2]||cz>box.max[2]);
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
