import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source = await readFile(new URL('../../src/components/base/feedback.js', import.meta.url), 'utf8');
const {tooltipPosition, mountTooltip, disposeTooltip} = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
test('tooltip centers below a trigger with no overlap', () => {
  assert.deepEqual(tooltipPosition({left:100,right:140,top:40,bottom:70},100,30,500,400),{left:70,top:78});
});
test('tooltip clamps horizontally and flips above near bottom', () => {
  assert.deepEqual(tooltipPosition({left:470,right:490,top:350,bottom:380},120,40,500,400),{left:372,top:302});
  assert.equal(tooltipPosition({left:0,right:10,top:40,bottom:70},120,40,500,400).left,8);
});

test('tooltip focus, hover, Escape dismissal and cleanup do not move focus', async () => {
  const button = new EventTarget();
  button.disabled = false;
  button.getBoundingClientRect = () => ({left:100,right:140,top:40,bottom:70});
  const tip = new EventTarget();
  Object.assign(tip, {hidden:true, offsetWidth:100, offsetHeight:30, style:{}, open:false,
    setAttribute(){}, matches(){return this.open;}, showPopover(){this.open=true;}, hidePopover(){this.open=false;}});
  const root = new EventTarget();
  root.querySelector = selector => selector === 'button' ? button : tip;
  globalThis.document = Object.assign(new EventTarget(), {getElementById:()=>root});
  globalThis.window = new EventTarget();
  globalThis.innerWidth=500; globalThis.innerHeight=400;
  try {
    mountTooltip('fixture'); mountTooltip('fixture');
    button.dispatchEvent(new Event('focus'));
    assert.equal(tip.hidden,false); assert.equal(tip.style.top,'78px');
    const escape = Object.assign(new Event('keydown',{cancelable:true}),{key:'Escape'});
    document.dispatchEvent(escape);
    assert.equal(tip.hidden,true); assert.equal(escape.defaultPrevented,true);
    root.dispatchEvent(new Event('mouseenter'));
    assert.equal(tip.hidden,true,'Escape suppression persists while trigger remains active');
    root.dispatchEvent(new Event('mouseleave')); button.dispatchEvent(new Event('blur'));
    await new Promise(resolve=>setTimeout(resolve,180));
    root.dispatchEvent(new Event('mouseenter'));
    assert.equal(tip.hidden,false);
    root.dispatchEvent(new Event('mouseleave')); tip.dispatchEvent(new Event('mouseenter'));
    await new Promise(resolve=>setTimeout(resolve,180));
    assert.equal(tip.hidden,false,'tip remains hoverable across the gap');
    disposeTooltip('fixture'); disposeTooltip('fixture');
    assert.equal(tip.hidden,true);
    button.dispatchEvent(new Event('focus')); assert.equal(tip.hidden,true,'listeners removed');
  } finally {
    disposeTooltip('fixture');
    delete globalThis.document; delete globalThis.window; delete globalThis.innerWidth; delete globalThis.innerHeight;
  }
});
