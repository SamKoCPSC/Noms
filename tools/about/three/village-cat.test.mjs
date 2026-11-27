import assert from 'node:assert/strict';
import test from 'node:test';
import * as THREE from 'three';
import { makeTabby } from '../village/tabby/model.mjs';
import { createVillageCat } from './village-cat.js';

test('nearby taps unlock the song on the third interaction and restore the tour afterward', (t) => {
  const surface = new EventTarget(),
    doc = new EventTarget(),
    scene = new THREE.Scene();
  const daylight = new THREE.Fog('#c6e0ed', 100, 300);
  scene.fog = daylight;
  const sky = {
    material: {
      uniforms: {
        top: { value: new THREE.Color('#70b6e1') },
        bottom: { value: new THREE.Color('#e4edcf') }
      }
    }
  };
  const dayTop = sky.material.uniforms.top.value.clone();
  let reduced = false;
  const camera = new THREE.PerspectiveCamera(65, 1, 0.08, 650);
  let callback,
    blocked = false,
    draws = 0;
  const recordings = [];
  const ambient = { muted: false };
  class Recording {
    currentTime = 0;
    duration = 30;
    play() {
      recordings.push(this);
      return Promise.resolve();
    }
    pause() {
      this.paused = true;
    }
    removeAttribute(name) {
      delete this[name];
    }
    load() {}
  }
  for (const [name, value] of Object.entries({
    Audio: Recording,
    document: doc,
    matchMedia: () => ({ matches: reduced }),
    requestAnimationFrame: (fn) => {
      callback = fn;
      return 1;
    },
    cancelAnimationFrame: () => {
      callback = null;
    }
  })) {
    const original = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { value, configurable: true });
    t.after(() => (original ? Object.defineProperty(globalThis, name, original) : delete globalThis[name]));
  }
  const canvas = {
    closest: () => ({
      dataset: { catSongUrl: '/assets/about/oiia-spinning-cat.mp3' },
      querySelector: (selector) => (selector === '#village-look-surface' ? surface : ambient)
    }),
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 100, height: 100 })
  };
  const secret = createVillageCat(
    scene,
    camera,
    canvas,
    { gap: () => (blocked ? null : {}) },
    () => {
      draws++;
    },
    sky,
    makeTabby()
  );

  const cat = scene.getObjectByName('Tabby'),
    home = cat.rotation.y;
  function aim(range) {
    camera.position.copy(cat.position).add(new THREE.Vector3(-range, 0.75, 0));
    camera.lookAt(cat.position.clone().add(new THREE.Vector3(0, 0.35, 0)));
    camera.updateMatrixWorld();
  }
  function pointer(type, x = 50) {
    const e = Object.assign(new Event(type), {
      isPrimary: true,
      button: 0,
      pointerId: 1,
      clientX: x,
      clientY: 50,
      pointerType: 'touch'
    });
    surface.dispatchEvent(e);
  }
  function tap() {
    pointer('pointerdown');
    pointer('pointerup');
  }
  aim(2);
  tap();
  assert.ok(!callback);
  secret.update([1]);
  aim(8);
  tap();
  assert.ok(!callback);
  aim(2);
  blocked = true;
  tap();
  assert.ok(!callback);
  blocked = false;
  pointer('pointerdown');
  pointer('pointermove', 65);
  pointer('pointerup');
  assert.ok(!callback);
  tap();
  assert.ok(callback);
  callback(performance.now() + 500);
  assert.notEqual(cat.rotation.y, home);
  const ongoing = callback;
  tap();
  assert.equal(callback, ongoing);
  callback(performance.now() + 3050);
  assert.equal(cat.rotation.y, home);
  assert.ok(!callback);
  doc.pointerLockElement = surface;
  pointer('pointerdown', 2);
  pointer('pointerup', 2);
  assert.ok(callback);
  secret.update([]);
  assert.equal(cat.visible, false);
  assert.equal(cat.rotation.y, home);
  assert.ok(!callback);
  assert.equal(draws, 2);
  const perch = cat.position.clone();
  const shadow = scene.children.find((mesh) => mesh !== cat);
  assert.equal(recordings.length, 0);
  secret.update([1]);
  tap();
  tap();
  assert.equal(recordings.length, 1);
  const audio = recordings[0];
  assert.equal(audio.src, '/assets/about/oiia-spinning-cat.mp3');
  assert.equal(scene.fog, daylight);
  audio.onplaying();
  assert.equal(ambient.muted, true);
  const dancers = scene.getObjectByName('Dancing tabbies');
  assert.equal(dancers.count, 0);
  audio.currentTime = 3.835;
  callback();
  assert.ok(cat.position.y < perch.y + 0.3);
  // Halfway through the opening burst, the cat has already made 1.5 turns.
  assert.ok(Math.abs(cat.rotation.y - home - 3 * Math.PI) < 1e-6);
  audio.currentTime = 4.24;
  callback();
  assert.equal(cat.rotation.y, home);
  audio.currentTime = 4.635;
  callback();
  assert.ok(Math.abs(cat.rotation.y - home + 3 * Math.PI) < 1e-6);
  assert.equal(dancers.count, 2);
  const rig = scene.getObjectByName('Cat party lights');
  assert.ok(rig);
  const smoke = rig.getObjectByName('Perch smoke jets');
  const openingDensity = smoke.material.uniforms.density.value;
  assert.ok(openingDensity > 0);
  const beams = rig.children.filter((child) => child.isMesh && child !== smoke);
  assert.equal(beams.length, 18);
  const backBeam = beams.at(-1);
  const openingStrength = backBeam.material.uniforms.strength.value;
  assert.ok(scene.fog.far < daylight.far);
  assert.ok(!sky.material.uniforms.top.value.equals(dayTop));
  const beam = rig.children[0];
  const direction = beam.quaternion.clone();
  const openingColor = beam.material.uniforms.tint.value.clone();
  audio.currentTime = 15;
  callback();
  assert.ok(cat.position.y > perch.y + 1.5);
  assert.ok(Math.abs(cat.position.x - perch.x) > 0.5);
  assert.equal(shadow.material.opacity, 0);
  assert.ok(!beam.quaternion.equals(direction));
  assert.ok(!beam.material.uniforms.tint.value.equals(openingColor));
  assert.ok(backBeam.material.uniforms.strength.value > openingStrength);
  assert.ok(smoke.material.uniforms.density.value > openingDensity);
  assert.equal(dancers.count, 4);
  const aloft = cat.position.clone();
  callback();
  assert.ok(cat.position.equals(aloft));
  audio.currentTime = 22;
  callback();
  assert.equal(dancers.count, 8);
  assert.ok(dancers.position.equals(cat.position));
  audio.currentTime = audio.duration - 0.1;
  callback();
  assert.ok(cat.position.distanceTo(perch) < 0.01);
  assert.equal(cat.rotation.y, home);
  assert.ok(shadow.material.opacity > 0.99);
  assert.ok(smoke.material.uniforms.density.value < 0.01);
  assert.equal(dancers.count, 0);
  audio.onended();
  assert.equal(scene.getObjectByName('Dancing tabbies'), undefined);
  assert.equal(scene.getObjectByName('Cat party lights'), undefined);
  assert.equal(scene.fog, daylight);
  assert.ok(sky.material.uniforms.top.value.equals(dayTop));
  assert.equal(audio.paused, true);
  assert.equal(audio.src, undefined);
  assert.equal(ambient.muted, false);
  assert.ok(cat.position.equals(perch));
  assert.equal(cat.rotation.y, home);
  assert.deepEqual(cat.scale.toArray(), [1, 1, 1]);
  assert.equal(shadow.material.opacity, 1);
  assert.ok(!callback);

  for (let i = 0; i < 2; i++) {
    tap();
    callback(performance.now() + 3050);
  }
  tap();
  recordings[1].onerror();
  tap();
  assert.equal(recordings.length, 3);
  ambient.muted = true;
  recordings[2].onplaying();
  secret.update([]);
  assert.equal(recordings[2].paused, true);
  assert.equal(ambient.muted, true);
  assert.ok(!callback);
  assert.equal(scene.fog, daylight);
  assert.equal(scene.getObjectByName('Dancing tabbies'), undefined);
  assert.equal(scene.getObjectByName('Cat party lights'), undefined);
  reduced = true;
  secret.update([1]);
  for (let i = 0; i < 2; i++) {
    tap();
    callback(performance.now() + 3050);
  }
  tap();
  recordings[3].onplaying();
  assert.equal(scene.getObjectByName('Dancing tabbies'), undefined);
  assert.equal(scene.getObjectByName('Cat party lights'), undefined);
  assert.equal(scene.fog, daylight);
  assert.ok(!callback);
  secret.dispose();
  assert.equal(scene.children.length, 0);
});
