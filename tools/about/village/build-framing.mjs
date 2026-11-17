// Tight screen-space framing avoids the empty corners of a world-axis box
// shrinking angled rooms (especially the bakery) in the responsive tour.
import {NodeIO} from '@gltf-transform/core';
import {ALL_EXTENSIONS} from '@gltf-transform/extensions';
import {MeshoptDecoder} from 'meshoptimizer';
import * as THREE from 'three';
import {readFile,writeFile} from 'node:fs/promises';
const dir='assets/about/village',layout=JSON.parse(await readFile(`${dir}/layout.json`,'utf8'));
const io=new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.decoder':MeshoptDecoder});
for(const stop of layout.stops){
 const doc=await io.read(`${dir}/${stop.id}.glb`),camera=new THREE.PerspectiveCamera();
 camera.position.fromArray(stop.pose.eye);camera.lookAt(new THREE.Vector3(...stop.pose.target));camera.updateMatrixWorld(true);
 const min=[Infinity,Infinity],max=[-Infinity,-Infinity],extrema=[null,null,null,null],point=new THREE.Vector3(),worldPoint=new THREE.Vector3();
 for(const node of doc.getRoot().listNodes()){
  if(!node.getMesh())continue;
  const world=new THREE.Matrix4().fromArray(node.getWorldMatrix()),view=new THREE.Matrix4().multiplyMatrices(camera.matrixWorldInverse,world);
  for(const primitive of node.getMesh().listPrimitives()){
   const accessor=primitive.getAttribute('POSITION'),positions=accessor.getArray(),vertex=[];
   for(let i=0;i<positions.length;i+=3){
    accessor.getElement(i/3,vertex);point.fromArray(vertex).applyMatrix4(view);
    if(point.z>=-.05)throw Error(`Walking camera inside ${stop.id}`);
    const slopes=[point.x/-point.z,point.y/-point.z];
    for(let axis=0;axis<2;axis++){
     if(slopes[axis]<min[axis]){min[axis]=slopes[axis];extrema[axis]=worldPoint.fromArray(vertex).applyMatrix4(world).toArray();}
     if(slopes[axis]>max[axis]){max[axis]=slopes[axis];extrema[axis+2]=worldPoint.fromArray(vertex).applyMatrix4(world).toArray();}
    }
   }
  }
 }
 stop.framing={min,max,extrema};console.log('FRAMING',stop.id,min,max);
}
await writeFile(`${dir}/layout.json`,JSON.stringify(layout));
