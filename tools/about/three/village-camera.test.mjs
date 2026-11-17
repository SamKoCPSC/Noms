import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import * as THREE from 'three';
import {frameCamera,routePose} from './village-camera.js';
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
