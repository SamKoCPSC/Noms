// Rebuild camera metadata independently of the textures and room geometry.
import {readFile,writeFile} from 'node:fs/promises';
const source='assets/models/terraced-village';
const layout=JSON.parse(await readFile(`${source}/layout.json`,'utf8'));
const originalPoses=JSON.parse(await readFile(`${source}/source/camera-poses.json`,'utf8'));
const originalRoutes=JSON.parse(await readFile(`${source}/source/walking-camera.json`,'utf8'));
const entrances=JSON.parse(await readFile(`${source}/source/entrance-verification.json`,'utf8')).entrances;
const walk={
 pathWidth:layout.walk.path_width,
 stops:layout.stops.map(({position,width})=>({position,width})),
 entrances:entrances.slice(0,2).map(({threshold,approach_end},i)=>({
  threshold,end:approach_end,width:layout.stops[i].entrance.width,depth:layout.stops[i].entrance.depth
 }))
};
const elevationDegrees=32;
const offsets=originalPoses.map(({eye,target})=>Math.max(2.5,target[1]+Math.hypot(eye[0]-target[0],eye[2]-target[2])*Math.tan(elevationDegrees*Math.PI/180)-eye[1]));
const poses=originalPoses.map((pose,i)=>({...pose,eye:[pose.eye[0],pose.eye[1]+offsets[i],pose.eye[2]]}));
const routes=originalRoutes.map((samples,i)=>samples.map(({eye,target},j)=>{
 const t=j/(samples.length-1),ease=t*t*t*(t*(t*6-15)+10);
 const lift=offsets[i]+(offsets[i+1]-offsets[i])*ease;
 return {eye:[eye[0],eye[1]+lift,eye[2]],target};
}));
await writeFile('assets/about/village/layout.json',JSON.stringify({
 version:layout.version,walk,camera:{elevation_degrees:elevationDegrees},
 lighting:JSON.parse(await readFile(layout.shared_lighting.settings,'utf8')),
 stops:layout.stops.map((s,i)=>({id:s.id,title:s.title,box:poses[i].bounds,pose:poses[i]})),routes
}));
await import('./build-framing.mjs');
