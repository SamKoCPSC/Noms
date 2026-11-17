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

// Off-axis perspective moves the composition, not the viewport. The village
// continues behind the text overlay, without scissoring or stretched proportions.
export function frameCamera(camera,layout,progress,width,height,slot) {
 const p=THREE.MathUtils.clamp(progress,0,layout.stops.length-1),i=Math.floor(p),j=Math.min(i+1,layout.stops.length-1),t=p-i;
 const a=stopFraming(layout.stops[i],width,height,slot),b=stopFraming(layout.stops[j],width,height,slot);
 const focal=THREE.MathUtils.lerp(a.focal,b.focal,t),x=THREE.MathUtils.lerp(a.x,b.x,t),y=THREE.MathUtils.lerp(a.y,b.y,t);
 const pose=routePose(layout,p);camera.position.fromArray(pose.eye);camera.lookAt(new THREE.Vector3(...pose.target));
 camera.aspect=width/height;camera.fov=THREE.MathUtils.radToDeg(2*Math.atan(height/(2*focal)));
 camera.setViewOffset(width,height,width/2-x,height/2-y,width,height);camera.updateMatrixWorld(true);
 return {pose,focal,principal:[x,y]};
}
