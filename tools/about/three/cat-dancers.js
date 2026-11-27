import * as THREE from 'three';
import { burstSpin, recordingBeat } from './cat-choreography.js';

const tau = Math.PI * 2;
const smooth = THREE.MathUtils.smootherstep;
const pairs = [
  { beat: 0, depth: 0.45, height: 0.24, width: 0.92 },
  { beat: 16, depth: 1.10, height: 0.76, width: 1.72 }
];
const ringSize = 8;

export function createCatDancers(scene, lead, home) {
  // Choose each flight once per performance. Sampling the song clock keeps curves
  // continuous even when the frame rate drops or the browser seeks in the audio.
  const flyers = Array.from({ length: 6 }, (_, i) => ({
    cue: 24 + 8 * Math.floor(i / 2),
    heading: (i + Math.random() * 0.65) * tau / 6,
    crossesPerch: i % 3 === 2,
    speed: (0.83 + Math.random() * 0.72) * (i % 2 ? -1 : 1),
    weave: 0.7 + Math.random() * 0.6,
    phase: Math.random() * tau,
    height: 1.4 + Math.random() * 1.6
  }));
  const dancers = new THREE.InstancedMesh(lead.geometry, lead.material, pairs.length * 2 + ringSize + flyers.length);
  dancers.name = 'Dancing tabbies';
  dancers.position.copy(home);
  dancers.count = 0;
  dancers.frustumCulled = false;
  dancers.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  scene.add(dancers);
  const pose = new THREE.Object3D();
  const leadOffset = new THREE.Vector3();
  function addPose() {
    pose.updateMatrix();
    dancers.setMatrixAt(dancers.count++, pose.matrix);
  }
  return {
    update(elapsed, duration) {
      const beat = recordingBeat(elapsed);
      const landing = smooth(duration - elapsed, 1, 4);
      leadOffset.subVectors(lead.position, home);
      dancers.count = 0;
      for (const { beat: cue, depth, height, width } of pairs) {
        const split = smooth(beat - cue, 0, 1.5) * landing;
        if (split === 0) continue;
        for (const side of [-1, 1]) {
          const sway = Math.sin(beat * Math.PI / 4 + width + side * 0.5);
          pose.position.set(depth + 0.18 * sway, height + 0.15 * sway, side * width).multiplyScalar(split).add(leadOffset);
          pose.rotation.copy(lead.rotation);
          pose.rotation.z += side * split * 0.16;
          pose.scale.copy(lead.scale).multiplyScalar(0.82 * split);
          addPose();
        }
      }
      for (let i = 0; i < ringSize; i++) {
        // Four dancers take their places first, then four more fill the gaps.
        const cue = i % 2 ? 16 : 8;
        const size = smooth(beat - cue, 0, 2) * landing;
        if (!size) continue;
        const angle = (i + 0.5) * tau / ringSize;
        const step = beat + i * 0.25;
        const bounce = Math.sin(Math.PI * step) ** 2;
        pose.position.set(2.15 * Math.cos(angle), -0.08 + 0.22 * bounce, 2.15 * Math.sin(angle));
        pose.rotation.set(0, -Math.PI / 2 - angle + burstSpin(step, 1), 0.12 * Math.sin(Math.PI * step));
        pose.scale.setScalar(0.77 * size);
        addPose();
      }
      for (const flight of flyers) {
        const age = beat - flight.cue;
        const size = smooth(age, 0, 2) * landing;
        if (!size) continue;
        const t = age * flight.speed / 4;
        const angle = flight.heading + t;
        const radius = 2.45 + 0.85 * Math.sin(t * flight.weave + flight.phase);
        const lift = smooth(age, 0, 4) * landing;
        pose.position.set(radius * Math.cos(angle),
          0.1 + lift * (flight.height + 0.75 * Math.sin(1.7 * t + flight.phase)),
          radius * Math.sin(angle));
        if (flight.crossesPerch) {
          // A figure eight sends these two through the middle of the other orbits.
          const along = 3.1 * Math.cos(t);
          const across = 1.65 * Math.sin(2 * t);
          pose.position.x = along * Math.cos(flight.heading) - across * Math.sin(flight.heading);
          pose.position.z = along * Math.sin(flight.heading) + across * Math.cos(flight.heading);
        }
        pose.rotation.set(0.3 * Math.sin(t), -angle + burstSpin(age + flight.phase, 2), 0.45 * Math.cos(t + flight.phase));
        pose.scale.setScalar(0.84 * size);
        addPose();
      }
      dancers.instanceMatrix.needsUpdate = true;
    },
    dispose() {
      dancers.removeFromParent();
      dancers.dispose();
    }
  };
}
