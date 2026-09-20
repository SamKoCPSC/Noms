// DOM-only tooltip lifetime. No page backdrop, focus transfer or interactive tip.
const tooltips = new Map();

export function tooltipPosition(trigger, width, height, viewportWidth, viewportHeight) {
    const left = Math.max(8, Math.min((trigger.left + trigger.right - width) / 2, viewportWidth - width - 8));
    const below = trigger.bottom + 8;
    const top = below + height <= viewportHeight - 8 ? below : Math.max(8, trigger.top - height - 8);
    return {left, top};
}

export function mountTooltip(id) {
    if (tooltips.has(id)) return;
    const root = document.getElementById(id);
    const button = root?.querySelector('button');
    const tip = root?.querySelector('[role="tooltip"]');
    if (!button || !tip) return;
    const controller = new AbortController();
    let hovered = false, focused = false, dismissed = false, timer;
    const position = () => {
        if (tip.hidden) return;
        const at = tooltipPosition(button.getBoundingClientRect(), tip.offsetWidth, tip.offsetHeight, innerWidth, innerHeight);
        tip.style.left = `${at.left}px`;
        tip.style.top = `${at.top}px`;
    };
    const hide = () => { if (typeof tip.hidePopover === 'function' && tip.matches(':popover-open')) tip.hidePopover(); tip.hidden = true; };
    const show = () => {
        clearTimeout(timer);
        if (dismissed || button.disabled) return;
        tip.hidden = false;
        if (typeof tip.showPopover === 'function') {
            tip.setAttribute('popover', 'manual');
            if (!tip.matches(':popover-open')) tip.showPopover();
        }
        position();
    };
    const leave = () => {
        clearTimeout(timer);
        // The small grace period lets the pointer cross the gap into the tip.
        timer = setTimeout(() => { if (!hovered && !focused) { hide(); dismissed = false; } }, 150);
    };
    const listen = (node, name, fn, extra = {}) => node.addEventListener(name, fn, {signal:controller.signal, ...extra});
    listen(root, 'mouseenter', () => { hovered = true; show(); });
    listen(root, 'mouseleave', () => { hovered = false; leave(); });
    listen(tip, 'mouseenter', () => { hovered = true; clearTimeout(timer); });
    listen(tip, 'mouseleave', () => { hovered = false; leave(); });
    listen(button, 'focus', () => { focused = true; show(); });
    listen(button, 'blur', () => { focused = false; leave(); });
    listen(document, 'keydown', event => {
        if (event.key === 'Escape' && !tip.hidden) { dismissed = true; hide(); event.preventDefault(); }
    });
    listen(window, 'resize', position);
    listen(document, 'scroll', position, {capture:true});
    tooltips.set(id, () => { clearTimeout(timer); controller.abort(); hide(); });
}

export function disposeTooltip(id) { tooltips.get(id)?.(); tooltips.delete(id); }
