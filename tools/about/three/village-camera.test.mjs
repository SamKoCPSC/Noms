import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import * as THREE from 'three';
import {createWalkingTurn,frameCamera,routePose} from './village-camera.js';
const layout=JSON.parse(readFileSync('assets/about/village/layout.json'));
for(const [width,height,slot] of [[1440,900,[528,116,892,736]],[393,852,[8,448.56,377,355.44]],[360,640,[8,346.8,344,245.2]]]){
 test(`all 12 locations fit their overlay-free space at ${width}×${height}`,()=>{
  for(let index=0;index<12;index++){
   const camera=new THREE.PerspectiveCamera(60,1,.08,650);
   frameCamera(camera,layout,index,width,height,slot);
   assert.deepEqual(camera.position.toArray(),layout.stops[index].pose.eye);
   assert.equal(camera.aspect,width/height);
   assert.equal(camera.view.width,width);assert.equal(camera.view.height,height);
   for(const vertex of layout.stops[index].framing.extrema){
    const p=new THREE.Vector3(...vertex).project(camera),px=(p.x+1)*width/2,py=(1-p.y)*height/2;
    assert.ok(px>=slot[0]&&px<=slot[0]+slot[2]&&py>=slot[1]&&py<=slot[1]+slot[3],`${index}: ${px},${py}`);
   }
  }
 });
}
test('walking camera uses every verified route sample and remains continuous at stop boundaries',()=>{
 for(let i=0;i<11;i++)for(let j=0;j<=200;j++){
  const pose=routePose(layout,i+j/200),expected=layout.routes[i][j];
  for(const key of ['eye','target'])for(let k=0;k<3;k++)assert.ok(Math.abs(pose[key][k]-expected[key][k])<.001,`${i}/${j}/${key}`);
 }
});

test('app catalog embeds usable lighting and matching world-space stop metadata',()=>{
 assert.equal(typeof layout.lighting.exposure,'number');
 assert.equal(layout.lighting.sky_color.length,3);
 assert.equal(layout.lighting.sun_direction_y_up.length,3);
 assert.equal(layout.routes.length,layout.stops.length-1);
 for(const stop of layout.stops)assert.equal(stop.pose.id,stop.id);
});

test('all stops look down from higher viewpoints without changing the ground route',()=>{
 const base='assets/models/terraced-village/source/';
 const poses=JSON.parse(readFileSync(base+'camera-poses.json'));
 const routes=JSON.parse(readFileSync(base+'walking-camera.json'));
 for(let i=0;i<12;i++){
  const {eye,target}=layout.stops[i].pose;
  assert.ok(eye[1]>=poses[i].eye[1]+2.5);
  const angle=Math.atan2(eye[1]-target[1],Math.hypot(eye[0]-target[0],eye[2]-target[2]))*180/Math.PI;
  assert.ok(angle>=32-1e-8,`${i}: ${angle}`);
 }
 for(let i=0;i<11;i++)for(let j=0;j<routes[i].length;j++){
  const original=routes[i][j],raised=layout.routes[i][j];
  assert.equal(raised.eye[0],original.eye[0]);assert.equal(raised.eye[2],original.eye[2]);
  assert.deepEqual(raised.target,original.target);assert.ok(raised.eye[1]>=original.eye[1]+2.5-1e-8);
 }
});

test('travel looks along the level path in either direction', () => {
 const camera = new THREE.PerspectiveCamera();
 for (let leg = 0; leg < layout.routes.length; leg++) {
  for (const fraction of [0.36, 0.5, 0.64]) {
   const progress = leg + fraction;
   const before = routePose(layout, progress - 0.01).eye;
   const after = routePose(layout, progress + 0.01).eye;
   const tangent = new THREE.Vector3(after[0]-before[0], 0, after[2]-before[2]).normalize();
   for (const reverse of [0, 1]) {
    frameCamera(camera, layout, progress, 393, 852, [8,450,377,350], reverse);
    const view = camera.getWorldDirection(new THREE.Vector3());
    assert.ok(Math.abs(view.y) < 1e-6);
    assert.ok(view.dot(tangent) * (reverse ? -1 : 1) > 0.95);
    assert.ok(Math.abs(camera.projectionMatrix.elements[8]) < 1e-6);
    assert.ok(Math.abs(camera.projectionMatrix.elements[9]) < 1e-6);
   }
  }
 }
});

test('eased walks turn less than four degrees per frame at 60 Hz on every leg', () => {
 const camera = new THREE.PerspectiveCamera();
 const previous = new THREE.Quaternion();
 const frames = 6.4 * 60;
 for (let leg = 0; leg < layout.routes.length; leg++) {
  for (const backwards of [false, true]) {
   for (let frame = 0; frame <= frames; frame++) {
    const t = THREE.MathUtils.smootherstep(frame / frames, 0, 1);
    frameCamera(camera, layout, leg + (backwards ? 1-t : t), 393, 852, [8,450,377,350], backwards);
    if (frame) assert.ok(previous.angleTo(camera.quaternion) < THREE.MathUtils.degToRad(4));
    previous.copy(camera.quaternion);
   }
  }
 }
});

test('scroll reversals ease from the visible bearing and hold that bearing when paused', () => {
 const camera = new THREE.PerspectiveCamera();
 const turn = createWalkingTurn();
 function bearing(p) {
  const {backwards, offset} = turn(layout, p);
  frameCamera(camera, layout, p, 393, 852, [8,450,377,350], backwards, 0, [0,0,0], offset);
  return camera.quaternion.clone();
 }
 for (const p of [0.03, 0.41, 0.55, 0.97]) {
  bearing(p - 0.01);
  const forward = bearing(p);
  assert.ok(forward.angleTo(bearing(p - 0.0001)) < 0.005);
  const paused = bearing(p - 0.02);
  assert.ok(paused.angleTo(bearing(p - 0.02)) < 1e-6);
  assert.ok(paused.angleTo(bearing(p - 0.0199)) < 0.005);
 }
});

test('look around rotates in place, keeps the horizon upright, and limits vertical tilt', () => {
 const camera = new THREE.PerspectiveCamera();
 const slot = [8, 450, 377, 350];
 const progress = 2.5;
 frameCamera(camera, layout, progress, 393, 852, slot);
 const eye = camera.position.clone();
 const heading = camera.getWorldDirection(new THREE.Vector3());
 for (const tilt of [-1, 0, 1]) {
  frameCamera(camera, layout, progress, 393, 852, slot, 0, 0, [Math.PI, tilt, 1]);
  const view = camera.getWorldDirection(new THREE.Vector3());
  assert.ok(camera.position.equals(eye));
  assert.ok(Math.abs(view.y - Math.sin(THREE.MathUtils.degToRad(80) * tilt)) < 1e-6);
  view.y = 0;
  assert.ok(view.normalize().dot(heading) < -0.999);
  assert.ok(Math.abs(camera.rotation.z) < 1e-6);
 }
});
