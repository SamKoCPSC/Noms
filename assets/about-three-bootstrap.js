// A narrow wasm-bindgen boundary; the bundled Three.js module is loaded on demand.
export async function loadKitchen(canvas, modelUrl, lightingUrl, bridgeUrl) {
  const { createKitchen } = await import(bridgeUrl);
  return createKitchen(canvas, modelUrl, lightingUrl);
}
export function drawKitchen(view, model, vp, cameraView, width, height, visibility) {
  return view.draw(model, vp, cameraView, width, height, visibility);
}
export function hideKitchen(view) { view.hide(); }
export function disposeKitchen(view) { view.dispose(); }
