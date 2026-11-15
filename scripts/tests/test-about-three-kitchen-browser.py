#!/usr/bin/env python3
"""Exercise the Three kitchen through the real Rust tour (local URL only)."""
import argparse
import asyncio
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright

CLOCK = '''(()=>{const nativeRAF=requestAnimationFrame.bind(window);
window.paint=()=>new Promise(r=>nativeRAF(()=>nativeRAF(r)));
// Software WebGL warms the full model set much more slowly than a real GPU.
// Extend only the production loader's 25s deadline for this functional test.
const timeout=setTimeout.bind(window);
window.setTimeout=(f,ms,...args)=>timeout(f,ms===25000?120000:ms,...args);
let q=new Map(),id=0,t=performance.now();
requestAnimationFrame=f=>{q.set(++id,f);return id};cancelAnimationFrame=i=>q.delete(i);
window.step=async n=>{for(let i=0;i<n;i++){t+=80;const a=[...q.values()];q.clear();
for(const f of a)f(t);await new Promise(r=>setTimeout(r,0));}}})();'''


async def ready(page):
    await page.wait_for_selector('.about-tour.is-flight')
    for _ in range(400):
        await page.evaluate('step(1)')
        await page.wait_for_timeout(30)
        if await page.locator('#about-tour').get_attribute('data-entry-state') == 'ready':
            await page.evaluate('paint()')
            await page.evaluate('step(2)')
            assert await page.locator('#about-tour').evaluate('e=>e.classList.contains("is-flight")'), 'Tour fell back to reading mode'
            print('READY', flush=True)
            return
    raise AssertionError('Tour did not finish loading')


async def go(page, index):
    await page.evaluate('i=>{scrollTo({top:document.querySelectorAll(".waypoint")[i].getBoundingClientRect().top+scrollY,behavior:"instant"});dispatchEvent(new Event("scroll"));}', index)
    await page.evaluate('paint()')
    await page.evaluate('step(45)')


async def main(args):
    assert urlsplit(args.url).hostname in {'localhost', '127.0.0.1', '::1'}
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path='/usr/bin/chromium', args=['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])
        context = await browser.new_context(viewport={'width': 393, 'height': 852})
        await context.add_init_script(CLOCK)
        page = await context.new_page()
        page.set_default_timeout(120000)
        errors = []
        page.on('pageerror', lambda error: errors.append(str(error)))
        await page.goto(args.url)
        await ready(page)
        root = page.locator('#about-tour')
        legacy = page.locator('#story-canvas')
        kitchen = page.locator('#story-kitchen-canvas')
        assert await root.get_attribute('data-three-kitchen-state') == 'ready'
        assert await kitchen.get_attribute('data-animations') == '0'
        assert await kitchen.get_attribute('data-baked-shadows') == 'true'
        assert float((await legacy.get_attribute('data-arrival-frames')).split(',')[0]) == 160
        assert await legacy.get_attribute('data-shadow-targets') == '0'
        assert await kitchen.is_visible(), await kitchen.evaluate('c=>({style:c.style.cssText,box:c.getBoundingClientRect().toJSON(),root:document.querySelector("#about-tour").className})')
        for viewport in ({'width':393,'height':852}, {'width':1280,'height':900}):
            await page.set_viewport_size(viewport)
            # Deliver native resize/media events even while the tour RAF is controlled.
            await page.evaluate('paint()')
            for _ in range(30):
                await page.wait_for_timeout(100)
                await page.evaluate('step(1)')
                if await legacy.evaluate('c=>Math.abs(c.width/c.height-c.clientWidth/c.clientHeight)<.003'):
                    break
            boxes = await kitchen.evaluate('c=>{let b=c.getBoundingClientRect(),o=document.querySelector("#story-canvas");return [b.x,b.y,b.width,b.height,c.width,c.height,o.width,o.height]}')
            assert boxes[0] == boxes[1] == 0 and boxes[3] == viewport['height'], boxes
            assert boxes[4:6] == boxes[6:8], boxes
            assert abs(boxes[4]/boxes[5] - boxes[2]/boxes[3]) < .003, boxes
        if args.output:
            await page.screenshot(path=str(args.output), timeout=60000)
        await go(page, 1)
        assert not await kitchen.is_visible()
        assert await legacy.get_attribute('data-shadow-room') == 'why'
        assert int(await legacy.get_attribute('data-shadow-targets')) > 0
        await go(page, 0)
        assert await kitchen.is_visible(), await legacy.get_attribute('data-progress')
        assert float((await legacy.get_attribute('data-arrival-frames')).split(',')[0]) == 160
        await page.emulate_media(reduced_motion='reduce')
        await page.evaluate('paint()')
        await page.evaluate('step(3)')
        assert not await kitchen.is_visible()
        assert await root.evaluate('e=>e.classList.contains("is-reader")')
        await page.emulate_media(reduced_motion='no-preference')
        await page.evaluate('paint()')
        await ready(page)
        await go(page, 0)
        assert await kitchen.is_visible(), await page.evaluate('()=>({root:document.querySelector("#about-tour").className,state:document.querySelector("#about-tour").dataset.threeKitchenState,progress:document.querySelector("#story-canvas").dataset.progress,style:document.querySelector("#story-kitchen-canvas").style.cssText})')
        # Losing only the Three context must preserve the tour via its old renderer.
        await kitchen.evaluate('c=>c.getContext("webgl2").getExtension("WEBGL_lose_context").loseContext()')
        await page.wait_for_timeout(150)
        await page.evaluate('step(5)')
        assert await root.get_attribute('data-three-kitchen-state') == 'fallback'
        assert not await kitchen.is_visible()
        assert int(await legacy.get_attribute('data-shadow-targets')) > 0
        assert not errors, errors
        await page.close()
        # Failed baked-asset requests also leave a fully usable legacy tour.
        fallback = await context.new_page()
        fallback.set_default_timeout(120000)
        fallback.on('pageerror', lambda error: errors.append(str(error)))
        await fallback.route('**/kitchen-baked*', lambda route: route.abort())
        await fallback.goto(args.url)
        await ready(fallback)
        assert await fallback.locator('#about-tour').get_attribute('data-three-kitchen-state') == 'fallback'
        assert not await fallback.locator('#story-kitchen-canvas').is_visible()
        await go(fallback, 1)
        assert await fallback.locator('#story-canvas').get_attribute('data-shadow-room') == 'why'
        assert not errors, errors
        await browser.close()
        print('PASS: initial framing, static kitchen, resize, camera travel, reduced motion, context loss, asset fallback')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:8765/about')
    parser.add_argument('--output', type=Path)
    asyncio.run(main(parser.parse_args()))
