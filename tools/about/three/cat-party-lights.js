// Backlit shafts are camera-facing ribbons so the light remains visible in the baked village.
import * as THREE from 'three';
import { createPartySmoke } from './cat-party-smoke.js';
import { recordingBeat } from './cat-choreography.js';

export function createCatPartyLights(scene, sky, home, camera) {
  const rig = new THREE.Group();
  rig.name = 'Cat party lights';
  rig.position.copy(home);
  const shafts = new THREE.PlaneGeometry(1, 1);
  shafts.translate(0, 0.5, 0);
  const colors = ['#ff51ba', '#47e5ff', '#ffbb4d', '#9871ff', '#54ffad', '#608cff'].map(
    (color) => new THREE.Color(color)
  );
  const fixtures = [
    { x: 0.65, z: -2 },
    { x: 0.65, z: 2 },
    { x: 3.6, z: -3.5 },
    { x: 3.6, z: 3.5 },
    ...Array.from({ length: 6 }, (_, i) => ({
      x: 6.8 - 0.3 * Math.abs(i - 2.5),
      z: (i - 2.5) * 1.75,
      bank: 'fan'
    })),
    ...Array.from({ length: 8 }, (_, i) => ({
      x: 0.4 + 1.65 * Math.floor(i / 2),
      z: (i % 2 ? 1 : -1) * (3.5 + 0.32 * Math.floor(i / 2)),
      bank: 'wings'
    }))
  ];
  const beams = fixtures.map(({ x, z, bank }, i) => {
    const material = new THREE.ShaderMaterial({
      uniforms: { tint: { value: colors[i % colors.length].clone() }, strength: { value: 0 } },
      transparent: true,
      depthWrite: false,
      side: THREE.DoubleSide,
      forceSinglePass: true,
      vertexShader: `
        #include <common>
        #include <logdepthbuf_pars_vertex>
        varying vec3 beamCoords;
        void main() {
          vec3 p = position;
          float width = mix(0.12, 4.8, uv.y);
          p.x *= width;
          beamCoords = vec3(2. * p.x, width, uv.y);
          p.y *= 7.;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.);
          #include <logdepthbuf_vertex>
        }`,
      fragmentShader: `
        #include <common>
        #include <logdepthbuf_pars_fragment>
        uniform vec3 tint;
        uniform float strength;
        varying vec3 beamCoords;
        void main() {
          #include <logdepthbuf_fragment>
          // Normalize after interpolation to keep the core straight across the taper.
          float side = abs(beamCoords.x / beamCoords.y);
          float haze = exp(-3.8 * side * side) * (1. - smoothstep(0.67, 1., side));
          float core = exp(-26. * side * side) * (1. - beamCoords.z);
          float fade = 1. - smoothstep(0.45, 1., beamCoords.z);
          gl_FragColor = vec4(mix(tint, vec3(1.), 0.4 * core), strength * fade * (0.59 * haze + 0.4 * core));
          #include <colorspace_fragment>
        }`
    });
    const beam = new THREE.Mesh(shafts, material);
    // The vertex shader widens and lengthens the unit plane.
    beam.frustumCulled = false;
    beam.position.set(x, -0.13, z);
    if (bank === 'fan') beam.scale.set(0.62, 1.35, 1);
    if (bank === 'wings') beam.scale.set(0.44, 1.15, 1);
    rig.add(beam);
    return beam;
  });
  const fill = new THREE.PointLight(colors[0], 0, 7);
  const smoke = createPartySmoke();
  rig.add(fill, smoke);
  scene.add(rig);

  const daylight = scene.fog;
  const fog = daylight.clone();
  scene.fog = fog;
  const skyTop = sky.material.uniforms.top.value;
  const skyBottom = sky.material.uniforms.bottom.value;
  const dayTop = skyTop.clone(),
    dayBottom = skyBottom.clone();
  const nightTop = new THREE.Color('#181b4c'),
    nightHorizon = new THREE.Color('#584479');
  const axis = new THREE.Vector3(),
    across = new THREE.Vector3(),
    normal = new THREE.Vector3();
  const view = new THREE.Vector3(),
    basis = new THREE.Matrix4();
  return {
    update(elapsed, duration, catPosition) {
      const beat = recordingBeat(elapsed);
      const smooth = THREE.MathUtils.smootherstep;
      const envelope = smooth(elapsed, 0, 1.6) * (1 - smooth(elapsed, duration - 2, duration));
      const pulse = 0.5 + 0.5 * Math.cos(beat * Math.PI * 2);
      const sweep = (beat * Math.PI) / 8;
      const flight = smooth(beat, 16, 24);
      smoke.material.uniforms.beat.value = beat;
      smoke.material.uniforms.density.value = envelope * (0.48 + 0.24 * flight);
      skyTop.copy(dayTop).lerp(nightTop, envelope);
      skyBottom.copy(dayBottom).lerp(nightHorizon, envelope);
      fog.color.copy(daylight.color).lerp(nightHorizon, envelope);
      fog.near = THREE.MathUtils.lerp(daylight.near, 0.75, envelope);
      fog.far = THREE.MathUtils.lerp(daylight.far, 22, envelope);

      // Advance the palette every four beats, then every two, then every beat in the finale.
      const colorSteps =
        THREE.MathUtils.clamp(beat, 0, 16) / 4 +
        THREE.MathUtils.clamp(beat - 16, 0, 24) / 2 +
        Math.max(0, beat - 40);
      beams.forEach((beam, i) => {
        const { z, bank } = fixtures[i];
        // Stagger the accents across the stage; each beam holds its hue between hits.
        const cue = colorSteps + (5 - z) * 0.09;
        const step = Math.floor(cue);
        const phase = cue - step;
        const previous = (i + step) % colors.length;
        const next = (previous + 1) % colors.length;
        const uniforms = beam.material.uniforms;
        uniforms.tint.value.lerpColors(colors[previous], colors[next], smooth(phase, 0, 0.2));
        const accent = 1 - smooth(phase, 0.16, 0.6);
        const wave = sweep + (i * Math.PI) / 2;
        if (bank === 'fan') {
          // Sweep the back row together, opening into a taller fan during flight.
          const spread = 0.07 + 0.11 * flight;
          axis.set(-0.15, 1, z * spread + 0.35 * Math.sin(sweep + z * 0.15));
          const chase = 0.5 + 0.5 * Math.cos((beat * Math.PI) / 2 - z * 0.85);
          uniforms.strength.value = envelope * (0.4 + 0.38 * flight + 0.22 * chase);
        } else if (bank === 'wings') {
          // Cross the sky from opposite sides above the smoke jets.
          axis.set(0.28 + 0.48 * Math.sin(wave), 1, -Math.sign(z) * (0.8 + 0.5 * Math.cos(wave)));
          uniforms.strength.value = envelope * (0.34 + 0.48 * flight + 0.18 * pulse);
        } else {
          axis.set(0.18 + 0.36 * Math.sin(wave), 1, -Math.sign(z) * (0.3 + 0.45 * Math.cos(wave)));
          uniforms.strength.value = envelope * (0.67 + 0.17 * pulse + 0.16 * flight);
        }
        uniforms.strength.value *= 0.74 + 0.26 * accent;
        axis.normalize();
        view.copy(camera.position).sub(home).sub(beam.position);
        across.crossVectors(axis, view).normalize();
        normal.crossVectors(across, axis);
        beam.quaternion.setFromRotationMatrix(basis.makeBasis(across, axis, normal));
      });
      fill.position.copy(catPosition).sub(home);
      fill.position.x -= 0.65;
      fill.position.y += 0.85;
      fill.color
        .copy(beams[0].material.uniforms.tint.value)
        .lerp(beams[1].material.uniforms.tint.value, 0.5 + 0.5 * Math.sin(sweep));
      fill.intensity = envelope * (5 + 2 * pulse);
    },
    dispose() {
      skyTop.copy(dayTop);
      skyBottom.copy(dayBottom);
      scene.fog = daylight;
      rig.removeFromParent();
      shafts.dispose();
      beams.forEach((beam) => beam.material.dispose());
      smoke.dispose();
      smoke.geometry.dispose();
      smoke.material.dispose();
      fill.dispose();
    }
  };
}
