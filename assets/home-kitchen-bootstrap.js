// Narrow wasm-bindgen boundary; all dependencies are bundled and self-hosted.
export async function loadKitchen(canvas, model, lighting, bridge, signal) {
  const {createKitchen} = await import(bridge);
  signal.throwIfAborted();
  return createKitchen(canvas, model, lighting, signal);
}
export function pauseKitchen(view, paused) { view.setPaused(paused); }
export function disposeKitchen(view) { view.dispose(); }
