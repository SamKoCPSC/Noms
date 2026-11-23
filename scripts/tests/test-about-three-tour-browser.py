#!/usr/bin/env python3
"""Exercise all baked scenes through the real Rust tour (local URL only)."""
import argparse
import asyncio
import base64
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright

CLOCK = '''(()=>{const nativeRAF=requestAnimationFrame.bind(window);
window.nativeRAF=nativeRAF;window.paint=()=>new Promise(r=>nativeRAF(()=>nativeRAF(r)));
const timeout=setTimeout.bind(window);
window.setTimeout=(f,ms,...args)=>timeout(f,(ms===25000||ms===20000)?120000:ms,...args);
let q=new Map(),id=0,t=performance.now();
requestAnimationFrame=f=>{q.set(++id,f);return id};window.controlledRAF=requestAnimationFrame;cancelAnimationFrame=i=>q.delete(i);
window.step=async n=>{for(let i=0;i<n;i++){t+=80;const a=[...q.values()];q.clear();
for(const f of a)f(t);await new Promise(r=>setTimeout(r,0));}}})();'''

async def ready(page):
    await page.wait_for_selector('.about-tour.is-flight')
    for _ in range(600):
        await page.evaluate('step(1)'); await page.wait_for_timeout(50)
        if await page.locator('#about-tour').get_attribute('data-entry-state') == 'ready':
            await page.evaluate('paint()'); await page.evaluate('step(2)')
            assert await page.locator('#about-tour').evaluate('e=>e.classList.contains("is-flight")')
            return
    raise AssertionError('Tour did not finish loading')

async def go(page, index):
    await page.evaluate('i=>{scrollTo({top:document.querySelectorAll(".waypoint")[i].getBoundingClientRect().top+scrollY,behavior:"instant"});dispatchEvent(new Event("scroll"));}', index)
    await page.evaluate('paint()'); await page.evaluate('step(45)')

async def baked(page, index):
    canvas=page.locator('#story-three-canvas')
    for _ in range(800):
        await page.evaluate('step(1)'); await page.wait_for_timeout(50)
        drawn=(await canvas.get_attribute('data-drawn-models') or '').split(',')
        if str(index) in drawn and await canvas.is_visible():
            assert int(await canvas.get_attribute('data-resident-models'))<=3
            assert int(await canvas.get_attribute('data-pending-models'))<=2
            return
    raise AssertionError(f'Stop {index} did not render with Three: {await canvas.evaluate("c=>({...c.dataset})")}')

async def screenshot(page, path):
    # Chromium's surface capture can stall with two WebGL canvases under
    # SwiftShader. Capture the visible compositor viewport directly instead.
    session=await page.context.new_cdp_session(page)
    try:
        result=await session.send('Page.captureScreenshot', {
            'format':'png', 'fromSurface':False, 'captureBeyondViewport':False})
        Path(path).write_bytes(base64.b64decode(result['data']))
    finally:
        await session.detach()

async def main(args):
    assert urlsplit(args.url).hostname in {'localhost','127.0.0.1','::1'}
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        context=await browser.new_context(viewport={'width':393,'height':852})
        await context.add_init_script(CLOCK)
        page=await context.new_page();page.set_default_timeout(120000)
        errors=[];page.on('pageerror',lambda error:errors.append(str(error)))
        await page.goto(args.url);await ready(page)
        root=page.locator('#about-tour');legacy=page.locator('#story-canvas');three=page.locator('#story-three-canvas')
        assert await root.get_attribute('data-three-tour-state')=='ready'
        assert await three.get_attribute('data-animations')=='0'
        assert await three.get_attribute('data-baked-shadows')=='true'
        assert all(float(n)==160 for n in (await legacy.get_attribute('data-arrival-frames')).split(','))
        for index in range(12):
            await go(page,index);await baked(page,index)
            assert await legacy.get_attribute('data-shadow-targets')=='0'
            if args.output:
                args.output.mkdir(parents=True,exist_ok=True)
                await screenshot(page,args.output/f'stop-{index:02}.png')
            print('PASS baked stop',index,flush=True)
        # Revisiting evicted rooms reloads them without replaying an entrance.
        await go(page,0);await baked(page,0)
        for viewport in ({'width':1280,'height':900},{'width':393,'height':852}):
            await page.set_viewport_size(viewport);await page.evaluate('paint()')
            for _ in range(30):
                await page.wait_for_timeout(100);await page.evaluate('step(1)')
                if await legacy.evaluate('c=>Math.abs(c.width/c.height-c.clientWidth/c.clientHeight)<.003'):break
            boxes=await three.evaluate('c=>{let b=c.getBoundingClientRect(),o=document.querySelector("#story-canvas");return [b.x,b.y,b.width,b.height,c.width,c.height,o.width,o.height]}')
            assert boxes[0]==boxes[1]==0 and boxes[3]==viewport['height'],boxes
            assert boxes[4:6]==boxes[6:8] and abs(boxes[4]/boxes[5]-boxes[2]/boxes[3])<.003,boxes
        print('PASS revisit and responsive framing',flush=True)
        await page.emulate_media(reduced_motion='reduce');await page.evaluate('paint()');await page.evaluate('step(3)')
        assert not await three.is_visible() and await root.evaluate('e=>e.classList.contains("is-reader")')
        await page.emulate_media(reduced_motion='no-preference');await page.evaluate('paint()');await ready(page);await go(page,0);await baked(page,0)
        await three.evaluate('c=>c.getContext("webgl2").getExtension("WEBGL_lose_context").loseContext()')
        await page.wait_for_timeout(150);await page.evaluate('step(5)')
        assert await root.get_attribute('data-three-tour-state')=='fallback'
        assert not await three.is_visible() and int(await legacy.get_attribute('data-shadow-targets'))>0
        assert all(float(n)==160 for n in (await legacy.get_attribute('data-arrival-frames')).split(','))
        print('PASS reduced motion and context-loss fallback',flush=True)
        await page.close()
        # A failed scene stays usable in the old renderer; the next scene still uses Three.
        page=await context.new_page();page.set_default_timeout(120000);page.on('pageerror',lambda error:errors.append(str(error)))
        await page.route('**/kitchen-baked*',lambda route:route.abort())
        await page.goto(args.url);await ready(page)
        assert '0' in (await page.locator('#story-three-canvas').get_attribute('data-failed-models')).split(',')
        assert int(await page.locator('#story-canvas').get_attribute('data-shadow-targets'))>0
        await go(page,1);await baked(page,1)
        print('PASS individual asset failure and recovery at the next stop',flush=True)
        await page.close()
        # Direct fragment entry must load that scene, not force a trip through the kitchen.
        page=await context.new_page();page.set_default_timeout(120000)
        await page.goto(args.url+'#your-privacy');await ready(page)
        await baked(page,10)
        assert not errors,errors
        await browser.close()
        print('PASS: all scenes, bounded cache, static arrivals, resize, travel, reduced motion, context loss and asset fallback')

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--url',default='http://127.0.0.1:8765/about');parser.add_argument('--output',type=Path)
    asyncio.run(main(parser.parse_args()))
