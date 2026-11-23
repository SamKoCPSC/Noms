// Keep below-the-fold model downloads out of the hero's initial load.
async function nearViewport(canvas, signal) {
  if (!window.IntersectionObserver) return;
  await new Promise((resolve, reject) => {
    const observer = new IntersectionObserver(entries => {
      if (entries.some(entry => entry.isIntersecting)) {
        observer.disconnect();
        signal.removeEventListener('abort', cancel);
        resolve();
      }
    }, {rootMargin: '120px'});
    function cancel() {
      observer.disconnect();
      reject(signal.reason);
    }
    signal.throwIfAborted();
    signal.addEventListener('abort', cancel, {once: true});
    observer.observe(canvas);
  });
}

export async function loadScene(canvas, signal) {
  const {scene, modelUrl, lightingUrl, rendererUrl} = canvas.closest('figure').dataset;
  if (scene !== 'kitchen') await nearViewport(canvas, signal);
  signal.throwIfAborted();
  const {createScene} = await import(rendererUrl);
  signal.throwIfAborted();
  return createScene(canvas, scene, modelUrl, lightingUrl, signal);
}
export function pauseScene(view, paused) { view.setPaused(paused); }
export function disposeScene(view) { view.dispose(); }
