import * as THREE from 'three';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { mergeVertices } from 'three/addons/utils/BufferGeometryUtils.js';

export const WORLD_STYLE = Object.freeze({ roughness: 1, metalness: 0, clearcoat: 0, specularIntensity: .08, envMapIntensity: .025 });
let roundedShapes = 0;
export const styleSummary = () => ({ ...WORLD_STYLE, rounded_shapes: roundedShapes });
export const worldMaterial = color => new THREE.MeshPhysicalMaterial({ color, ...WORLD_STYLE, toneMapped: false });

/** Small, real bevels inside the original footprint; no change to placement. */
export function softBox(size, radius = .12) {
  // Thin signs and doors need rounded face corners as well as a small bevel;
  // a 3D box bevel alone would be limited by their shallow thickness.
  if(size[2]<.3 && size[0]>.6 && size[1]>.32) {
    const [w,h,d]=size,x=w/2,y=h/2,r=Math.min(.18,w*.12,h*.18),s=new THREE.Shape();
    s.moveTo(-x+r,-y);s.lineTo(x-r,-y);s.quadraticCurveTo(x,-y,x,-y+r);
    s.lineTo(x,y-r);s.quadraticCurveTo(x,y,x-r,y);s.lineTo(-x+r,y);
    s.quadraticCurveTo(-x,y,-x,y-r);s.lineTo(-x,-y+r);s.quadraticCurveTo(-x,-y,-x+r,-y);
    const bevel=Math.min(.015,d*.2),g=new THREE.ExtrudeGeometry(s,{depth:d-2*bevel,bevelEnabled:true,bevelThickness:bevel,bevelSize:bevel,bevelSegments:2,curveSegments:4,steps:1});
    g.computeBoundingBox();const center=g.boundingBox.getCenter(new THREE.Vector3()),extent=g.boundingBox.getSize(new THREE.Vector3());
    g.translate(-center.x,-center.y,-center.z);g.scale(w/extent.x,h/extent.y,d/extent.z);
    g.deleteAttribute('uv');const welded=mergeVertices(g,1e-6);g.dispose();roundedShapes++;return welded;
  }
  const g = new RoundedBoxGeometry(...size, 2, Math.min(radius, Math.min(...size) * .24));
  g.deleteAttribute('uv');
  const welded = mergeVertices(g, 1e-6); g.dispose(); roundedShapes++;
  return welded;
}

/** Bevel a roof prism, then fit it back to the original silhouette bounds. */
export function softRoof(width, depth, rise) {
  const s = new THREE.Shape();s.moveTo(-width/2,0);s.lineTo(width/2,0);s.lineTo(0,rise);s.closePath();
  const g = new THREE.ExtrudeGeometry(s,{depth:depth-.2,bevelEnabled:true,bevelThickness:.1,bevelSize:.1,bevelSegments:3,steps:1,curveSegments:3});
  g.computeBoundingBox();const b=g.boundingBox,center=b.getCenter(new THREE.Vector3()),size=b.getSize(new THREE.Vector3());
  g.translate(-center.x,-center.y,-center.z);g.scale(width/size.x,rise/size.y,depth/size.z);g.translate(0,rise/2,0);
  g.deleteAttribute('uv');const welded=mergeVertices(g,1e-6);g.dispose();roundedShapes++;return welded;
}

/** Gentle shoulders on path ribbons, keeping the centerline and width intact. */
export function softRibbon(points, width, heightAt, edgeDrop = .08) {
  const cross = [-1,-.97,-.9,.9,.97,1], drop = [1,.3,0,0,.3,1], positions = [], indices = [];
  for(let i=0;i<points.length;i++) {
    const p=points[i],a=points[Math.max(0,i-1)],b=points[Math.min(points.length-1,i+1)];
    const side=new THREE.Vector3(-(b.z-a.z),0,b.x-a.x).normalize();
    for(let j=0;j<cross.length;j++) {
      const v=p.clone().addScaledVector(side,cross[j]*width/2);v.y=heightAt(v,i/(points.length-1))-drop[j]*edgeDrop;positions.push(...v.toArray());
    }
    if(i<points.length-1)for(let j=0;j<cross.length-1;j++){const n=i*cross.length+j;indices.push(n,n+1,n+cross.length,n+1,n+cross.length+1,n+cross.length);}
  }
  const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(positions,3));g.setIndex(indices);g.computeVertexNormals();return g;
}

/** Compact portable geometry, including the normals that make bevels look soft.
 * Six decimal places are well below a visible change at village scale. Only
 * generated world meshes enter this export; source room assets stay untouched.
 */
export function exportEnvironment(terrain) {
  terrain.updateMatrixWorld(true);const result=[],round=x=>Math.round(x*1e6)/1e6;
  terrain.traverse(o=>{
    if(!o.isMesh)return;
    const original=o.geometry.clone();original.deleteAttribute('uv');
    const g=mergeVertices(original,1e-6);original.dispose();g.applyMatrix4(o.matrixWorld);
    const p=g.attributes.position,n=g.attributes.normal;
    const vectors=a=>Array.from({length:a.count},(_,i)=>[round(a.getX(i)),round(a.getY(i)),round(a.getZ(i))]);
    result.push({name:o.name||'Landscape detail',vertices:vectors(p),normals:vectors(n),indices:g.index?Array.from(g.index.array):Array.from({length:p.count},(_,i)=>i),color:o.material.color.toArray(),roughness:o.material.roughness,specular_intensity:o.material.specularIntensity});g.dispose();
  });return result;
}
