// The four outer corners in the master layout define a rectangular meadow.
import * as THREE from 'three';
import {groundHeight} from './walking-world.js';

const postWidth = 0.2;
const maxSpan = 3.2;

export function createPerimeter(outline) {
  const bounds = new THREE.Box2().setFromPoints(outline.map(([x,z]) => new THREE.Vector2(x,z)));
  const {min, max} = bounds;
  const corners = [[min.x,min.y], [max.x,min.y], [max.x,max.y], [min.x,max.y]];
  return {
    corners,
    constrain(x,z,bodyRadius) {
      const inset = postWidth / 2 + bodyRadius;
      return [THREE.MathUtils.clamp(x,min.x+inset,max.x-inset),
              THREE.MathUtils.clamp(z,min.y+inset,max.y-inset)];
    }
  };
}

export function createFence(perimeter, terrain) {
  const points = [];
  perimeter.corners.forEach(([x,z], i, corners) => {
    const [nextX,nextZ] = corners[(i+1) % corners.length];
    const spans = Math.ceil(Math.hypot(nextX-x,nextZ-z) / maxSpan);
    // The next side supplies the corner post, including the closing corner.
    for (let j=0; j<spans; j++) {
      const px = THREE.MathUtils.lerp(x,nextX,j/spans);
      const pz = THREE.MathUtils.lerp(z,nextZ,j/spans);
      points.push(new THREE.Vector3(px,groundHeight(terrain,px,pz),pz));
    }
  });

  // A low, chamfered cap on each square post catches the village daylight.
  const radius = postWidth / Math.sqrt(2);
  const profile = [[radius,-0.12],[radius,1.31],[radius*0.77,1.36],[0,1.36]];
  const post = new THREE.LatheGeometry(profile.map(([r,y]) => new THREE.Vector2(r,y)),4,Math.PI/4);
  const posts = new THREE.InstancedMesh(post,new THREE.MeshStandardMaterial({color:0x84573c,roughness:1,flatShading:true}),points.length);
  const rails = new THREE.InstancedMesh(new THREE.BoxGeometry(1,0.14,0.09),new THREE.MeshStandardMaterial({color:0xb38658,roughness:1}),points.length*2);
  const matrix = new THREE.Matrix4(), beam = new THREE.Object3D();
  const delta = new THREE.Vector3();
  beam.rotation.order = 'YXZ';
  points.forEach((point,i) => {
    const next = points[(i+1) % points.length];
    posts.setMatrixAt(i,matrix.makeTranslation(point.x,point.y,point.z));
    delta.subVectors(next,point);
    beam.scale.x = delta.length();
    // Aim along the span without twisting the board on reverse-facing runs.
    beam.rotation.y = Math.atan2(-delta.z,delta.x);
    beam.rotation.z = Math.atan2(delta.y,Math.hypot(delta.x,delta.z));
    beam.position.addVectors(point,next).multiplyScalar(0.5);
    beam.position.y += 0.52;
    for (let rail=0; rail<2; rail++) {
      beam.updateMatrix();
      rails.setMatrixAt(i*2+rail,beam.matrix);
      beam.position.y += 0.58;
    }
  });
  const fence = new THREE.Group();
  fence.name = 'Village perimeter fence';
  fence.add(posts,rails);
  return fence;
}
