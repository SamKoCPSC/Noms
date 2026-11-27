import * as THREE from 'three';
import { recordingBeat } from './cat-choreography.js';

// Each cue splits a pair off the lead. Offsets are in village coordinates so the
// formation follows the flying cat without revolving at the speed of its spin.
const pairs = [
  { beat: 0, depth: 0.45, height: 0.24, width: 0.92 },
  { beat: 16, depth: 1.10, height: 0.76, width: 1.72 },
  { beat: 40, depth: 1.55, height: 0.18, width: 2.45 },
  { beat: 40, depth: 1.55, height: 1.50, width: 0.84 }
];

export function createCatDancers(scene, lead) {
  const dancers = new THREE.InstancedMesh(lead.geometry, lead.material, pairs.length * 2);
  dancers.name = 'Dancing tabbies';
  dancers.count = 0;
  dancers.frustumCulled = false;
  dancers.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  scene.add(dancers);
  const pose = new THREE.Object3D();
  return {
    update(elapsed, duration) {
      const beat = recordingBeat(elapsed);
      const landing = THREE.MathUtils.smootherstep(duration - elapsed, 1, 4);
      dancers.position.copy(lead.position);
      dancers.count = 0;
      for (const { beat: cue, depth, height, width } of pairs) {
        const split = THREE.MathUtils.smootherstep(beat - cue, 0, 1.5) * landing;
        if (split === 0) continue;
        for (const side of [-1, 1]) {
          const sway = Math.sin(beat * Math.PI / 4 + width + side * 0.5);
          pose.position.set(depth + 0.18 * sway, height + 0.15 * sway, side * width).multiplyScalar(split);
          pose.rotation.copy(lead.rotation);
          pose.rotation.z += side * split * 0.16;
          pose.scale.copy(lead.scale).multiplyScalar(0.82 * split);
          pose.updateMatrix();
          dancers.setMatrixAt(dancers.count++, pose.matrix);
        }
      }
      dancers.instanceMatrix.needsUpdate = true;
    },
    dispose() {
      dancers.removeFromParent();
      dancers.dispose();
    }
  };
}
