// Each cell lists occupied vertical intervals. Query every cell touched by the
// player's circular footprint so thin walls, glass and narrow legs stay solid.
export function createCollisionMap({minX,minZ,cell,unit,width,depth,rows,profiles}) {
 const Index=profiles.length<=65536 ? Uint16Array : Uint32Array;
 const cells=new Index(width*depth);
 for(const [z,row] of rows.entries()){
  for(let i=0;i<row.length;i+=2){
   cells.fill(row[i+1],z*width+row[i],z*width+(row[i+2]??width));
  }
 }
 function gap(x,z,bottom,level,radius){
  let top=Infinity;
  const gx=(x-minX)/cell,gz=(z-minZ)/cell,r=radius/cell;
  for(let j=Math.max(0,Math.floor(gz-r));j<=Math.min(depth-1,Math.floor(gz+r));j++){
   const dz=Math.max(j-gz,0,gz-j-1);
   for(let i=Math.max(0,Math.floor(gx-r));i<=Math.min(width-1,Math.floor(gx+r));i++){
    const dx=Math.max(i-gx,0,gx-i-1);
    if(dx*dx+dz*dz>=r*r)continue;
    const spans=profiles[cells[j*width+i]];
    for(let k=0;k<spans.length;k+=2){
     const lo=spans[k]*unit,hi=spans[k+1]*unit;
     if(lo>level){top=Math.min(top,lo);break;}
     if(hi>level)return null;
     bottom=Math.max(bottom,hi);
    }
   }
  }
  return {bottom,top};
 }
 return {gap};
}

export const bodyRadius=0.3;
export const headClearance=0.14; // includes the highest footstep bob at any room scale
const stepUp=0.32;

export function createWalkBody(surface,map,arrivals=[]){
 let jumpBase=0, lastLift=0;
 function spawn(startX,startZ){
  function tryAt(rawX,rawZ){
   const [x,z]=surface.constrain(rawX,rawZ);
   const {floor,eyeHeight}=surface.sample(x,z);
   const height=eyeHeight+headClearance;
   const gap=map.gap(x,z,floor,floor+height/2,bodyRadius);
   if(!gap || gap.bottom>floor+stepUp || gap.top-gap.bottom<height)return null;
   return {x,z,feet:gap.bottom,height};
  }
  lastLift=0;
  let body=tryAt(startX,startZ);
  // A guided camera can be above a sign or a roof when exploration starts.
  // Search outward on the ground instead of spawning inside the geometry.
  for(let ring=1;!body && ring<=32;ring++){
   for(let i=-ring;i<=ring && !body;i++){
    const d=ring*bodyRadius,offset=i*bodyRadius;
    body=tryAt(startX+offset,startZ-d) || tryAt(startX+offset,startZ+d) ||
         tryAt(startX-d,startZ+offset) || tryAt(startX+d,startZ+offset);
   }
  }
  for(const [x,,z] of arrivals){
   if(body)break;
   body=tryAt(x,z);
  }
  return body;
 }
 function move(body,targetX,targetZ,lift,drop){
  const origin=surface.sample(body.x,body.z);
  const standing=origin.eyeHeight+headClearance-drop;
  const gap=map.gap(body.x,body.z,origin.floor,body.feet+body.height/2,bodyRadius);
  if(lastLift===0)jumpBase=gap.bottom-origin.floor;
  lastLift=lift;
  // Keep the head under a low ceiling on jumping or releasing crouch.
  body.height=Math.min(standing,gap.top-gap.bottom);
  body.feet=lift>0 ? Math.max(gap.bottom,Math.min(origin.floor+jumpBase+lift,gap.top-body.height)) : gap.bottom;
  let previousFloor=origin.floor;
  function tryMove(x,z){
   const {floor,eyeHeight}=surface.sample(x,z);
   const height=eyeHeight+headClearance-drop;
   const opening=map.gap(x,z,floor,body.feet+body.height/2,bodyRadius);
   if(!opening || opening.bottom>body.feet+(lift>0 ? 0 : stepUp))return false;
   const feet=lift>0 ? Math.max(opening.bottom,body.feet+floor-previousFloor) : opening.bottom;
   // Someone already under a low shelf can back out before standing up.
   if(feet+Math.min(height,body.height)>opening.top)return false;
   body.x=x;body.z=z;body.feet=feet;body.height=Math.min(height,opening.top-feet);
   previousFloor=floor;
   return true;
  }
  const [endX,endZ]=surface.constrain(targetX,targetZ);
  const dx=endX-body.x,dz=endZ-body.z;
  // Keep even a sprint step shorter than the footprint diameter. On contact
  // keep the clear component, so diagonal input slides along a wall.
  const steps=Math.ceil(Math.hypot(dx,dz)/(bodyRadius/2));
  for(let i=0;i<steps;i++){
   const x=body.x+dx/steps,z=body.z+dz/steps;
   if(!tryMove(x,z)){
    tryMove(x,body.z);
    tryMove(body.x,z);
   }
  }
  return body;
 }
 return {spawn,move};
}
