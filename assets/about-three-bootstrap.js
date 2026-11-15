// A narrow wasm-bindgen boundary; Rust owns all tour state and camera transforms.
export async function loadTour(canvas, assets, initial, bridgeUrl) {
  const { createTour } = await import(bridgeUrl);
  return createTour(canvas, JSON.parse(assets), initial);
}
export function beginTour(view, progress, width, height) { return view.begin(progress, width, height); }
export function hasTourScene(view, index) { return view.has(index); }
export function drawTour(view, index, model, vp, cameraView, visibility) { return view.draw(index, model, vp, cameraView, visibility); }
export function hideTour(view) { view.hide(); }
export function disposeTour(view) { view.dispose(); }
