// Two low jets take turns emitting bursts every two beats of the recording.
import * as THREE from 'three';

export function createPartySmoke() {
  const material = new THREE.ShaderMaterial({
    uniforms: { beat: { value: 0 }, density: { value: 0 } },
    transparent: true,
    depthWrite: false,
    vertexShader: `
      #include <common>
      #include <logdepthbuf_pars_vertex>
      uniform float beat;
      varying vec2 puffUV;
      varying float age, seed, jet;
      void main() {
        float id = float(gl_InstanceID);
        jet = mod(id, 2.);
        float side = 2. * jet - 1.;
        float puff = floor(id * 0.5);
        // Three puffs per burst, four bursts per jet in each 16-beat cycle.
        float launch = 4. * floor(puff / 3.) + mod(puff, 3.) * 0.44 + 2. * jet;
        float cycle = floor((beat - launch) / 16.);
        seed = fract(sin(id * 17.31 + cycle * 37.2) * 4375.2);
        age = clamp((beat - launch - cycle * 16.) / 9., 0., 1.);
        vec3 center = vec3(
          0.8 + age * (1.6 + seed) + 0.23 * sin(age * 5. + seed * 6.),
          -0.10 + age * (0.55 + 0.52 * seed),
          side * (2.75 - age * (2.8 + 0.9 * seed))
        );
        vec4 eye = modelViewMatrix * vec4(center, 1.);
        eye.xy += position.xy * vec2(0.32 + age * 3.9, 0.22 + age * 1.65);
        if (age >= 1.) eye = vec4(2., 2., 1., 1.); // Hide expired puffs behind the camera.
        puffUV = uv * 2. - 1.;
        gl_Position = projectionMatrix * eye;
        #include <logdepthbuf_vertex>
      }`,
    fragmentShader: `
      #include <common>
      #include <logdepthbuf_pars_fragment>
      uniform float density;
      varying vec2 puffUV;
      varying float age, seed, jet;
      float hash(vec2 p) {
        p = fract(p * vec2(0.1031, 0.11369));
        p += dot(p, p.yx + 19.19);
        return fract((p.x + p.y) * p.x);
      }
      float noise(vec2 p) {
        vec2 cell = floor(p), f = fract(p);
        f = f * f * (3. - 2. * f);
        return mix(mix(hash(cell), hash(cell + vec2(1., 0.)), f.x),
                   mix(hash(cell + vec2(0., 1.)), hash(cell + 1.), f.x), f.y);
      }
      void main() {
        #include <logdepthbuf_fragment>
        vec2 q = puffUV * 2.7 + vec2(seed * 35., -age * 1.7);
        q += 0.85 * noise(q * 0.8 + age);
        float curls = 0.56 * noise(q) + 0.30 * noise(q * 2.1) + 0.14 * noise(q * 4.3);
        float cloud = 1. - smoothstep(curls * 0.48, 0.66 + curls * 0.34, length(puffUV));
        float life = smoothstep(0., 0.12, age) * (1. - smoothstep(0.52, 1., age));
        vec3 tint = mix(vec3(0.80, 0.42, 0.71), vec3(0.30, 0.76, 0.90), jet);
        vec3 color = mix(tint, vec3(0.88, 0.90, 0.94), curls * 0.74);
        gl_FragColor = vec4(color, density * life * cloud * smoothstep(0.20, 0.78, curls));
        #include <colorspace_fragment>
      }`
  });
  const smoke = new THREE.InstancedMesh(new THREE.PlaneGeometry(1, 1), material, 24);
  smoke.name = 'Perch smoke jets';
  // The vertex shader places each puff independently of the unit plane.
  smoke.frustumCulled = false;
  return smoke;
}
