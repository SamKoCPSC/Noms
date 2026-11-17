export async function loadVillage(canvas, catalog, bridge) {
  const { createVillage } = await import(bridge);
  return createVillage(canvas, JSON.parse(catalog));
}
export async function readyVillage(view) { return view.ready; }
export function drawVillage(view, progress, width, height, slot, openingDistance, look) { return view.draw(progress, width, height, Array.from(slot), openingDistance, look); }
export function disposeVillage(view) { view.dispose(); }
