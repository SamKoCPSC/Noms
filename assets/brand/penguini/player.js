(function () {
"use strict";
// Animation definitions are registered independently of the playback loop.
// Scope one player to one inline SVG; no global selectors or network requests.
class PenguiniPlayer {
  constructor(svg, {idle, animations = [], idleDurationMs = 4000, transitionMs = 240, autoplay = true, random = Math.random} = {}) {
    if (!(svg instanceof SVGElement)) throw new TypeError('An inline SVG is required.');
    if (!Number.isFinite(idleDurationMs) || idleDurationMs <= 0 || !Number.isFinite(transitionMs) || transitionMs <= 0) throw new TypeError('Durations must be positive.');
    this.svg = svg;
    this.idle = {...idle, id: 'idle', durationMs: idleDurationMs};
    this.transitionMs = transitionMs;
    this.random = random;
    this.registry = new Map();
    this.active = [];
    this.blends = [];
    this.userPaused = !autoplay;
    this.offscreen = false;
    this.destroyed = false;
    this.raf = null;
    this.lastTime = null;
    this.motion = matchMedia('(prefers-reduced-motion: reduce)');
    this.parts = ['.penguin','.head','.flipper','.shadow','.eyes-open','.eyes-happy','.blink','.lower-beak','.front-arm','.front-flipper','.kiss-spark','.back-arm'].map(selector => ({selector, element: svg.querySelector(selector)}));
    if (this.parts.some(p => !p.element)) throw new TypeError('The mascot rig is incomplete.');
    const origins = ['580px 990px','350px 740px','1305px 610px','580px 1000px',null,null,'350px 550px','355px 610px',null,'1305px 610px','1020px 330px',null];
    this.parts.forEach((p,i) => { if (origins[i]) p.element.style.transformOrigin = origins[i]; });
    this._validate(this.idle);
    for (const clip of animations) this.registerAnimation(clip);
    this._begin(this.idle);
    this.onVisibility = () => this._sync();
    this.onMotion = () => { this._begin(this.idle); this._sync(); };
    document.addEventListener('visibilitychange', this.onVisibility);
    this.motion.addEventListener('change', this.onMotion);
    this.observer = new IntersectionObserver(entries => { this.offscreen = !entries[0].isIntersecting; this._sync(); });
    this.observer.observe(svg);
    this._sync();
  }
  _validate(clip) {
    if (!clip || typeof clip.id !== 'string' || !clip.id || !Number.isFinite(clip.durationMs) || clip.durationMs <= 0 || !Array.isArray(clip.tracks) || !clip.tracks.length) throw new TypeError('A clip requires an id, positive durationMs and tracks.');
    const seen = new Set();
    for (const track of clip.tracks) {
      if (!this.parts.some(p => p.selector === track.selector) || seen.has(track.selector) || !Array.isArray(track.keyframes) || !track.keyframes.length) throw new TypeError('Tracks require unique rig selectors and keyframes.');
      if (track.durationMs !== undefined && (!Number.isFinite(track.durationMs) || track.durationMs <= 0)) throw new TypeError('Track durations must be positive.');
      for (const frame of track.keyframes) if (Object.keys(frame).some(k => !['transform','opacity','offset','easing'].includes(k))) throw new TypeError('Only transform and opacity are animated.');
      seen.add(track.selector);
    }
  }
  registerAnimation(clip) {
    if (this.destroyed) throw new Error('Player has been destroyed.');
    this._validate(clip);
    if (clip.id === 'idle' || this.registry.has(clip.id)) throw new Error('Animation id already exists or is reserved.');
    this.registry.set(clip.id, structuredClone(clip));
    return this;
  }
  get state() { return {phase:this.phase, animation:this.clip.id, elapsedMs:this.elapsed, paused:this.userPaused, reducedMotion:this.motion.matches}; }
  pause() { this.userPaused = true; this._sync(); }
  play() { if (!this.destroyed) { this.userPaused = false; this._sync(); } }
  _clear() {
    for (const a of [...this.blends,...this.active]) a.cancel();
    this.blends = []; this.active = [];
  }
  _rest() {
    for (const {element} of this.parts) { element.style.transform = 'none'; element.style.opacity = '1'; }
    this.svg.querySelector('.flipper').style.transform = 'rotate(88deg)';
    this.svg.querySelector('.eyes-happy').style.opacity = '0';
    this.svg.querySelector('.lower-beak').style.transform = 'scaleY(.45)';
    this.svg.querySelector('.shadow').style.opacity = '.13';
    this.svg.querySelector('.front-arm').style.opacity = '0';
    this.svg.querySelector('.front-flipper').style.transform = 'rotate(88deg)';
    this.svg.querySelector('.kiss-spark').style.opacity = '0';
  }
  _install(clip) {
    this._clear(); this._rest();
    this.clip = clip; this.elapsed = 0;
    this.phase = clip.id === 'idle' ? 'idle' : 'activity';
    this.duration = clip.durationMs;
    this.active = clip.tracks.map(track => {
      const a = this.svg.querySelector(track.selector).animate(track.keyframes, {
        duration:track.durationMs ?? clip.durationMs,
        easing:track.easing ?? 'linear',
        iterations:clip.id === 'idle' ? Infinity : 1,
        fill:'both'
      });
      a.pause(); a.currentTime = 0;
      return a;
    });
  }
  _begin(clip) { this._install(clip); this._emit(); }
  _pose() { return this.parts.map(({element}) => { const s = getComputedStyle(element); return {transform:s.transform,opacity:s.opacity}; }); }
  _transition(clip) {
    const from = this._pose();
    this._install(clip);
    const to = this._pose();
    this.phase = 'transition'; this.duration = this.transitionMs;
    this.blends = this.parts.map(({element,selector},i) => {
      // Switch eye expressions together; fading them overlaps two eye shapes.
      const easing = selector.startsWith('.eyes-') ? 'steps(1,end)' : 'ease-in-out';
      const a = element.animate([from[i],to[i]], {duration:this.transitionMs,easing,fill:'both'});
      a.pause(); a.currentTime = 0; return a;
    });
    this._emit();
  }
  _next() {
    if (this.phase === 'transition') {
      for (const a of this.blends) a.cancel();
      this.blends = []; this.phase = this.clip.id === 'idle' ? 'idle' : 'activity';
      this.elapsed = 0; this.duration = this.clip.durationMs; this._emit();
    } else if (this.phase === 'idle' && this.registry.size) {
      const choices = [...this.registry.values()];
      this._transition(choices[Math.min(choices.length - 1, Math.max(0, Math.floor(this.random() * choices.length)))]);
    } else if (this.phase === 'idle') {
      this._begin(this.idle);
    } else this._transition(this.idle);
  }
  // Deterministic advancement also supports offline recording and integration tests.
  advanceBy(ms) {
    if (this.destroyed || this.motion.matches) return;
    if (!Number.isFinite(ms) || ms < 0) throw new TypeError('Time must be finite and nonnegative.');
    while (ms > 0) {
      const dt = Math.min(ms, this.duration - this.elapsed);
      this.elapsed += dt; ms -= dt;
      for (const a of this.phase === 'transition' ? this.blends : this.active) a.currentTime = this.elapsed;
      if (this.elapsed >= this.duration) this._next();
    }
  }
  _emit() { this.svg.dispatchEvent(new CustomEvent('mascotphasechange', {detail:this.state})); }
  _sync() {
    if (this.raf !== null) cancelAnimationFrame(this.raf);
    this.raf = null; this.lastTime = null;
    if (this.destroyed || this.userPaused || this.offscreen || document.hidden || this.motion.matches) return;
    const tick = now => {
      if (this.lastTime !== null) this.advanceBy(Math.min(now - this.lastTime, 100));
      this.lastTime = now;
      this.raf = requestAnimationFrame(tick);
    };
    this.raf = requestAnimationFrame(tick);
  }
  destroy() {
    this.destroyed = true; this._sync(); this._clear(); this._rest();
    this.observer.disconnect();
    document.removeEventListener('visibilitychange', this.onVisibility);
    this.motion.removeEventListener('change', this.onMotion);
  }
}
window.PenguiniPlayer = PenguiniPlayer;

})();
