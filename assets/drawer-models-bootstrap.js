// Upgrade only a mounted navigation drawer. Its model renders are immediately
// available as posters while the shared Three.js bundle or a GLB is loading.
class NomsDrawerModels extends HTMLElement {
  connectedCallback() {
    const controller = new AbortController();
    this.controller = controller;
    import(this.dataset.renderer).then(module => {
      if (!controller.signal.aborted) return module.mountDrawerModels(this, controller.signal);
    }).catch(error => {
      if (!controller.signal.aborted) console.warn('Using drawer model posters', error);
    });
  }
  disconnectedCallback() {
    this.controller?.abort();
  }
}
if (!customElements.get('noms-drawer-models')) customElements.define('noms-drawer-models', NomsDrawerModels);
