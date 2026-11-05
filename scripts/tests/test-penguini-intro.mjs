import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import vm from "node:vm";
import test from "node:test";

// Exercise the real clips and player clock without requiring a GPU/browser.
function environment(reduced = false) {
  const frames = new Map();
  let frameId = 0;
  const media = new EventTarget();
  media.matches = reduced;
  const document = new EventTarget();
  document.hidden = false;
  class SVGElement extends EventTarget {
    constructor() {
      super();
      this.parts = new Map();
    }
    querySelector(selector) {
      if (!this.parts.has(selector)) this.parts.set(selector, {
        style: {},
        animate() { return {pause() {}, cancel() {}, currentTime: 0}; },
      });
      return this.parts.get(selector);
    }
  }
  const observers = [];
  class IntersectionObserver {
    constructor(callback) { this.callback = callback; observers.push(this); }
    observe() {}
    disconnect() { this.disconnected = true; }
  }
  const context = vm.createContext({
    window: {}, document, SVGElement, IntersectionObserver, CustomEvent,
    structuredClone, matchMedia: () => media,
    getComputedStyle: element => ({transform: element.style.transform ?? "none", opacity: element.style.opacity ?? "1"}),
    requestAnimationFrame: callback => { frames.set(++frameId, callback); return frameId; },
    cancelAnimationFrame: id => frames.delete(id),
  });
  for (const file of ["clips.js", "player.js"]) {
    vm.runInContext(readFileSync(new URL(`../../assets/brand/penguini/${file}`, import.meta.url), "utf8"), context);
  }
  const clips = context.window.PenguiniClips;
  const svg = new SVGElement();
  const make = (options = {}) => new context.window.PenguiniPlayer(svg, {
    idle: clips.idle, animations: [clips.excited, clips["shy-hello"]],
    initialAnimation: "excited", random: () => .75, ...options,
  });
  return {make, svg, frames, media, document, observers};
}

test("excited starts immediately, completes once, then returns to the normal idle/activity loop", () => {
  const {make} = environment();
  const player = make();
  assert.equal(player.state.animation, "excited");
  assert.equal(player.state.elapsedMs, 0);
  player.advanceBy(6599);
  assert.equal(player.state.animation, "excited");
  player.advanceBy(1);
  assert.equal(player.state.animation, "idle");
  assert.equal(player.state.phase, "transition");
  player.advanceBy(240);
  assert.equal(player.state.phase, "idle");
  player.advanceBy(4000 + 240);
  assert.equal(player.state.animation, "shy-hello");
  assert.equal(player.state.phase, "activity");
  player.advanceBy(5200 + 240);
  assert.equal(player.state.phase, "idle");
  player.destroy();
});

test("the default player still starts in idle and invalid intro IDs fail explicitly", () => {
  const {make} = environment();
  const player = make({initialAnimation: null});
  assert.equal(player.state.animation, "idle");
  player.destroy();
  assert.throws(() => make({initialAnimation: "missing"}), /must be registered/);
});

test("reduced motion suppresses the excited intro and freezes the resting pose", () => {
  const {make, frames, media, svg} = environment(true);
  const player = make();
  assert.equal(player.state.animation, "idle");
  assert.equal(frames.size, 0);
  player.advanceBy(20000);
  assert.equal(player.state.elapsedMs, 0);
  assert.equal(svg.querySelector(".eyes-happy").style.opacity, "0");
  media.matches = false;
  media.dispatchEvent(new Event("change"));
  assert.equal(frames.size, 1);
  player.destroy();
});

test("pause, hidden tabs, offscreen states and destruction stop scheduled animation work", () => {
  const {make, frames, document, observers} = environment();
  const player = make();
  assert.equal(frames.size, 1);
  player.pause();
  assert.equal(frames.size, 0);
  player.play();
  document.hidden = true;
  document.dispatchEvent(new Event("visibilitychange"));
  assert.equal(frames.size, 0);
  document.hidden = false;
  document.dispatchEvent(new Event("visibilitychange"));
  observers[0].callback([{isIntersecting: false}]);
  assert.equal(frames.size, 0);
  observers[0].callback([{isIntersecting: true}]);
  assert.equal(frames.size, 1);
  player.destroy();
  assert.equal(frames.size, 0);
  assert.equal(observers[0].disconnected, true);
  document.dispatchEvent(new Event("visibilitychange"));
  assert.equal(frames.size, 0);
});
