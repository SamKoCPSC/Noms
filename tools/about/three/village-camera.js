import * as THREE from 'three';

// Evaluate the approved, collision-checked walking path; never move the models.
export function routePose(layout, progress) {
 const p=THREE.MathUtils.clamp(progress,0,layout.stops.length-1),index=Math.floor(p);
 if(index===layout.stops.length-1)return layout.stops[index].pose;
 const samples=layout.routes[index],at=(p-index)*(samples.length-1),a=samples[Math.floor(at)],b=samples[Math.min(samples.length-1,Math.floor(at)+1)],t=at%1;
 return {eye:a.eye.map((v,i)=>THREE.MathUtils.lerp(v,b.eye[i],t)),target:a.target.map((v,i)=>THREE.MathUtils.lerp(v,b.target[i],t))};
}

export function stopFraming(stop,width,height,slot) {
 const {min,max}=stop.framing;
 const focal=Math.min(slot[2]*.9/(max[0]-min[0]),slot[3]*.9/(max[1]-min[1]));
 return {focal,x:slot[0]+slot[2]/2-focal*(min[0]+max[0])/2,y:slot[1]+slot[3]/2+focal*(min[1]+max[1])/2};
}

// Sample a short stretch of path so the heading turns with the bends.
function pathHeading(layout, index, fraction) {
 const before = routePose(layout, index + Math.max(0, fraction - 0.07)).eye;
 const after = routePose(layout, index + Math.min(1, fraction + 0.07)).eye;
 return Math.atan2(after[0] - before[0], after[2] - before[2]);
}

const turnSpan = 0.36;
const shortestAngle = angle => Math.atan2(Math.sin(angle), Math.cos(angle));

function roomTurn(layout, leg, t, backwards) {
 const endpoint = t < 0.5 ? 0 : 1;
 const room = layout.stops[leg + endpoint].pose;
 const [dx, dy, dz] = room.target.map((value, axis) => value - room.eye[axis]);
 const travel = THREE.MathUtils.smootherstep(Math.min(t, 1-t), 0, turnSpan);
 // Aim for the visible stretch of path ahead. The camera can turn directly
 // from the room to that heading even when the first steps curve sharply.
 const along = THREE.MathUtils.clamp(t, turnSpan, 1-turnSpan);
 const pathYaw = pathHeading(layout, leg, along) + (backwards ? Math.PI : 0);
 return {
  yaw: pathYaw + (1-travel) * shortestAngle(Math.atan2(dx, dz) - pathYaw),
  pitch: (1-travel) * Math.atan2(dy, Math.hypot(dx, dz)),
  travel
 };
}

// Carry the current bearing through a mid-walk reversal, then ease into the
// new travel direction with scroll distance. Holding still holds the view.
export function createWalkingTurn() {
 let previous = null, backwards = false, offset = 0;
 let start = 0, span = 0, initialOffset = 0;
 return (layout, progress) => {
  if (previous !== null && progress !== previous) {
   const reverse = progress < previous;
   if (reverse !== backwards && !Number.isInteger(previous)) {
    const leg = Math.floor(previous), t = previous - leg;
    const from = roomTurn(layout, leg, t, backwards).yaw + offset;
    const to = roomTurn(layout, leg, t, reverse).yaw;
    initialOffset = shortestAngle(from - to);
    start = previous;
    span = Math.min(turnSpan, reverse ? t : 1-t);
   } else if (Number.isInteger(previous)) {
    span = 0;
   }
   backwards = reverse;
   offset = span > 0 ? initialOffset * (1-THREE.MathUtils.smootherstep(Math.abs(progress-start), 0, span)) : 0;
  }
  previous = progress;
  return {backwards, offset};
 };
}

// Off-axis perspective moves the composition, not the viewport. The village
// continues behind the text overlay, without scissoring or stretched proportions.
export function frameCamera(camera,layout,progress,width,height,slot,backwards=false,openingDistance=0,look=[0,0,0],yawOffset=0) {
 const p=THREE.MathUtils.clamp(progress,0,layout.stops.length-1),i=Math.floor(p),j=Math.min(i+1,layout.stops.length-1),t=p-i;
 const a=stopFraming(layout.stops[i],width,height,slot),b=stopFraming(layout.stops[j],width,height,slot);
 const focal=THREE.MathUtils.lerp(a.focal,b.focal,t),x=THREE.MathUtils.lerp(a.x,b.x,t),y=THREE.MathUtils.lerp(a.y,b.y,t);
 const pose = routePose(layout, p);
 camera.position.fromArray(pose.eye);
 let travel = 0;
 if (t === 0) {
  const target = new THREE.Vector3(...pose.target);
  if (p === 0 && openingDistance > 0) {
   // Dolly along the kitchen sightline to keep it in the model area on
   // both layouts while the surrounding village moves in perspective.
   camera.position.sub(target).multiplyScalar(1 + 1.6 * openingDistance).add(target);
  }
  camera.lookAt(target);
 } else {
  const aim = roomTurn(layout, i, t, backwards);
  travel = aim.travel;
  const heading = aim.yaw + yawOffset, pitch = aim.pitch;
  camera.lookAt(camera.position.clone().add(new THREE.Vector3(
   Math.sin(heading) * Math.cos(pitch), Math.sin(pitch), Math.cos(heading) * Math.cos(pitch)
  )));
 }
 camera.aspect=width/height;camera.fov=THREE.MathUtils.radToDeg(2*Math.atan(height/(2*focal)));
 const [yaw, tilt, lookWeight] = look;
 if (lookWeight > 0) {
  camera.rotation.reorder('YXZ');
  camera.rotation.y += yaw;
  const limit = THREE.MathUtils.degToRad(80);
  camera.rotation.x = THREE.MathUtils.lerp(camera.rotation.x, Math.sign(tilt)*limit, Math.abs(tilt));
 }
 const centered = travel + (1-travel)*lookWeight;
 camera.setViewOffset(width,height,(width/2-x)*(1-centered),(height/2-y)*(1-centered),width,height);camera.updateMatrixWorld(true);
 return {pose,focal,principal:[THREE.MathUtils.lerp(x,width/2,centered),THREE.MathUtils.lerp(y,height/2,centered)]};
}
