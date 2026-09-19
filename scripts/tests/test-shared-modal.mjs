// Deterministic lifecycle tests, not browser automation. Native focus/inert
// semantics are checked separately in the running application.
import test, { afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const source = await readFile(new URL('../../src/components/base/shared_modal.js', import.meta.url), 'utf8');
const { syncModal, disposeModal } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
let fixture;

function element() {
    const styles = new Map();
    return {
        isConnected: true,
        focusCalls: 0,
        style: {
            setProperty(name, value, priority = '') { styles.set(name, { value, priority }); },
            getPropertyValue(name) { return styles.get(name)?.value ?? ''; },
            getPropertyPriority(name) { return styles.get(name)?.priority ?? ''; },
            removeProperty(name) { styles.delete(name); },
        },
        attributes: new Set(),
        setAttribute(name) { this.attributes.add(name); },
        removeAttribute(name) { this.attributes.delete(name); },
        focus() { this.focusCalls++; fixture.document.activeElement = this; },
    };
}

function setup() {
    const trigger = element();
    const dialogs = new Map();
    fixture = { dialogs, trigger, document: {
        documentElement: element(), body: element(), activeElement: trigger,
        getElementById(id) { return dialogs.get(id); },
    } };
    globalThis.document = fixture.document;
    return fixture;
}

function addDialog(id) {
    const dialog = Object.assign(element(), {
        open: false, showCalls: 0, controls: new Map(),
        querySelector(selector) { return this.controls.get(selector); },
        contains(control) { return [...this.controls.values()].includes(control); },
        showModal() { this.open = true; this.showCalls++; },
        close() { this.open = false; },
    });
    dialog.controls.set('button', element());
    dialog.controls.set('input', element());
    fixture.dialogs.set(id, dialog);
    return dialog;
}

afterEach(() => {
    for (const id of [...fixture.dialogs.keys()].reverse()) disposeModal(id);
    delete globalThis.document;
});

test('open focuses the safe target, locks both roots, and close restores focus', () => {
    const { document, trigger } = setup();
    const dialog = addDialog('one');
    syncModal('one', 'button');
    assert.equal(dialog.open, true);
    assert.equal(document.activeElement, dialog.controls.get('button'));
    for (const root of [document.documentElement, document.body]) {
        assert.equal(root.style.getPropertyValue('overflow'), 'hidden');
        assert.equal(root.style.getPropertyPriority('overflow'), 'important');
    }
    disposeModal('one');
    assert.equal(dialog.open, false);
    assert.equal(document.activeElement, trigger);
    assert.equal(document.body.style.getPropertyValue('overflow'), '');
});

test('stage changes move focus but ordinary rerenders do not steal it', () => {
    const { document } = setup();
    const dialog = addDialog('one');
    syncModal('one', 'button');
    syncModal('one', 'input');
    assert.equal(document.activeElement, dialog.controls.get('input'));
    dialog.controls.get('button').focus();
    syncModal('one', 'input');
    assert.equal(document.activeElement, dialog.controls.get('button'));
    assert.equal(dialog.showCalls, 1);
});

test('cleanup preserves exact preexisting inline overflow and priority', () => {
    const { document } = setup();
    document.body.style.setProperty('overflow', 'clip', 'important');
    document.documentElement.style.setProperty('overflow', 'scroll');
    addDialog('one'); syncModal('one', 'button'); disposeModal('one');
    assert.equal(document.body.style.getPropertyValue('overflow'), 'clip');
    assert.equal(document.body.style.getPropertyPriority('overflow'), 'important');
    assert.equal(document.documentElement.style.getPropertyValue('overflow'), 'scroll');
    assert.equal(document.documentElement.style.getPropertyPriority('overflow'), '');
});

test('nested dialog close restores parent focus without releasing its scroll lock', () => {
    const { document, trigger } = setup();
    const parent = addDialog('parent'); syncModal('parent', 'button');
    addDialog('child'); syncModal('child', 'input'); disposeModal('child');
    assert.equal(document.activeElement, parent.controls.get('button'));
    assert.equal(document.body.style.getPropertyValue('overflow'), 'hidden');
    disposeModal('parent');
    assert.equal(document.body.style.getPropertyValue('overflow'), '');
    assert.equal(document.activeElement, trigger);
});

test('out-of-order parent disposal does not steal focus from the surviving dialog', () => {
    const { document } = setup();
    addDialog('parent'); syncModal('parent', 'button');
    const child = addDialog('child'); syncModal('child', 'input');
    disposeModal('parent');
    assert.equal(document.activeElement, child.controls.get('input'));
    assert.equal(document.body.style.getPropertyValue('overflow'), 'hidden');
    disposeModal('child');
    assert.equal(document.body.style.getPropertyValue('overflow'), '');
});

test('missing mounts and duplicate cleanup are safe and disconnected triggers are not focused', () => {
    const { trigger, document } = setup();
    syncModal('missing', 'button'); disposeModal('missing');
    assert.equal(document.body.style.getPropertyValue('overflow'), '');
    addDialog('one'); syncModal('one', 'button'); trigger.isConnected = false;
    disposeModal('one'); disposeModal('one');
    assert.equal(trigger.focusCalls, 0);
    assert.equal(document.body.style.getPropertyValue('overflow'), '');
});
