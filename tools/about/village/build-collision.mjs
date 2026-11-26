// Bake vertical solid spans from the exact meshes used by Explore, including glass.
import {readFile, writeFile} from 'node:fs/promises';
import {loadGrassFaces} from './grass-collision-filter.mjs';
import {NodeIO} from '@gltf-transform/core';
import {ALL_EXTENSIONS} from '@gltf-transform/extensions';
import {MeshoptDecoder} from 'meshoptimizer';

const dir='assets/about/village';
const layout=JSON.parse(await readFile(`${dir}/layout.json`));
const cell=0.25, unit=0.125;
const outline=layout.walk.outline;
const minX=Math.min(...outline.map(p=>p[0])), minZ=Math.min(...outline.map(p=>p[1]));
const width=Math.ceil((Math.max(...outline.map(p=>p[0]))-minX)/cell);
const depth=Math.ceil((Math.max(...outline.map(p=>p[1]))-minZ)/cell);
const columns=new Array(width*depth);

function mark(x,z,min,max){
 const at=z*width+x;
 let lo=Math.floor(min/unit), hi=Math.max(lo+1,Math.ceil(max/unit));
 const spans=columns[at]??(columns[at]=[]);
 let i=0;
 while(i<spans.length && spans[i+1]<lo) i+=2;
 let end=i;
 while(end<spans.length && spans[end]<=hi){
  if(spans[end]<=lo && spans[end+1]>=hi)return;
  lo=Math.min(lo,spans[end]);hi=Math.max(hi,spans[end+1]);end+=2;
 }
 spans.splice(i,end-i,lo,hi);
}

// Clip against each cell before taking its height range, so sloping roofs and
// long diagonal walls occupy only the cells their triangles actually touch.
function clip(poly,axis,edge,sign){
 const result=[];
 let a=poly.at(-1);
 for(const b of poly){
  const da=sign*(a[axis]-edge), db=sign*(b[axis]-edge);
  if((da<0)!==(db<0)){
   const t=da/(da-db);
   result.push([a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1]),a[2]+t*(b[2]-a[2])]);
  }
  if(db>=0)result.push(b);
  a=b;
 }
 return result;
}
function raster(a,b,c){
 const x0=Math.floor(Math.min(a[0],b[0],c[0])),x1=Math.floor(Math.max(a[0],b[0],c[0]));
 const z0=Math.floor(Math.min(a[2],b[2],c[2])),z1=Math.floor(Math.max(a[2],b[2],c[2]));
 if(x0===x1 && z0===z1){
  if(x0>=0 && x0<width && z0>=0 && z0<depth)
   mark(x0,z0,Math.min(a[1],b[1],c[1]),Math.max(a[1],b[1],c[1]));
  return;
 }
 for(let z=Math.max(0,z0);z<=Math.min(depth-1,z1);z++){
  const strip=clip(clip([a,b,c],2,z,1),2,z+1,-1);
  if(!strip.length)continue;
  for(let x=Math.max(0,x0);x<=Math.min(width-1,x1);x++){
   const polygon=clip(clip(strip,0,x,1),0,x+1,-1);
   if(polygon.length){
    const heights=polygon.map(p=>p[1]);
    mark(x,z,Math.min(...heights),Math.max(...heights));
   }
  }
 }
}

await MeshoptDecoder.ready;
const io=new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.decoder':MeshoptDecoder});
for(const id of ['world',...layout.stops.map(s=>s.id)]){
 const grass=id==='world' ? await loadGrassFaces() : null;
 const doc=await io.read(`${dir}/${id}.glb`);
 for(const node of doc.getRoot().listNodes()){
  // The existing continuous walk surface already handles the meadow terrain.
  if(!node.getMesh() || (id==='world' && node.getName().startsWith('Daylight ground ')))continue;
  // Cutout leaf sprays have transparent space between leaves. Only the wood is solid.
  if (id==='world' && node.getName().startsWith('Village tree foliage ')) continue;
  const m=node.getWorldMatrix();
  for(const primitive of node.getMesh().listPrimitives()){
   const points=primitive.getAttribute('POSITION'),indices=primitive.getIndices();
   const worldVertices=Array.from({length:points.getCount()},(_,i)=>{
    const [x,y,z]=points.getElement(i,[]);
    return [m[0]*x+m[4]*y+m[8]*z+m[12],
             m[1]*x+m[5]*y+m[9]*z+m[13],
            m[2]*x+m[6]*y+m[10]*z+m[14]];
   });
   const vertices=worldVertices.map(([x,y,z])=>[(x-minX)/cell,y,(z-minZ)/cell]);
   const order=indices?.getArray();
   for(let i=0;i<(order?.length??vertices.length);i+=3){
    const a=order?.[i]??i,b=order?.[i+1]??i+1,c=order?.[i+2]??i+2;
    if(grass?.take(worldVertices[a],worldVertices[b],worldVertices[c]))continue;
    raster(vertices[a],vertices[b],vertices[c]);
   }
  }
 }
 grass?.verify();
 console.log('Rasterized',id);
}

// Adjacent cells usually share a profile. Store each profile once and encode row runs.
const profiles=[[]], ids=new Map([['',0]]), rows=[];
for(let z=0;z<depth;z++){
 const row=[];
 for(let x=0;x<width;x++){
  const spans=columns[z*width+x]??[],key=spans.join(',');
  if(!ids.has(key)){ids.set(key,profiles.length);profiles.push(spans);}
  const id=ids.get(key);
  if(id!==row.at(-1))row.push(x,id);
 }
 rows.push(row);
}
await writeFile(`${dir}/collision.json`,JSON.stringify({minX,minZ,cell,unit,width,depth,profiles,rows}));
