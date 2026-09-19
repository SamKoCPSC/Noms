// Native modal dialogs own focus containment and background inertness. The shared
// registry owns only scroll locking and restoration, including nested lifetimes.
const dialogs = new Map();
let scrollSnapshot = null;

export function syncModal(id, focusSelector) {
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
        state = {dialog, previousFocus, focusSelector: null};
        dialogs.set(id, state);
    }
    // Do not steal focus on typing, failure messages, or unrelated parent renders.
    if (state.focusSelector !== focusSelector) {
        dialog.querySelector(focusSelector)?.focus({preventScroll: false});
        state.focusSelector = focusSelector;
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
