import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import {createCollisionMap,createWalkBody} from './village-collision.js';
import {createWalkSurface} from './village-walk.js';

const read = name => JSON.parse(readFileSync(`assets/about/village/${name}.json`));
const layout = read('layout');
const village = createCollisionMap(read('collision'));
const surface = createWalkSurface(layout);

function flatRoom(spans) {
 const wall = createCollisionMap({
  minX: 0, minZ: 0, width: 32, depth: 24, cell: .25, unit: .125,
  // A single wall or shelf between x=3 and x=3.25, with a doorway from z=3 to 4.5.
  profiles: [[],spans],
  rows: Array.from({length:24}, (_,z) => z>=12 && z<18 ? [0,0] : [0,0,12,1,13,0]),
 });
 return createWalkBody({sample:()=>({floor:0,eyeHeight:1.65}),constrain:(x,z)=>[x,z]},wall);
}

test('swept body stops at thin walls, slides alongside them and fits through doors',()=>{
 const room=flatRoom([0,40]), body=room.spawn(1,1);
 room.move(body,6,1,0,0);
 assert.ok(body.x>2.5 && body.x<=2.7);
 room.move(body,6,2,1,0); // pushing diagonally into the wall at jump apex
 assert.ok(body.x<=2.7 && Math.abs(body.z-2)<1e-6);
 room.move(body,body.x,3.75,0,0);
 room.move(body,6,3.75,0,0);
 assert.ok(Math.abs(body.x-6)<1e-6);
 const displaced=room.spawn(3.1,1);
 assert.ok(displaced.x<=2.7 || displaced.x>=3.55);
});

test('crouching fits under shelves and standing or jumping cannot push the head through',()=>{
 const room=flatRoom([11,13]), body=room.spawn(1,1);
 room.move(body,3.1,1,0,0);
 assert.ok(body.x<=2.7);
 room.move(body,3.1,1,0,.6);
 assert.ok(Math.abs(body.x-3.1)<1e-6);
 room.move(body,body.x,1,1,.6);
 assert.ok(Math.abs(body.feet+body.height-1.375)<1e-6);
 room.move(body,body.x,1,0,0);
 assert.equal(body.height,1.375);
 room.move(body,5,1,0,0);
 assert.equal(body.height,1.79);
});

test('a jump can clear a low obstacle, land on top, and step back down',()=>{
 const room=flatRoom([0,6]), body=room.spawn(1,1);
 room.move(body,3.1,1,0,0);
 assert.ok(body.x<=2.7);
 room.move(body,3.1,1,1,0);
 assert.ok(Math.abs(body.x-3.1)<1e-6);
 room.move(body,body.x,1,0,0);
 assert.equal(body.feet,.75);
 room.move(body,5,1,0,0);
 assert.equal(body.feet,0);
});

test('the shipped collision map keeps all twelve entrance approaches open',()=>{
 for(const {end,threshold,width} of layout.walk.entrances){
  const walk=createWalkBody(surface,village),body=walk.spawn(end[0],end[2]);
  assert.ok(Math.hypot(body.x-end[0],body.z-end[2])<width/2);
  walk.move(body,threshold[0],threshold[2],0,0);
  assert.ok(Math.hypot(body.x-threshold[0],body.z-threshold[2])<1e-5);
 }
});

test('the shipped kitchen furniture and cottage exterior block forward travel',()=>{
 for(const [x,z,tx,tz] of [[-30,45,-37.5,30],[-62,45,-62,33]]){
  const walk=createWalkBody(surface,village), body=walk.spawn(x,z);
  walk.move(body,tx,tz,0,0);
  assert.ok(Math.hypot(body.x-x,body.z-z)>1);
  assert.ok(Math.hypot(body.x-tx,body.z-tz)>2);
 }
});

test('the meadow grass east of the kitchen is walkable at terrain height',()=>{
 const walk=createWalkBody(surface,village),z=40;
 const body=walk.spawn(-17,z);
 for(let x=-16.75;x<=-11;x+=.25){
  walk.move(body,x,z,0,0);
  assert.ok(Math.abs(body.x-x)<1e-6 && Math.abs(body.z-z)<1e-6);
  assert.ok(Math.abs(body.feet-surface.sample(x,z).floor)<1e-6);
 }
});

test('the maple near the brick kitchen has walkable ground under its crown and a solid trunk',()=>{
 const walk=createWalkBody(surface,village);
 const x=-9, trunkZ=37.24;
 const body=walk.spawn(x,trunkZ+2);
 walk.move(body,x,trunkZ+1,0,0);
 assert.ok(Math.abs(body.z-(trunkZ+1))<1e-6);
 walk.move(body,x,trunkZ,0,0);
 assert.ok(body.z>trunkZ+.25 && body.z<trunkZ+1);
});

test('every wayside cottage has a walkable approach from the paved tour route to its closed front door', () => {
  const plots = JSON.parse(readFileSync('assets/models/terraced-village/source/wayside-cottages.json'));
  const route = layout.routes.flat().map(pose => pose.eye);
  for (const {id, center, join, bend, depth} of plots) {
    assert.ok(route.some(([x, , z]) => Math.hypot(x - join[0], z - join[1]) < .65), id + ' joins the tour');
    const dx = bend[0] - center[0], dz = bend[1] - center[1];
    // Leave room for the player's body between the end of the walk and the sill.
    const distance = (depth / 2 + 1.1) / Math.hypot(dx, dz);
    const end = [center[0] + dx * distance, center[1] + dz * distance];
    const walk = createWalkBody(surface, village);
    const body = walk.spawn(...join);

    function reach(t) {
      const [x, z] = join.map((start, axis) =>
        (1 - t) ** 2 * start + 2 * t * (1 - t) * bend[axis] + t ** 2 * end[axis]);
      walk.move(body, x, z, 0, 0);
      assert.ok(Math.hypot(body.x - x, body.z - z) < .04, id + ' walk blocked at ' + t);
      if (t === .5) {
        const y = surface.sample(x, z).floor;
        const stone = village.gap(x, z, y - 1, y + .3, .14);
        assert.ok(stone && stone.bottom >= y - .05, id + ' has a stone surface underfoot');
      }
    }

    for (let i = 0; i <= 48; i++) reach(i / 48);
    walk.move(body, ...center, 0, 0);
    const doorDistance = Math.hypot(body.x - center[0], body.z - center[1]) - depth / 2;
    assert.ok(doorDistance >= 0 && doorDistance < 1.2, id + ' stops the player at the doorstep');
    for (let i = 48; i >= 0; i--) reach(i / 48);
  }
});
