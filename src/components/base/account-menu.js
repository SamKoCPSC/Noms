// P1 is nonmodal. Keep its DOM mounted so native logout POST is never interrupted.
const accountMenus = new Map();

export function accountMenuPosition(trigger, width, height, viewportWidth, viewportHeight) {
    const below = Math.max(0, viewportHeight - trigger.bottom - 16);
    const above = Math.max(0, trigger.top - 16);
    const opensAbove = height > below && above > below;
    const maxHeight = opensAbove ? above : below;
    return {
        left: Math.max(8, Math.min(trigger.right - width, viewportWidth - width - 8)),
        top: opensAbove ? Math.max(8, trigger.top - Math.min(height, maxHeight) - 8) : trigger.bottom + 8,
        maxHeight,
    };
}

export function mountAccountMenu(id) {
    if (accountMenus.has(id)) return;
    const root = document.getElementById(id);
    const trigger = root?.querySelector('[data-account-trigger]');
    const panel = root?.querySelector('[data-account-panel]');
    if (!trigger || !panel) return;
    const controller = new AbortController();
    const listen = (node, name, fn, extra = {}) => node.addEventListener(name, fn, {signal:controller.signal, ...extra});
    const items = () => [...panel.querySelectorAll('[role="menuitem"]')].filter(item => !item.disabled && item.getAttribute('aria-disabled') !== 'true');
    const close = (restore = false) => {
        if (typeof panel.hidePopover === 'function' && panel.matches(':popover-open')) panel.hidePopover();
        panel.hidden = true;
        trigger.setAttribute('aria-expanded', 'false');
        if (restore && trigger.isConnected) trigger.focus();
    };
    const position = () => {
        if (panel.hidden) return;
        // Closing at the desktop breakpoint avoids an orphaned top-layer menu.
        if (!trigger.getClientRects().length) { close(); return; }
        // Measure full content without temporarily enlarging the scroll container:
        // doing so during focus scrolling can leave the focused row clipped.
        const naturalHeight = panel.scrollHeight + panel.offsetHeight - panel.clientHeight;
        const at = accountMenuPosition(trigger.getBoundingClientRect(), panel.offsetWidth, naturalHeight, innerWidth, innerHeight);
        panel.style.left = `${at.left}px`;
        panel.style.top = `${at.top}px`;
        panel.style.maxHeight = `${at.maxHeight}px`;
    };
    const open = (last = false) => {
        panel.hidden = false;
        if (typeof panel.showPopover === 'function') {
            panel.setAttribute('popover', 'manual');
            if (!panel.matches(':popover-open')) panel.showPopover();
        }
        trigger.setAttribute('aria-expanded', 'true');
        position();
        if (!panel.hidden) (last ? items().at(-1) : items()[0])?.focus();
    };
    listen(trigger, 'click', () => panel.hidden ? open() : close(true));
    listen(trigger, 'keydown', event => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault(); open(event.key === 'ArrowUp');
        }
    });
    listen(panel, 'keydown', event => {
        if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(true); return; }
        // Resume natural page tab order from the trigger; never trap focus.
        if (event.key === 'Tab') { close(true); return; }
        const entries = items(), index = entries.indexOf(document.activeElement);
        let next;
        if (event.key === 'ArrowDown') next = (index + 1) % entries.length;
        if (event.key === 'ArrowUp') next = (index - 1 + entries.length) % entries.length;
        if (event.key === 'Home') next = 0;
        if (event.key === 'End') next = entries.length - 1;
        if (next !== undefined) { event.preventDefault(); entries[next]?.focus(); }
        else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey && event.key !== ' ') {
            const ordered = [...entries.slice(index + 1), ...entries.slice(0, index + 1)];
            const match = ordered.find(item => item.textContent.trim().toLowerCase().startsWith(event.key.toLowerCase()));
            if (match) { event.preventDefault(); match.focus(); }
        } else if (event.key === ' ' && document.activeElement?.tagName === 'A') {
            event.preventDefault(); document.activeElement.click();
        }
    });
    listen(panel, 'click', event => {
        if (event.target.closest?.('[role="menuitem"]')) close(true);
        // Do not preventDefault or remove the form: submit must remain native.
    });
    listen(document, 'pointerdown', event => { if (!panel.hidden && !root.contains(event.target)) close(); });
    listen(document, 'focusin', event => { if (!panel.hidden && !root.contains(event.target)) close(); });
    listen(window, 'resize', position);
    listen(document, 'scroll', position, {capture:true});
    accountMenus.set(id, () => { controller.abort(); close(); });
}

export function disposeAccountMenu(id) { accountMenus.get(id)?.(); accountMenus.delete(id); }
