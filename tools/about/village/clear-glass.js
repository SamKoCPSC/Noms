import { ShaderChunk } from 'three';

// Three's reflection anti-aliasing adds a minimum/geometry roughness even to a
// polished material, and its bicubic transmission lookup softens the image.
// Keep that AA for reflected highlights, but sample the unblurred scene for our
// explicitly clear panes. Fresnel, IOR, refraction and energy conservation remain
// in MeshPhysicalMaterial; this is not an alpha-blended glass substitute.
const source = ShaderChunk.transmission_pars_fragment;
const signature = /vec4 getTransmissionSample\( const in vec2 fragCoord, const in float roughness, const in float ior \) \{[\s\S]*?\n\t\}/;
if (!signature.test(source)) throw new Error('Three.js clear-glass transmission shader changed; review the sampling override.');
const clearTransmission = source.replace(signature, `vec4 getTransmissionSample( const in vec2 fragCoord, const in float roughness, const in float ior ) {
    return textureLod( transmissionSamplerMap, fragCoord, 0.0 );
  }`);

export function configureClearGlass(material) {
  if (!(material.userData.village_clear_glass || material.name.startsWith('Village clear cottage glazing'))) return;
  if (!material.isMeshPhysicalMaterial || material.transmission !== 1 || material.roughness !== 0) {
    throw new Error(`Invalid clear-glass material: ${material.name}`);
  }
  // The captured opaque scene already includes distance fog. Applying it
  // again to the pane washes out the transmitted view, especially in Overview.
  material.fog = false;
  material.onBeforeCompile = shader => {
    shader.fragmentShader = shader.fragmentShader.replace('#include <transmission_pars_fragment>', clearTransmission);
  };
  material.customProgramCacheKey = () => 'noms-clear-transmission-level-zero-v2';
  material.userData.clearTransmissionSampling = 'level-zero';
  material.needsUpdate = true;
}
