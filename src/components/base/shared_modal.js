// Native modal dialogs own background inertness. Supplement their Tab boundaries
// so browser chrome does not become a stop inside a modal keyboard sequence.
const dialogs = new Map();
let scrollSnapshot = null;

export function syncModal(id, focusSelector, pending = false) {
    const dialog = document.getElementById(id);
    if (!dialog || !dialog.isConnected) return;
    let state = dialogs.get(id);
    if (!state) {
        const previousFocus = document.activeElement;
        const target = dialog.querySelector(focusSelector);
        // Native showModal chooses autofocus before its default first control.
        target?.setAttribute('autofocus', '');
        dialog.showModal();
        target?.removeAttribute('autofocus');
        if (!dialogs.size) {
            scrollSnapshot = [document.documentElement, document.body].map(element => ({
                element,
                value: element.style.getPropertyValue('overflow'),
                priority: element.style.getPropertyPriority('overflow'),
            }));
            for (const {element} of scrollSnapshot) element.style.setProperty('overflow', 'hidden', 'important');
        }
        state = {dialog, previousFocus, focusSelector: null, pending: false};
        dialogs.set(id, state);
    }
    // Do not steal focus on typing, failure messages, or unrelated parent renders.
    if (state.focusSelector !== focusSelector || state.pending !== pending) {
        // Disabling the focused submit button can move focus to body. Keep an
        // explicit focus owner while controls are unavailable, then restore the
        // safe action after a failed request permits interaction again.
        if (pending) {
            dialog.setAttribute('tabindex', '-1');
            dialog.focus({preventScroll: true});
        } else {
            dialog.querySelector(focusSelector)?.focus({preventScroll: false});
        }
        state.focusSelector = focusSelector;
        state.pending = pending;
    }
}

export function disposeModal(id) {
    const state = dialogs.get(id);
    if (!state) return;
    dialogs.delete(id);
    state.dialog.close();
    if (!dialogs.size && scrollSnapshot) {
        for (const {element, value, priority} of scrollSnapshot) {
            if (value) element.style.setProperty('overflow', value, priority);
            else element.style.removeProperty('overflow');
        }
        scrollSnapshot = null;
    }
    const top = [...dialogs.values()].at(-1)?.dialog;
    if (state.previousFocus?.isConnected && (!top || top.contains(state.previousFocus))) {
        state.previousFocus.focus({preventScroll: true});
    }
}

// Return true only when the caller must suppress native Tab navigation. Interior
// controls retain their platform behavior and only the top modal can wrap focus.
export function wrapModalTab(id, backwards) {
    const state = dialogs.get(id);
    if (!state || state !== [...dialogs.values()].at(-1) || !state.dialog.open || !state.dialog.isConnected) return false;
    const dialog = state.dialog;
    const controls = [...dialog.querySelectorAll(
        'button, input, select, textarea, a[href], area[href], summary, [tabindex], [contenteditable="true"]'
    )].filter(control => {
        if (control.disabled || control.hidden || (control.tabIndex ?? 0) < 0 || !control.getClientRects().length) return false;
        if (control.matches?.(':disabled') || control.closest?.('[inert]')) return false;
        const visibility = typeof getComputedStyle === 'function' ? getComputedStyle(control).visibility : 'visible';
        return visibility !== 'hidden' && visibility !== 'collapse';
    }).sort((left, right) => {
        // Positive tabindex precedes ordinary document order; stable sort retains
        // the DOM order for equal priorities (including the normal zero case).
        const priority = control => control.tabIndex > 0 ? control.tabIndex : Infinity;
        return priority(left) - priority(right);
    });
    if (!controls.length) {
        dialog.setAttribute('tabindex', '-1');
        dialog.focus();
        return true;
    }
    const activeIndex = controls.indexOf(document.activeElement);
    if (activeIndex === -1 || (backwards ? activeIndex === 0 : activeIndex === controls.length - 1)) {
        controls[backwards ? controls.length - 1 : 0].focus();
        return true;
    }
    return false;
}
