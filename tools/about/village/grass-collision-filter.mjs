import {readFile} from 'node:fs/promises';
import {gunzipSync} from 'node:zlib';

// The daylight bake merges grass and solid props into the same spatial chunks.
// Identify grass by its authored faces, allowing for the GLB's 16-bit positions.
const tolerance = 0.005;
const bucket = (x,z) => `${x},${z}`;
const sameVertex = (a,b) => a.every((v,i) => Math.abs(v-b[i]) <= tolerance);

export async function loadGrassFaces() {
 const source = 'assets/models/terraced-village/daylight/source/vegetation.json.gz';
 const {meshes} = JSON.parse(gunzipSync(await readFile(source)));
 const buckets = new Map();
 let count = 0, matched = 0;
 for (const {name,vertices,indices} of meshes) {
  if (!name.startsWith('Folded meadow grass ')) continue;
  for (let i=0; i<indices.length; i+=3) {
   const points = indices.slice(i,i+3).map(index => vertices[index]);
   const x = (points[0][0]+points[1][0]+points[2][0])/3;
   const z = (points[0][2]+points[1][2]+points[2][2])/3;
   const face = {points,used:false};
   // Include neighboring buckets when quantization could move the centroid across an edge.
   for (let ix=Math.floor(x-tolerance); ix<=Math.floor(x+tolerance); ix++) {
    for (let iz=Math.floor(z-tolerance); iz<=Math.floor(z+tolerance); iz++) {
     const key = bucket(ix,iz);
     if (!buckets.has(key)) buckets.set(key,[]);
     buckets.get(key).push(face);
    }
   }
   count++;
  }
 }
 function take(a,b,c) {
  const key = bucket(Math.floor((a[0]+b[0]+c[0])/3), Math.floor((a[2]+b[2]+c[2])/3));
  const faces = buckets.get(key);
  if (!faces) return false;
  for (const face of faces) {
   if (face.used) continue;
   // Mesh optimization can rotate the index order while preserving winding.
   const p = face.points;
   for (let i=0; i<3; i++) {
    if (sameVertex(a,p[i]) && sameVertex(b,p[(i+1)%3]) && sameVertex(c,p[(i+2)%3])) {
     face.used=true;
     matched++;
     return true;
    }
   }
  }
  return false;
 }
 function verify() {
  if (!count || matched!==count) throw Error(`Matched ${matched} of ${count} authored grass faces in the village GLB`);
  console.log('Excluded meadow grass faces:',matched);
 }
 return {take,verify};
}
