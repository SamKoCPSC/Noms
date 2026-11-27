// A standing tabby on the limestone slab east of the first kitchen.
import * as THREE from 'three';
import { createCatDancers } from './cat-dancers.js';
import { createCatPartyLights } from './cat-party-lights.js';
import { burstSpin, danceToRecording } from './cat-choreography.js';

// The slab has two pitched top faces. Fit a feathered contact shadow across their ridge.
function makeContactShadow() {
  const geometry = new THREE.PlaneGeometry(0.96, 0.72, 40, 32);
  const position = geometry.getAttribute('position'),
    colors = [];
  for (let i = 0; i < position.count; i++) {
    const x = position.getX(i),
      z = -position.getY(i);
    const surface = Math.min(0.254 + 0.067 * x - 0.065 * z, 0.244 - 0.186 * x - 0.071 * z);
    position.setXYZ(i, x, surface + 0.005, z);
    const falloff = Math.max(0, 1 - (x / 0.48) ** 2 - (z / 0.36) ** 2);
    // Four soft contacts beneath the standing paws, plus the wider body shadow.
    let contact = 0;
    for (const [px, pz] of [
      [-0.16, 0.13],
      [-0.2, -0.05],
      [0.25, 0.03],
      [0.3, -0.18]
    ]) {
      contact = Math.max(contact, Math.exp(-(((x - px) / 0.071) ** 2 + ((z - pz) / 0.059) ** 2)));
    }
    colors.push(0.035, 0.027, 0.022, Math.min(0.78, 0.29 * falloff ** 1.5 + 0.5 * contact));
  }
  geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 4));
  const shadow = new THREE.Mesh(
    geometry,
    new THREE.MeshBasicMaterial({
      vertexColors: true,
      transparent: true,
      depthWrite: false
    })
  );
  shadow.position.set(-15.1, 0, 42.51);
  return shadow;
}

// One short original vowel chant. Shape a pitched buzz with three vocal resonances.
const beat = 0.235;
const phrase = [
  [196, 340, 800, 2450],
  [247, 280, 2220, 3100],
  [294, 280, 2220, 3100],
  [220, 780, 1320, 2500],
  [196, 340, 800, 2450],
  [262, 280, 2220, 3100],
  [330, 280, 2220, 3100],
  [220, 780, 1320, 2500],
  [247, 340, 800, 2450],
  [294, 280, 2220, 3100],
  [262, 280, 2220, 3100],
  [196, 780, 1320, 2500]
];
const duration = beat * phrase.length;

function chant(context) {
  const start = context.currentTime + 0.015;
  const voice = context.createOscillator(),
    envelope = context.createGain();
  voice.type = 'sawtooth';
  envelope.gain.value = 0;
  envelope.connect(context.destination);
  const filters = [1, 2, 3].map((_, i) => {
    const filter = context.createBiquadFilter();
    filter.type = 'bandpass';
    filter.Q.value = [3, 6, 9][i];
    voice.connect(filter);
    filter.connect(envelope);
    return filter;
  });
  phrase.forEach(([pitch, ...formants], i) => {
    const t = start + i * beat;
    voice.frequency.setTargetAtTime(pitch, t, 0.022);
    filters.forEach((f, j) => f.frequency.setTargetAtTime(formants[j], t, 0.028));
    envelope.gain.setValueAtTime(0.0001, t);
    envelope.gain.exponentialRampToValueAtTime(0.065, t + 0.026);
    envelope.gain.exponentialRampToValueAtTime(0.0001, t + beat - 0.005);
  });
  voice.start(start);
  voice.stop(start + duration);
  return () => {
    voice.disconnect();
    filters.forEach((f) => f.disconnect());
    envelope.disconnect();
  };
}

export function createVillageCat(scene, camera, canvas, collision, redraw, sky, cat) {
  const restYaw = -1.48;
  cat.position.set(-15.1, 0.208, 42.51);
  cat.rotation.y = restYaw;
  const shadow = makeContactShadow();
  cat.visible = shadow.visible = false;
  scene.add(cat, shadow);
  const tour = canvas.closest('#about-tour');
  const surface = tour.querySelector('#village-look-surface');
  const home = cat.position.clone();
  let interactions = 0,
    song = null;
  const ray = new THREE.Raycaster(),
    pointer = new THREE.Vector2(),
    sample = new THREE.Vector3();
  const events = new AbortController(),
    options = { signal: events.signal, capture: true };
  let gesture = null,
    active = false,
    frame = 0,
    context = null,
    silence = null;

  function hitsCat(x, y) {
    if (!cat.visible || camera.position.distanceTo(cat.position) > 5.5) return false;
    if (x === null) pointer.set(0, 0);
    else {
      const box = canvas.getBoundingClientRect();
      pointer.set((2 * (x - box.left)) / box.width - 1, 1 - (2 * (y - box.top)) / box.height);
    }
    ray.setFromCamera(pointer, camera);
    cat.updateMatrixWorld(true);
    const hit = ray.intersectObject(cat, false)[0];
    if (!hit) return false;
    // The village's solid spans also keep clicks from passing through walls or the perch.
    for (let d = 0.1; d < hit.distance; d += 0.12) {
      ray.ray.at(d, sample);
      if (!collision.gap(sample.x, sample.z, -Infinity, sample.y, 0.025)) return false;
    }
    return true;
  }
  function stop() {
    cancelAnimationFrame(frame);
    frame = 0;
    active = false;
    silence?.();
    silence = null;
    cat.position.copy(home);
    cat.rotation.set(0, restYaw, 0);
    cat.scale.setScalar(1);
    shadow.material.opacity = 1;
  }

  function playRecording() {
    const audio = new Audio();
    song = audio;
    const ambient = tour.querySelector('noms-tour-soundtrack audio');
    const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    let lights = null;
    let dancers = null;
    let restoreAmbient = false;
    function finish(failed) {
      if (song !== audio) return;
      if (failed) interactions = 2;
      stop();
      redraw();
    }
    function animate() {
      const elapsed = audio.currentTime;
      const duration = Number.isFinite(audio.duration) ? audio.duration : 30;
      danceToRecording(cat, shadow, home, restYaw, elapsed, duration);
      dancers.update(elapsed, duration);
      lights.update(elapsed, duration, cat.position);
      redraw();
      frame = requestAnimationFrame(animate);
    }
    audio.onplaying = () => {
      if (ambient && !ambient.muted) {
        ambient.muted = true;
        restoreAmbient = true;
      }
      if (!reduced && !frame) {
        dancers = createCatDancers(scene, cat);
        lights = createCatPartyLights(scene, sky, home, camera);
        animate();
      }
    };
    audio.onended = () => finish(false);
    audio.onerror = () => finish(true);
    silence = () => {
      song = null;
      lights?.dispose();
      dancers?.dispose();
      audio.onplaying = audio.onended = audio.onerror = null;
      audio.pause();
      audio.removeAttribute('src');
      audio.load();
      if (restoreAmbient) ambient.muted = false;
    };
    audio.src = tour.dataset.catSongUrl;
    audio.play().catch(() => finish(true));
  }
  function play() {
    if (active || !cat.visible) return;
    active = true;
    interactions++;
    if (interactions === 3) {
      interactions = 0;
      playRecording();
      return;
    }
    try {
      context ??= new AudioContext();
      context.resume().catch(() => {});
      silence = chant(context);
    } catch {
      /* The visual routine also works when audio is unavailable. */
    }
    const start = performance.now();
    const turns = matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 2;
    function animate(now) {
      const t = Math.min(1, Math.max(0, (now - start) / (duration * 1000)));
      if (t === 1) stop();
      else {
        cat.rotation.y = restYaw + burstSpin(t * phrase.length, turns);
        frame = requestAnimationFrame(animate);
      }
      redraw();
    }
    frame = requestAnimationFrame(animate);
  }

  surface.addEventListener(
    'pointerdown',
    (e) => {
      if (!e.isPrimary || e.button !== 0 || !cat.visible) return;
      const locked = document.pointerLockElement === surface;
      gesture = hitsCat(locked ? null : e.clientX, e.clientY)
        ? { id: e.pointerId, x: e.clientX, y: e.clientY, locked }
        : null;
      // Keep this click on the cat; an ordinary click on the scenery still locks the mouse.
      if (gesture && !locked && e.pointerType === 'mouse') e.stopImmediatePropagation();
    },
    options
  );
  surface.addEventListener(
    'pointermove',
    (e) => {
      if (
        gesture?.id === e.pointerId &&
        !gesture.locked &&
        Math.hypot(e.clientX - gesture.x, e.clientY - gesture.y) > 8
      )
        gesture = null;
    },
    options
  );
  surface.addEventListener(
    'pointerup',
    (e) => {
      if (gesture?.id !== e.pointerId) return;
      const { locked } = gesture;
      gesture = null;
      if (hitsCat(locked ? null : e.clientX, e.clientY)) play();
    },
    options
  );
  surface.addEventListener(
    'pointercancel',
    () => {
      gesture = null;
    },
    options
  );
  document.addEventListener(
    'visibilitychange',
    () => {
      if (document.hidden) {
        gesture = null;
        stop();
      } else if (cat.visible) redraw();
    },
    options
  );
  return {
    update(walk) {
      cat.visible = shadow.visible = (walk?.[0] ?? 0) >= 0.999;
      if (!cat.visible) {
        gesture = null;
        if (active) stop();
      }
    },
    dispose() {
      events.abort();
      stop();
      context?.close().catch(() => {});
      cat.material.map.source.data?.close?.();
      cat.material.map.dispose();
      for (const mesh of [cat, shadow]) {
        mesh.removeFromParent();
        mesh.geometry.dispose();
        mesh.material.dispose();
      }
    }
  };
}
