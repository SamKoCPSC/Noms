export async function loadVillage(canvas, catalog, bridge) {
  const { createVillage } = await import(bridge);
  return createVillage(canvas, JSON.parse(catalog));
}
export async function readyVillage(view) { return view.ready; }
export function drawVillage(view, progress, width, height, slot, openingDistance) { return view.draw(progress, width, height, Array.from(slot), openingDistance); }
export function disposeVillage(view) { view.dispose(); }
