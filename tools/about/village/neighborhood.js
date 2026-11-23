import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { FontLoader } from 'three/addons/loaders/FontLoader.js';
import { TextGeometry } from 'three/addons/geometries/TextGeometry.js';
import fontData from 'three/examples/fonts/droid/droid_sans_bold.typeface.json';
import { groundHeight } from './walking-world.js';
import { softBox, softRoof } from './world-style.js';

// Droid Sans: Copyright 2005–2008 The Android Open Source Project, Apache 2.0.
// Full notice accompanies the downloadable preview and is embedded in its HTML.
export function addNeighborhood({ layout, stops, walking, terrain, mat, riverCurve }) {
  const font = new FontLoader().parse(fontData);
  const group = new THREE.Group(); group.name = 'Noms village neighborhood';
  const occupied = [], reservations = [], solids = [], signs = [], homes = [], mountChecks = [];
  terrain.updateMatrixWorld(true);
  // Existing individual props are still separate here (landscape is added later).
  terrain.traverse(o => {
    if (o.isMesh && o.geometry.type !== 'PlaneGeometry' && !['Ground-level walking path','River surface','River banks','Distant hillside'].includes(o.name)) occupied.push(new THREE.Box3().setFromObject(o));
  });
  terrain.add(group);
  const paths = walking.routes.flatMap(r => r.points.filter((_, i) => i % 2 === 0));
  const river = riverCurve.getPoints(300);
  const overlaps = (p, radius, b) => p.x + radius > b.min.x && p.x - radius < b.max.x && p.z + radius > b.min.z && p.z - radius < b.max.z;
  function sightDistance(p, pose) {
    const a = new THREE.Vector2(pose.eye.x, pose.eye.z), b = new THREE.Vector2(pose.target.x, pose.target.z), v = b.clone().sub(a);
    const t = THREE.MathUtils.clamp(new THREE.Vector2(p.x, p.z).sub(a).dot(v) / v.lengthSq(), 0, 1);
    return a.addScaledVector(v, t).distanceTo(new THREE.Vector2(p.x, p.z));
  }
  function clear(p, radius, pathMargin = 2.1) {
    return stops.every(s => !overlaps(p, radius + 1, s.box))
      && occupied.every(b => !overlaps(p, radius + .25, b))
      && reservations.every(b => !overlaps(p, radius + .4, b))
      && paths.every(q => Math.hypot(q.x - p.x, q.z - p.z) > radius + pathMargin)
      && river.every(q => Math.hypot(q.x - p.x, q.z - p.z) > radius + 3.5)
      && walking.poses.every(pose => sightDistance(p, pose) > radius + 1.5);
  }
  function mesh(parent, name, geometry, color, p, rotation = [0,0,0]) {
    const o = new THREE.Mesh(geometry, mat(color)); o.name = name;o.position.set(...p);o.rotation.set(...rotation);parent.add(o);solids.push(o);return o;
  }
  const box = (parent,name,p,size,color,rotation) => mesh(parent,name,softBox(size),color,p,rotation);
  function localBounds(object) {
    object.updateMatrix();
    return new THREE.Box3().setFromBufferAttribute(object.geometry.attributes.position).applyMatrix4(object.matrix);
  }
  function text(parent, words, p, width, size = .43, color = '#fff0ca', back = false) {
    const g = new TextGeometry(words,{font,size,depth:.012,curveSegments:3,bevelEnabled:false});g.computeBoundingBox();
    const b=g.boundingBox,scale=Math.min(1,width/(b.max.x-b.min.x));g.translate(-(b.min.x+b.max.x)/2,-(b.min.y+b.max.y)/2,0);g.scale(scale,scale,1);
    mesh(parent,`Lettering: ${words}`,g,color,p,[0,back?Math.PI:0,0]);
  }
  function plaque(parent, words, y, width, color = '#345d43', size = .43, textWidth = width-.28) {
    box(parent,`Street plaque: ${words}`,[0,y,0],[width+.12,.95,.18],'#d5bd83');
    box(parent,'Painted sign inset',[0,y,.105],[width,.81,.04],color);
    box(parent,'Painted sign reverse',[0,y,-.105],[width,.81,.04],color);
    text(parent,words,[0,y,.135],textWidth,size);text(parent,words,[0,y,-.135],textWidth,size,'#fff0ca',true);
  }
  function reserve(parent) {
    parent.updateMatrixWorld(true);const b=new THREE.Box3().setFromObject(parent);reservations.push(b);return b;
  }
  function groundGroup(name,p,yaw) {
    const g=new THREE.Group();g.name=name;g.position.set(p.x,groundHeight(layout,p.x,p.z),p.z);g.rotation.y=yaw;group.add(g);return g;
  }
  function roadside(routeIndex, radius, preferred = .35) {
    const r=walking.routes[routeIndex];
    for (const offset of [radius+2.8,radius+4,radius+6,radius+8,radius+12,radius+16]) for (const t of [preferred,.04,.1,.16,.22,.28,.4,.48,.56,.64,.72,.8,.88,.96]) for (const sign of [1,-1]) {
      const at=r.curve.getPointAt(t),d=r.curve.getTangentAt(t),p=at.clone().addScaledVector(new THREE.Vector3(-d.z,0,d.x).normalize(),sign*offset);
      if (clear(p,radius)) return {p,at,t,yaw:Math.atan2(at.x-p.x,at.z-p.z)};
    }
    return null;
  }
  // One named street for every connecting route. Place these before homes so
  // every segment gets a readable sign, with geometry on both sides.
  for(let i=0;i<walking.routes.length;i++) {
    const name=layout.neighborhood.streets[i],spot=roadside(i,1.95);
    if(!spot)throw Error(`No clear street-sign location: ${name}`);
    const g=groundGroup(`Street sign ${i+1}: ${name}`,spot.p,spot.yaw);
    // Flatten the support inside the solid board core; both painted faces stay clear.
    const post=mesh(g,'Street sign timber post',new THREE.CylinderGeometry(.13,.16,4.96,8),'#65503b',[0,2.48,0]);post.scale.z=.25;
    plaque(g,name,4.05,3.6,'#345d43',.32);
    const next=stops[i+1].title;
    plaque(g,next,2.95,3.6,'#7f4934',.24,2.35);
    const badge=box(g,'Noms village sign badge',[0,4.83,0],[2.5,.42,.14],'#dfc894');
    const postBounds=localBounds(post),badgeBounds=localBounds(badge);
    mountChecks.push({kind:'street',face_clearance:Math.min(badgeBounds.max.z-postBounds.max.z,postBounds.min.z-badgeBounds.min.z),top_clearance:badgeBounds.max.y-postBounds.max.y});
    text(g,'NOMS VILLAGE',[0,4.83,.09],2.3,.22,'#39513a');
    text(g,'NOMS VILLAGE',[0,4.83,-.09],2.3,.22,'#39513a',true);
    // Arrow direction follows the route toward its next stop in sign-local space.
    const d=walking.routes[i].curve.getTangentAt(spot.t).applyAxisAngle(new THREE.Vector3(0,1,0),-spot.yaw);
    const direction=d.x>=0?1:-1;
    const arrow=new THREE.Shape();arrow.moveTo(-.22,-.15);arrow.lineTo(.04,-.15);arrow.lineTo(.04,-.3);arrow.lineTo(.38,0);arrow.lineTo(.04,.3);arrow.lineTo(.04,.15);arrow.lineTo(-.22,.15);arrow.closePath();
    const ag=new THREE.ExtrudeGeometry(arrow,{depth:.02,bevelEnabled:false});
    mesh(g,'Direction arrow',ag,'#f9e8bf',[direction*1.48,2.95,.14],[0,0,direction===1?0:Math.PI]);
    mesh(g,'Direction arrow reverse',ag.clone(),'#f9e8bf',[direction*1.48,2.95,-.14],[0,Math.PI,direction===1?Math.PI:0]);
    reserve(g);signs.push({street:name,destination:next,position:g.position.toArray(),yaw:spot.yaw,route:i});
  }
  function roof(parent,w,d,base,rise,color) {
    mesh(parent,'Pitched cottage roof',softRoof(w,d,rise),color,[0,base,0]);
  }
  function window(parent,x,y,z,side=false) {
    const g=new THREE.Group();g.position.set(x,y,z);if(side)g.rotation.y=Math.PI/2;parent.add(g);
    box(g,'Cream window casing',[0,0,0],[1.5,1.65,.16],'#e5d4aa');box(g,'Blue window pane',[0,0,.1],[1.26,1.4,.06],'#5a95a0');
    box(g,'Window mullion',[0,0,.15],[.065,1.4,.06],'#e5d4aa');box(g,'Window crossbar',[0,0,.15],[1.26,.065,.06],'#e5d4aa');box(g,'Deep window sill',[0,-.86,.15],[1.72,.15,.42],'#dbbd86');
  }
  const colors=[['#c9845d','#714d3c','#56806a'],['#87a598','#475e61','#b77e4d'],['#d4b66f','#a45c43','#5c7a55'],['#b17d71','#5c6658','#6f8e8e'],['#8d9fae','#705242','#9b583e'],['#b5b68d','#596d66','#ad6943'],['#d3a082','#596d8c','#5e8058'],['#9cb29a','#9f684e','#756043']];
  for(let i=0;i<layout.neighborhood.houses.length;i++) {
    const specification=layout.neighborhood.houses[i],w=7+(i%3)*.65,d=6.5+(i%2)*.7,radius=Math.hypot(w/2+1,d/2+3.3)*layout.neighborhood.house_scale,p=new THREE.Vector3();let found=false;
    for(const radiusOffset of [0,6,12,18,24,30]) {
      for(let j=0;j<(radiusOffset?12:1);j++) {
        p.set(specification[0]+Math.cos(j*Math.PI/6)*radiusOffset,0,specification[1]+Math.sin(j*Math.PI/6)*radiusOffset);
        if(clear(p,radius)){found=true;break;}
      }
      if(found)break;
    }
    if(!found)throw Error(`No clear cottage location ${i+1}`);
    const nearest=paths.reduce((a,b)=>Math.hypot(a.x-p.x,a.z-p.z)<Math.hypot(b.x-p.x,b.z-p.z)?a:b);
    const g=groundGroup(`Village cottage ${i+1}`,p,Math.atan2(nearest.x-p.x,nearest.z-p.z)),[wall,roofColor,door]=colors[i%colors.length],h=4.4+(i%2)*.8;
    g.scale.setScalar(layout.neighborhood.house_scale);
    box(g,'Cottage foundation',[0,.15,0],[w+.5,.3,d+.5],'#a69a7d');box(g,'Colored cottage walls',[0,h/2+.3,0],[w,h,d],wall);
    for(let j=0;j<6;j++)box(g,'Horizontal siding seam',[0,.65+j*h/6,d/2+.015],[w,.045,.045],'#c9bb98');
    roof(g,w+.7,d+.7,h+.28,2.1,roofColor);
    box(g,'Chimney',[w*.28,h+1.45,-d*.2],[.72,2.5,.8],'#995c43');box(g,'Chimney cap',[w*.28,h+2.7,-d*.2],[.95,.16,1.02],'#d0bc98');
    box(g,'Door casing',[0,1.72,d/2+.1],[1.6,2.85,.2],'#ebd8b2');box(g,'Painted front door',[0,1.64,d/2+.23],[1.3,2.54,.12],door);
    mesh(g,'Brass door knob',new THREE.SphereGeometry(.085,8,6),'#cf9e48',[.43,1.55,d/2+.34]);
    for(const x of [-w*.32,w*.32])window(g,x,2.7,d/2+.11);
    for(const z of [-d*.25,d*.25])window(g,w/2+.08,2.7,z,true);
    box(g,'Front porch deck',[0,.34,d/2+1.05],[w*.75,.25,1.9],'#b38b58');
    for(let j=0;j<2;j++)box(g,'Cottage front step',[0,.12+j*.1,d/2+2.35-j*.4],[2.4,.2,.45],'#b8a27e');
    for(const x of [-w*.32,w*.32])box(g,'Porch upright',[x,1.83,d/2+1.7],[.14,3,.14],'#ebd8b2');
    box(g,'Porch awning',[0,3.35,d/2+1.1],[w*.78,.16,2.15],roofColor,[-.12,0,0]);
    for(const x of [-w*.34,w*.34]) {
      box(g,'Window flower box',[x,1.72,d/2+.4],[1.7,.32,.5],door);
      for(let j=0;j<4;j++)mesh(g,'Window herb leaves',new THREE.IcosahedronGeometry(.24,0),'#50804a',[x-.6+j*.4,1.98,d/2+.4]);
    }
    box(g,'Mailbox post',[-w/2-.5,.75,d/2+1.4],[.16,1.5,.16],'#725137');box(g,'Cottage mailbox',[-w/2-.5,1.55,d/2+1.4],[.65,.48,.75],door);
    box(g,'Mailbox slot',[-w/2-.5,1.61,d/2+1.79],[.4,.05,.03],'#3b4438');text(g,String(i+1),[-w/2-.5,1.43,d/2+1.8],.4,.2);
    const b=reserve(g);homes.push({number:i+1,position:g.position.toArray(),bounds:{min:b.min.toArray(),max:b.max.toArray()}});
  }
  // A few friendly pedestrian road signs and lanterns, rather than urban traffic.
  let roadSigns=0,lanterns=0;
  for(const [i,words] of [[0,['SLOW','GOOD FOOD AHEAD']],[3,['SHARE','THE PATH']],[7,['MARKET','CROSSING']],[9,['TAKE TIME','TO GATHER']]]) {
    const spot=roadside(i,2.3,.68);if(!spot)continue;
    const g=groundGroup(`Village road sign ${words.join(' ')}`,spot.p,spot.yaw);
    const post=mesh(g,'Road sign post',new THREE.CylinderGeometry(.11,.14,3.8,7),'#61523c',[0,1.9,-.27]);
    const rim=box(g,'Road sign ochre rim',[0,3.25,0],[3.7,1.65,.17],'#c69a42');box(g,'Road sign cream face',[0,3.25,.1],[3.49,1.43,.05],'#e7d5a0');
    mountChecks.push({kind:'road',rear_clearance:localBounds(rim).min.z-localBounds(post).max.z});
    text(g,words[0],[0,3.55,.15],3.2,.38,'#3b5540');text(g,words[1],[0,2.96,.15],3.2,.27,'#3b5540');reserve(g);roadSigns++;
  }
  for(let i=0;i<11;i++) {
    const spot=roadside(i,.75,.61);if(!spot)continue;
    const g=groundGroup('Village lantern post',spot.p,0);
    mesh(g,'Lantern fluted post',new THREE.CylinderGeometry(.09,.17,4.7,8),'#40544a',[0,2.35,0]);
    box(g,'Lantern warm glass',[0,4.8,0],[.55,.8,.55],'#d8b66e');box(g,'Lantern base',[0,4.36,0],[.75,.12,.75],'#40544a');
    for(const x of [-.3,.3])for(const z of [-.3,.3])box(g,'Lantern frame',[x,4.8,z],[.06,.8,.06],'#40544a');
    mesh(g,'Lantern roof',new THREE.ConeGeometry(.6,.42,4),'#40544a',[0,5.4,0],[0,Math.PI/4,0]);reserve(g);lanterns++;
  }
  const welcome=roadside(0,4,.08);
  if(welcome) {
    const g=groundGroup('Noms village welcome board',welcome.p,welcome.yaw);
    const posts=[-3.2,3.2].map(x=>box(g,'Welcome board posts',[x,2,-.24],[.22,4,.22],'#71543b'));
    const rim=box(g,'Welcome board frame',[0,3.15,0],[7.1,2,.2],'#b5935d');box(g,'Welcome board green face',[0,3.15,.13],[6.85,1.75,.08],'#395b43');
    mountChecks.push({kind:'welcome',rear_clearance:localBounds(rim).min.z-Math.max(...posts.map(p=>localBounds(p).max.z))});
    text(g,'NOMS VILLAGE',[0,3.55,.19],6.2,.57);text(g,'GOOD FOOD. GOOD COMPANY.',[0,2.75,.19],6.1,.25);reserve(g);
  }
  group.updateMatrixWorld(true);
  const bounds=solids.map(o=>new THREE.Box3().setFromObject(o).expandByScalar(.2));
  // Keep checks against individual parts, but share draw calls for repeated
  // window frames, porch details, signposts and lettering on both sign faces.
  const batches=new Map();
  group.traverse(o=>{
    if(!o.isMesh)return;
    const key=o.name+o.material.color.getHexString();
    if(!batches.has(key))batches.set(key,{name:o.name,material:o.material,geometry:[]});
    batches.get(key).geometry.push(o.geometry.clone().applyMatrix4(o.matrixWorld));
    o.geometry.dispose();
  });
  group.clear();
  for(const batch of batches.values()) {
    const o=new THREE.Mesh(mergeGeometries(batch.geometry),batch.material);o.name=batch.name;group.add(o);batch.geometry.forEach(g=>g.dispose());
  }
  return { reservations, check:()=>{
    const collisions=[];for(let i=0;i<walking.routes.length;i++)for(let j=0;j<=200;j++)if(bounds.some(b=>b.containsPoint(walking.viewAt(i+j/200).eye)))collisions.push(i+j/200);
    return {houses:homes,streets:signs,road_signs:roadSigns,lanterns,welcome_board:!!welcome,camera_collisions:collisions,lettering_is_geometry:true,sign_mounts:mountChecks,detail_parts:solids.length,render_meshes:batches.size};
  }};
}
