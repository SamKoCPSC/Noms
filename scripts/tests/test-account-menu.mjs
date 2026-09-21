import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const source = await readFile(new URL('../../src/components/base/account-menu.js', import.meta.url), 'utf8');
const {mountAccountMenu, disposeAccountMenu, accountMenuPosition} = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('menu geometry clamps at edges and uses available vertical space', () => {
    assert.deepEqual(accountMenuPosition({right:990,top:20,bottom:60},280,320,1000,800), {left:710,top:68,maxHeight:724});
    assert.deepEqual(accountMenuPosition({right:90,top:600,bottom:640},280,320,400,700), {left:8,top:272,maxHeight:584});
});

test('keyboard navigation, dismissal, native submit lifetime and cleanup', () => {
    class Node extends EventTarget {
        constructor() { super(); this.attrs = {}; this.style = {}; this.hidden = false; this.isConnected = true; }
        setAttribute(k,v) { this.attrs[k]=v; }
        getAttribute(k) { return this.attrs[k]; }
        focus() { document.activeElement=this; }
        getBoundingClientRect() { return {right:990,top:20,bottom:60}; }
        getClientRects() { return [this.getBoundingClientRect()]; }
        contains(node) { return node === this; }
    }
    const trigger=new Node(), panel=new Node(), items=Array.from({length:4},()=>new Node());
    panel.hidden=true; panel.offsetWidth=280; panel.offsetHeight=320; panel.scrollHeight=318; panel.clientHeight=318;
    const heightWrites=[];
    panel.style=new Proxy({}, {set(target,key,value) { if(key==='maxHeight') heightWrites.push(value); target[key]=value; return true; }});
    panel.contains=node=>node===panel || items.includes(node);
    panel.querySelectorAll=()=>items;
    items.forEach(item=>{item.closest=()=>item; item.textContent='Action';});
    const root=new Node(); root.querySelector=q=>q==='[data-account-trigger]'?trigger:panel;
    root.contains=node=>node===trigger || panel.contains(node);
    globalThis.document=new Node(); document.getElementById=()=>root;
    globalThis.window=new Node(); globalThis.innerWidth=1000; globalThis.innerHeight=800;
    const key=(node,value,shiftKey=false)=> { const event=new Event('keydown',{cancelable:true}); Object.assign(event,{key:value,shiftKey}); node.dispatchEvent(event); return event; };
    try {
        mountAccountMenu('account'); mountAccountMenu('account');
        key(trigger,'ArrowDown'); assert.equal(panel.hidden,false); assert.equal(document.activeElement,items[0]);
        assert.equal(trigger.attrs['aria-expanded'],'true');
        assert.deepEqual(heightWrites,['724px'], 'positioning must not transiently expand the scroll container');
        key(panel,'End'); assert.equal(document.activeElement,items[3]);
        key(panel,'ArrowDown'); assert.equal(document.activeElement,items[0]);
        key(panel,'ArrowUp'); assert.equal(document.activeElement,items[3]);
        key(panel,'Home'); assert.equal(document.activeElement,items[0]);
        items[1].textContent='My profile'; items[2].textContent='Settings';
        key(panel,'s'); assert.equal(document.activeElement,items[2]);
        items[3].attrs['aria-disabled']='true';
        key(panel,'ArrowDown'); assert.equal(document.activeElement,items[0]);
        delete items[3].attrs['aria-disabled'];
        key(panel,'Escape'); assert.equal(panel.hidden,true); assert.equal(document.activeElement,trigger);
        key(trigger,'ArrowUp'); assert.equal(document.activeElement,items[3]);
        const tab=key(panel,'Tab'); assert.equal(tab.defaultPrevented,false); assert.equal(panel.hidden,true); assert.equal(document.activeElement,trigger);
        trigger.dispatchEvent(new Event('click')); assert.equal(panel.hidden,false);
        document.dispatchEvent(new Event('pointerdown')); assert.equal(panel.hidden,true);
        assert.equal(trigger.attrs['aria-expanded'],'false');
        trigger.dispatchEvent(new Event('click'));
        document.dispatchEvent(new Event('focusin')); assert.equal(panel.hidden,true);
        trigger.dispatchEvent(new Event('click'));
        trigger.getClientRects=()=>[];
        window.dispatchEvent(new Event('resize')); assert.equal(panel.hidden,true);
        trigger.getClientRects=()=>[trigger.getBoundingClientRect()];
        trigger.dispatchEvent(new Event('click'));
        const click=new Event('click',{cancelable:true}); Object.defineProperty(click,'target',{value:items[3]}); panel.dispatchEvent(click);
        assert.equal(click.defaultPrevented,false); assert.equal(panel.hidden,true); assert.equal(items[3].isConnected,true);
        disposeAccountMenu('account'); disposeAccountMenu('account');
        trigger.dispatchEvent(new Event('click')); assert.equal(panel.hidden,true);
    } finally { delete globalThis.document; delete globalThis.window; delete globalThis.innerWidth; delete globalThis.innerHeight; }
});

test('markup preserves logout POST and makes P1 a named nonmodal menu', async () => {
    const rust=await readFile(new URL('../../src/components/navbar.rs',import.meta.url),'utf8');
    assert.ok(rust.includes('aria_haspopup: "menu"'));
    assert.ok(rust.includes('role: "menu"'));
    assert.ok(rust.includes('"My profile"'));
    assert.ok(rust.includes('action: "/auth/logout?redirect_uri=/"'));
    assert.ok(!rust.includes('navbar-dropdown-item navbar-dropdown-item-danger'));
    assert.ok(!source.includes('preventDefault(); // submit'));
});
