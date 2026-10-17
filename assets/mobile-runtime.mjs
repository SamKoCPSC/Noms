// Keep action bars reachable above the visual keyboard. Rust owns form state.
export function mountMobileViewport() {
  window.nomsMobileViewportCleanup?.();
  const root = document.documentElement, viewport = window.visualViewport;
  let frame = 0;
  const update = () => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      const editing = document.activeElement?.matches('input:not([type=checkbox]):not([type=radio]):not([type=range]), textarea, select');
      const offset = viewport ? Math.max(0, window.innerHeight - viewport.height - viewport.offsetTop) : 0;
      const open = window.innerWidth <= 760 && editing && offset > 120;
      root.dataset.mobileKeyboard = open ? 'open' : 'closed';
      root.style.setProperty('--mobile-keyboard-offset', open ? `${offset}px` : '0px');
    });
  };
  const selectSection = event => {
    const anchor = event.target.closest?.('.recipe-editor__contents a[href^="#recipe-"]');
    if (!anchor || window.innerWidth > 760) return;
    requestAnimationFrame(() => {
      const heading = document.getElementById(anchor.hash.slice(1));
      heading?.focus({ preventScroll: true });
      heading?.scrollIntoView({ block: 'start', behavior: 'instant' });
    });
  };
  viewport?.addEventListener('resize', update); viewport?.addEventListener('scroll', update);
  window.addEventListener('resize', update); document.addEventListener('focusin', update);
  document.addEventListener('focusout', update); document.addEventListener('click', selectSection); update();
  window.nomsMobileViewportCleanup = () => {
    cancelAnimationFrame(frame); viewport?.removeEventListener('resize', update); viewport?.removeEventListener('scroll', update);
    window.removeEventListener('resize', update); document.removeEventListener('focusin', update);
    document.removeEventListener('focusout', update); document.removeEventListener('click', selectSection);
    delete root.dataset.mobileKeyboard; root.style.removeProperty('--mobile-keyboard-offset');
  };
}
