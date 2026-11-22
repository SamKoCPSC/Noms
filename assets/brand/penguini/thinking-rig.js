(function () {
"use strict";
let sequence = 0;
const NS = "http://www.w3.org/2000/svg";
const XL = "http://www.w3.org/1999/xlink";
const clamp = x => Math.max(0, Math.min(1, x));
const ease = x => { x = clamp(x); return x * x * (3 - 2 * x); };
const soft = x => { x = clamp(x); return x * x * x * (10 + x * (-15 + 6 * x)); };
const mix = (a, b, t) => a + (b - a) * t;
const radians = degrees => degrees * Math.PI / 180;
function element(tag, attributes = {}) {
  const node = document.createElementNS(NS, tag);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}
function bone(angle, tx, ty) {
  const c = Math.cos(angle / 2), s = Math.sin(angle / 2);
  return {c, s, dx: (tx * c + ty * s) / 2, dy: (-tx * s + ty * c) / 2};
}

// Create once per inline rig, await readiness, then pass as renderers.thinking.
// The player owns the clock; this renderer has no independent animation loop.
async function create(svg) {
  const rear = svg.querySelector(".back-arm");
  const penguin = svg.querySelector(".penguin");
  const head = svg.querySelector(".head").parentNode;
  const flipper = svg.querySelector(".flipper");
  const eyes = [...svg.querySelectorAll(".blink > ellipse")];
  const highlights = [...svg.querySelectorAll(".blink > g > ellipse")];
  if (eyes.length !== 2 || highlights.length !== 2) throw new Error("Unsupported Penguini eye rig.");
  const artwork = new Image();
  artwork.src = svg.querySelector("#art").getAttributeNS(XL, "href");
  await artwork.decode();

  const group = element("foreignObject", {class: "thinking-arm-mesh", x: 0, y: 0, width: 1200, height: 1060});
  const canvas = document.createElementNS("http://www.w3.org/1999/xhtml", "canvas");
  canvas.width = 2400; canvas.height = 2120;
  canvas.style.cssText = "width:1200px;height:1060px;display:block;pointer-events:none";
  group.append(canvas);
  const gl = canvas.getContext("webgl", {alpha: true, antialias: false, premultipliedAlpha: true, preserveDrawingBuffer: true});
  if (!gl) throw new Error("Penguini's thinking animation requires WebGL.");
  const shaders = [];
  function shader(type, source) {
    const result = gl.createShader(type);
    shaders.push(result);
    gl.shaderSource(result, source); gl.compileShader(result);
    if (!gl.getShaderParameter(result, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(result));
    return result;
  }
  let program, buffer, texture;
  try {
    program = gl.createProgram();
    gl.attachShader(program, shader(gl.VERTEX_SHADER, `
      attribute vec2 position;
      attribute vec2 uv;
      varying vec2 coord;
      void main() {
        coord = uv;
        gl_Position = vec4(position.x / 600. - 1., 1. - position.y / 530., 0., 1.);
      }`));
    gl.attachShader(program, shader(gl.FRAGMENT_SHADER, `
      precision mediump float;
      varying vec2 coord;
      uniform sampler2D artwork;
      void main() {
        if (coord.x < 1240./1536. || coord.x > 1530./1536. ||
            coord.y < 330./1024. || coord.y > 680./1024.) discard;
        vec4 texel = texture2D(artwork, coord);
        gl_FragColor = vec4(texel.rgb * texel.a, texel.a);
      }`));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
    gl.useProgram(program);
    buffer = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    for (const [name, offset] of [["position", 0], ["uv", 8]]) {
      const location = gl.getAttribLocation(program, name);
      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(location, 2, gl.FLOAT, false, 16, offset);
    }
    texture = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, artwork);
    // Transparent texels must not erase overlapping parts of the folded arm.
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.clearColor(0, 0, 0, 0); gl.viewport(0, 0, 2400, 2120);
  } catch (error) {
    gl.getExtension("WEBGL_lose_context")?.loseContext();
    throw error;
  }

  const prefix = `penguini-thinking-${++sequence}`;
  const defs = element("defs");
  const lids = ["M235 510 L295 510 L295 590 L235 590Z", "M403 500 L463 500 L463 580 L403 580Z"];
  const paths = lids.map((d, index) => {
    const clip = element("clipPath", {id: `${prefix}-${index}`});
    const path = element("path", {d}); clip.append(path); defs.append(clip); return path;
  });
  const originalClips = [...eyes, ...highlights].map(node => [node, node.getAttribute("clip-path")]);
  let masksActive = false;
  function masks(enabled) {
    if (enabled === masksActive) return;
    masksActive = enabled;
    if (enabled) {
      [eyes, highlights].forEach(pair => pair.forEach((node, index) => node.setAttribute("clip-path", `url(#${prefix}-${index})`)));
    } else {
      for (const [node, clip] of originalClips) {
        if (clip === null) node.removeAttribute("clip-path"); else node.setAttribute("clip-path", clip);
      }
    }
  }
  const brows = element("g", {class: "thinking-brows", opacity: 0, fill: "none", stroke: "#493125", "stroke-width": 5, "stroke-linecap": "round"});
  brows.style.display = "none";
  brows.append(element("path", {d: "M245 516 Q264 514 282 523"}), element("path", {d: "M413 511 Q432 503 450 502"}));
  svg.append(defs); svg.querySelector(".eyes-open").append(brows);
  penguin.insertBefore(group, rear); group.style.display = "none";

  const sourceRoot = [1305, 610], shoulder = [745, 540];
  const axis = Math.atan2(-235, 185), upperLength = 140, lowerLength = 145;
  const goal = [625, 590], dx = goal[0] - shoulder[0], dy = goal[1] - shoulder[1];
  const folded = Math.acos((dx * dx + dy * dy - upperLength ** 2 - lowerLength ** 2) / (2 * upperLength * lowerLength));
  const raised = Math.atan2(dy, dx) - Math.atan2(lowerLength * Math.sin(folded), upperLength + lowerLength * Math.cos(folded));
  const columns = [-60,-40,-20,0,20,40,60,80,100,110,120,130,140,150,160,170,180,200,220,240,260,280,300,320,340];
  const rows = [-90,-60,-30,0,30,60,90], vertices = [], indices = [];
  for (const u of columns) for (const v of rows) vertices.push({
    u, v, weight: ease((u - 110) / 60),
    x: sourceRoot[0] + Math.cos(axis) * u - Math.sin(axis) * v,
    y: sourceRoot[1] + Math.sin(axis) * u + Math.cos(axis) * v,
  });
  for (let i = 0; i < columns.length - 1; i++) for (let j = 0; j < rows.length - 1; j++) {
    const a = i * rows.length + j, b = (i + 1) * rows.length + j, c = b + 1, d = a + 1;
    indices.push(a, b, c, a, c, d);
  }
  const data = new Float32Array(indices.length * 4);
  // Approved low reach and gentle easing, with a 25% faster lift and recovery.
  const armSpeed = 1.25, liftStart = 350, liftDuration = 650 / armSpeed;
  const foldStart = liftStart + liftDuration, foldDuration = 950 / armSpeed;
  const relaxStart = 4700, relaxDuration = 1050 / armSpeed;
  const lowerStart = relaxStart + relaxDuration, lowerDuration = 650 / armSpeed;
  let destroyed = false, active = false, inFront = false, lastPose = "", savedOpacity;

  function mesh(upper, bend, lift) {
    const key = [upper, bend, lift].map(value => value.toFixed(8)).join(",");
    if (key === lastPose) return;
    lastPose = key;
    const root = [shoulder[0], shoulder[1] - lift], lower = upper + bend;
    const elbow = [root[0] + upperLength * Math.cos(upper), root[1] + upperLength * Math.sin(upper)];
    const a = bone(upper, ...root);
    const b = bone(lower, elbow[0] - upperLength * Math.cos(lower), elbow[1] - upperLength * Math.sin(lower));
    // Dual-quaternion skinning preserves thickness at the elbow.
    const posed = vertices.map(point => {
      let c = mix(a.c, b.c, point.weight), s = mix(a.s, b.s, point.weight);
      let x = mix(a.dx, b.dx, point.weight), y = mix(a.dy, b.dy, point.weight);
      const norm = Math.hypot(c, s); c /= norm; s /= norm; x /= norm; y /= norm;
      const cosine = c * c - s * s, sine = 2 * c * s;
      return [2 * (x * c - y * s) + cosine * point.u - sine * point.v,
              2 * (x * s + y * c) + sine * point.u + cosine * point.v];
    });
    indices.forEach((index, i) => data.set([posed[index][0], posed[index][1], vertices[index].x / 1536, vertices[index].y / 1024], i * 4));
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer); gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
    gl.clear(gl.COLOR_BUFFER_BIT); gl.drawArrays(gl.TRIANGLES, 0, indices.length);
  }
  function hideArm() {
    group.style.display = "none";
    if (active) { rear.style.opacity = savedOpacity; active = false; }
  }
  function reset() {
    hideArm();
    masks(false);
    paths.forEach((path, index) => path.setAttribute("d", lids[index]));
    brows.style.opacity = "0";
    brows.style.display = "none";
  }
  function draw(state) {
    if (destroyed) return;
    if (state.phase !== "activity" || state.animation !== "thinking" || state.reducedMotion || gl.isContextLost()) { reset(); return; }
    const t = state.elapsedMs;
    const clearance = soft((t - liftStart) / liftDuration) * (1 - soft((t - lowerStart) / lowerDuration));
    const bend = soft((t - foldStart) / foldDuration) * (1 - soft((t - relaxStart) / relaxDuration));
    const squint = ease((t - 850) / 950) * (1 - ease((t - 4850) / 1250));
    masks(squint > 0);
    if (t > liftStart && t < lowerStart + lowerDuration) {
      const front = t >= foldStart && t < lowerStart;
      if (front !== inFront) { inFront = front; penguin.insertBefore(group, front ? head : rear); }
      const transform = new DOMMatrix(getComputedStyle(flipper).transform);
      const idleAngle = axis + Math.atan2(transform.b, transform.a);
      mesh(mix(mix(idleAngle, radians(18), clearance), raised, bend), folded * bend, 60 * clearance * (1 - bend));
      if (!active) { savedOpacity = rear.style.opacity; active = true; }
      rear.style.opacity = "0"; group.style.display = "";
    } else hideArm();
    paths[0].setAttribute("d", `M235 ${mix(510,535,squint)} L295 ${mix(510,543,squint)} L295 590 L235 590Z`);
    paths[1].setAttribute("d", `M403 ${mix(500,529,squint)} L463 ${mix(500,522,squint)} L463 580 L403 580Z`);
    brows.style.opacity = squint * .55;
    brows.style.display = squint > 0 ? "" : "none";
  }
  function destroy() {
    if (destroyed) return;
    reset(); destroyed = true;
    group.remove(); defs.remove(); brows.remove();
    gl.deleteBuffer(buffer); gl.deleteTexture(texture); gl.deleteProgram(program);
    shaders.forEach(item => gl.deleteShader(item));
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  }
  return {draw, reset, destroy};
}
window.PenguiniThinkingRig = {create};
})();
