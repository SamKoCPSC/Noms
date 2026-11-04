(function () {
"use strict";
let sequence = 0;
const NS = "http://www.w3.org/2000/svg";
const XL = "http://www.w3.org/1999/xlink";
const ease = x => { x = Math.max(0, Math.min(1, x)); return x * x * (3 - 2 * x); };
function element(tag, attributes = {}) {
  const node = document.createElementNS(NS, tag);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}

// Create once per inline rig and pass as renderers.dance. The player owns the
// clock; the invisible left-flipper track controls this mesh's shoulder angle.
async function create(svg) {
  const body = svg.querySelector('use[clip-path="url(#body-clip)"]');
  const control = svg.querySelector(".left-flipper");
  if (!body || !control) throw new Error("Unsupported Penguini dance rig.");
  const originalClip = body.getAttribute("clip-path");
  const artwork = new Image();
  artwork.src = svg.querySelector("#art").getAttributeNS(XL, "href");
  await artwork.decode();

  const surface = element("foreignObject", {x: 450, y: 150, width: 850, height: 800, class: "dance-body-mesh"});
  const canvas = document.createElementNS("http://www.w3.org/1999/xhtml", "canvas");
  canvas.width = 1700; canvas.height = 1600;
  canvas.style.cssText = "width:850px;height:800px;display:block;pointer-events:none";
  surface.append(canvas);
  const gl = canvas.getContext("webgl", {alpha: true, antialias: false, premultipliedAlpha: true, preserveDrawingBuffer: true});
  if (!gl) throw new Error("Penguini's articulated gestures require WebGL.");
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
        gl_Position = vec4((position.x - 450.) / 425. - 1., 1. - (position.y - 150.) / 400., 0., 1.);
      }`));
    gl.attachShader(program, shader(gl.FRAGMENT_SHADER, `
      #ifdef GL_FRAGMENT_PRECISION_HIGH
      precision highp float;
      #else
      precision mediump float;
      #endif
      varying vec2 coord;
      uniform sampler2D artwork;
      void main() {
        if (coord.x * 1536. + .6 * coord.y * 1024. > 1003. || coord.y * 1024. > 513.) discard;
        if (coord.x < 570./1536. || coord.x > 1220./1536. ||
            coord.y < 300./1024. || coord.y > 860./1024.) discard;
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
    for (const [key, value] of [[gl.TEXTURE_MIN_FILTER, gl.LINEAR], [gl.TEXTURE_MAG_FILTER, gl.LINEAR], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) {
      gl.texParameteri(gl.TEXTURE_2D, key, value);
    }
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, artwork);
    // Transparent texels must not erase overlapping parts of the folded flipper.
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.clearColor(0, 0, 0, 0); gl.viewport(0, 0, 1700, 1600);
  } catch (error) {
    gl.getExtension("WEBGL_lose_context")?.loseContext();
    throw error;
  }

  const columns = [...Array.from({length: 66}, (_, i) => 570 + i * 4), 900, 1000, 1120, 1220];
  const rows = [...Array.from({length: 56}, (_, i) => 300 + i * 4), 580, 660, 760, 860];
  const vertices = [], indices = [];
  for (const x of columns) for (const y of rows) {
    // Carry the underside into the shoulder so a lowered flipper does not fold
    // around a stationary patch and expose a rectangular background gap.
    // The fine mesh smooths that bend; zero influence at the torso and bow
    // keeps the seam aligned with the unchanged original artwork.
    const weight = ease((760 - x) / 100) * (1 - ease((y - 475) / 35)) * ease((995 - x - .6 * y) / 25);
    vertices.push({x, y, weight});
  }
  for (let i = 0; i < columns.length - 1; i++) for (let j = 0; j < rows.length - 1; j++) {
    const a = i * rows.length + j, b = (i + 1) * rows.length + j, c = b + 1, d = a + 1;
    indices.push(a, b, c, a, c, d);
  }
  const data = new Float32Array(indices.length * 4), pivot = [750, 400];
  let lastAngle = null, destroyed = false, active = false;

  // Only replace the flipper side of the stationary shoulder boundary. Leave
  // the torso and apron on the original SVG image, with a small seam overlap.
  const clipId = `penguini-dance-body-${++sequence}`;
  const defs = element("defs"), clip = element("clipPath", {id: clipId});
  clip.append(element("path", {"clip-rule": "evenodd", d: "M570 300H1220V860H570Z M570 300L820 300L694 510H570Z"}));
  defs.append(clip); svg.append(defs);
  surface.style.display = "none";
  body.parentNode.insertBefore(surface, body);

  // Expression overlays share the flipper renderer and the player's clock.
  // Preserve the normal eyes while switching to a squeeze or relaxed closure.
  const head = svg.querySelector(".head"), eyes = svg.querySelector(".eyes-open");
  const originalVisibility = eyes.style.visibility;
  const closed = element("g", {class: "gesture-closed-eyes", fill: "none", stroke: "#392519", "stroke-width": 11, "stroke-linecap": "round", "stroke-linejoin": "round"});
  const leftEye = element("path"), rightEye = element("path");
  closed.append(leftEye, rightEye); closed.style.display = "none"; head.append(closed);
  const air = element("g", {class: "sneeze-air", fill: "none", stroke: "#a98e70", "stroke-width": 5, "stroke-linecap": "round"});
  air.append(element("path", {d: "M4 -8 Q24 -14 38 -5"}), element("path", {d: "M10 7 Q38 5 53 24"}), element("path", {d: "M2 22 Q25 28 29 43"}));
  air.style.display = "none"; head.append(air);
  let hiddenEyes = false;
  function showEyes(visible) {
    if (visible === !hiddenEyes) return;
    hiddenEyes = !visible;
    eyes.style.visibility = visible ? originalVisibility : "hidden";
  }
  function face(state) {
    const t = state.elapsedMs, sneeze = state.animation === "sneeze", yawn = state.animation === "yawn";
    const start = sneeze ? 1980 : 1900, end = sneeze ? 2660 : 4700;
    const shut = (sneeze || yawn) && t >= start && t <= end;
    showEyes(!shut); closed.style.display = shut ? "" : "none";
    if (shut) {
      const depth = ease((t - start) / 160) * (1 - ease((t - (end - 140)) / 140));
      if (sneeze) {
        leftEye.setAttribute("d", `M242 ${550 - 15 * depth} L273 550 L242 ${550 + 15 * depth}`);
        rightEye.setAttribute("d", `M456 ${541 - 15 * depth} L425 541 L456 ${541 + 15 * depth}`);
      } else {
        leftEye.setAttribute("d", `M241 551 Q265 ${551 + 10 * depth} 289 551`);
        rightEye.setAttribute("d", `M409 541 Q433 ${541 + 10 * depth} 457 541`);
      }
    }
    if (sneeze && t >= 2140 && t < 2500) {
      const f = (t - 2140) / 360;
      air.style.display = "";
      air.style.opacity = .65 * ease(f / .18) * (1 - ease((f - .18) / .82));
      air.setAttribute("transform", `translate(${398 + 55 * f} ${626 + 25 * f}) rotate(15) scale(${.5 + 1.2 * f})`);
    } else air.style.display = "none";
  }

  function pose(angle) {
    if (angle === lastAngle) return;
    lastAngle = angle;
    const points = vertices.map(point => {
      const a = angle * point.weight, c = Math.cos(a), s = Math.sin(a);
      const x = point.x - pivot[0], y = point.y - pivot[1];
      return [pivot[0] + c * x - s * y, pivot[1] + s * x + c * y];
    });
    indices.forEach((index, i) => data.set([points[index][0], points[index][1], vertices[index].x / 1536, vertices[index].y / 1024], i * 4));
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer); gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
    gl.clear(gl.COLOR_BUFFER_BIT); gl.drawArrays(gl.TRIANGLES, 0, indices.length);
  }
  function resetFlipper() {
    surface.style.display = "none";
    if (active) { body.setAttribute("clip-path", originalClip); active = false; }
  }
  function reset() {
    resetFlipper(); showEyes(true);
    closed.style.display = "none"; air.style.display = "none";
  }
  function draw(state) {
    if (destroyed) return;
    if (state.phase !== "activity" || state.reducedMotion || gl.isContextLost()) { reset(); return; }
    face(state);
    const matrix = new DOMMatrix(getComputedStyle(control).transform);
    const angle = Math.atan2(matrix.b, matrix.a);
    if (Math.abs(angle) < 1e-7) { resetFlipper(); return; }
    pose(angle);
    if (!active) { body.setAttribute("clip-path", `url(#${clipId})`); active = true; }
    surface.style.display = "";
  }
  function destroy() {
    if (destroyed) return;
    reset(); destroyed = true;
    surface.remove(); defs.remove(); closed.remove(); air.remove();
    gl.deleteBuffer(buffer); gl.deleteTexture(texture); gl.deleteProgram(program);
    shaders.forEach(item => gl.deleteShader(item));
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  }
  return {draw, reset, destroy};
}
window.PenguiniDanceRig = {create};
})();
