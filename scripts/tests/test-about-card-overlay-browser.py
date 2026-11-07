#!/usr/bin/env python3
"""Exercise the actual tour runtime at a local app/preview URL (Playwright)."""
import argparse
import asyncio
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright
CLOCK = '(()=>{let q=new Map(),id=0,t=performance.now();requestAnimationFrame=f=>{q.set(++id,f);return id};cancelAnimationFrame=i=>q.delete(i);window.step=async n=>{for(let i=0;i<n;i++){t+=80;const a=[...q.values()];q.clear();for(const f of a)f(t);await new Promise(r=>setTimeout(r,0));}}})();'

async def main(args):
    assert urlsplit(args.url).hostname in {'localhost', '127.0.0.1', '::1'}
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path='/usr/bin/chromium', args=['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])
        context = await browser.new_context(viewport={'width': 393, 'height': 852})
        await context.add_init_script(CLOCK)
        page = await context.new_page()
        errors = []
        page.on('pageerror', lambda e: errors.append(str(e)))
        await page.goto(args.url)
        await page.wait_for_selector('.about-tour.is-flight')
        for _ in range(400):
            await page.evaluate('step(1)')
            await page.wait_for_timeout(30)
            if await page.locator('#about-tour').get_attribute('data-entry-state') == 'ready':
                break
        else:
            raise AssertionError('Loading did not finish')
        print('READY', flush=True)
        canvas = page.locator('#story-canvas')
        assert await canvas.get_attribute('data-scene-instances') == '12'

        async def go(i):
            await page.evaluate('(i)=>scrollTo(0,document.querySelectorAll(".waypoint")[i].getBoundingClientRect().top+scrollY)', i)
            await page.wait_for_timeout(150)
            await page.evaluate('step(45)')
        for i in [0, 2, 3, 6, 7, 10, 11]:
            await go(i)
            card = page.locator('.story-panel[data-active=true]')
            assert await card.get_attribute('data-panel') == str(i)
            state = await page.evaluate("()=>{let c=document.querySelector('#story-canvas'),r=c.getBoundingClientRect(),p=document.querySelector('.story-panel[data-active=true]'),s=document.querySelector('#story').getBoundingClientRect(),pr=p.getBoundingClientRect();return {x:r.x,y:r.y,w:r.width,h:r.height,vw:document.documentElement.clientWidth,vh:innerHeight,aspect:c.width/c.height,overflow:p.scrollHeight>p.clientHeight+1,center:(pr.top+pr.bottom-s.top-s.bottom)/2}}")
            assert state['x'] == state['y'] == 0, state
            assert 0 <= state['vw'] - state['w'] <= 16 and state['h'] == state['vh'], state
            assert abs(state['aspect'] - state['w'] / state['h']) < 0.003, state
            assert not state['overflow'] and abs(state['center']) < 1, state
            frames = list(map(float, (await canvas.get_attribute('data-arrival-frames')).split(',')))
            assert len(frames) == 12 and frames[i] == 160, frames
            assert int(await canvas.get_attribute('data-shadow-targets')) <= 2
            if i == 2:
                assert frames[3] == 1, frames
            print('STOP', i, state, flush=True)
        await go(0)
        await page.evaluate('scrollBy(0,innerHeight*1.25)')
        await page.wait_for_timeout(150)
        await page.evaluate('step(25)')
        eye = list(map(float, (await canvas.get_attribute('data-camera')).split(',')))
        assert all((abs(a - b) < 0.01 for a, b in zip(eye, [4.1875, 3.54375, 2.375]))), eye
        await page.screenshot(path=str(args.output))
        await go(6)
        for w, h in [(360, 640), (1280, 900), (393, 852)]:
            await page.set_viewport_size({'width': w, 'height': h})
            await page.wait_for_timeout(500)
            await page.evaluate('step(25)')
            await go(6)
            assert abs(float(await canvas.evaluate('c=>c.width/c.height-c.getBoundingClientRect().width/c.getBoundingClientRect().height'))) < 0.003
            assert await page.locator('.story-panel[data-active=true]').get_attribute('data-panel') == '6'
            print('RESIZE', w, h, flush=True)
        await page.emulate_media(reduced_motion='reduce')
        await page.evaluate('step(2)')
        await page.wait_for_selector('.about-tour.is-reader')
        assert await page.locator('.about-tour.is-reader').count() == 1
        assert await page.locator('.story-panel[inert]').count() == 0
        assert not errors, errors
        await browser.close()
        print('PASS: full viewport, centered overlay, independent arrivals, original camera, resize, reduced motion', flush=True)
if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://localhost:8765')
    parser.add_argument('--output', type=Path, default=Path('/tmp/about-overlay-travel.png'))
    asyncio.run(main(parser.parse_args()))
