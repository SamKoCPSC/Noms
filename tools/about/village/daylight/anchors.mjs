// Keep the approved browser placements and camera route while baking in world space.
import {NodeIO} from '@gltf-transform/core';
import {ALL_EXTENSIONS} from '@gltf-transform/extensions';
import {MeshoptDecoder} from 'meshoptimizer';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const out=process.argv[2];if(!out)throw Error('Usage: anchors.mjs OUTPUT.json');
await MeshoptDecoder.ready;const io=new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.decoder':MeshoptDecoder});
const layout=JSON.parse(await readFile('assets/models/terraced-village/layout.json')),anchors={};
for(const stop of layout.stops){
 const bytes=await readFile(stop.model),doc=await io.readBinary(new Uint8Array(bytes)),min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];let triangles=0;
 for(const node of doc.getRoot().listNodes())if(node.getMesh()){
  const m=node.getWorldMatrix();for(const p of node.getMesh().listPrimitives()){
   const a=p.getAttribute('POSITION'),v=[];triangles+=(p.getIndices()?.getCount()||a.getCount())/3;
   for(let i=0;i<a.getCount();i++){a.getElement(i,v);for(let j=0;j<3;j++){const x=m[j]*v[0]+m[4+j]*v[1]+m[8+j]*v[2]+m[12+j];min[j]=Math.min(min[j],x);max[j]=Math.max(max[j],x);}}
  }
 }
 anchors[stop.id]={min,max,scale:stop.width/Math.max(max[0]-min[0],max[2]-min[2]),position:stop.position,yaw:stop.yaw,source:stop.model,sha256:createHash('sha256').update(bytes).digest('hex'),triangles};
 console.log(stop.id,triangles);
}
await writeFile(out,JSON.stringify(anchors,null,2)+'\n');
