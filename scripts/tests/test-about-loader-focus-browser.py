#!/usr/bin/env python3
"""Verify loader focus handoffs against the real local About tour runtime."""
import argparse
import asyncio
from urllib.parse import urlsplit

from playwright.async_api import async_playwright, expect

CLOCK = '''(() => {
const nativeRAF=requestAnimationFrame.bind(window);
window.paint=()=>new Promise(resolve=>nativeRAF(()=>nativeRAF(resolve)));
const timeout=setTimeout.bind(window);
window.setTimeout=(callback,ms,...args)=>timeout(callback,(ms===25000||ms===20000)?300000:ms,...args);
let queue=new Map(),id=0,time=performance.now();
requestAnimationFrame=callback=>{queue.set(++id,callback);return id};
cancelAnimationFrame=id=>queue.delete(id);
window.step=async count=>{for(let i=0;i<count;i++){time+=80;
const callbacks=[...queue.values()];queue.clear();
for(const callback of callbacks)callback(time);
await new Promise(resolve=>setTimeout(resolve,0));}};
})();'''


async def focus_loader(page):
    await page.wait_for_selector('.about-tour.is-flight')
    pause = page.locator('#tour-loader [data-penguini-pause]')
    for _ in range(300):
        if await pause.is_visible():
            await pause.focus()
            assert await pause.evaluate('button=>document.activeElement===button')
            return
        await page.evaluate('step(1)')
        await page.wait_for_timeout(30)
    raise AssertionError('Mascot pause control did not become available while loading')


async def finish_loading(page):
    for _ in range(1200):
        await page.evaluate('step(1)')
        if await page.locator('#about-tour').get_attribute('data-entry-state') == 'ready':
            # The frame that opens the gate still has inert chapter panels.
            # Allow the next frame to make the visible chapter interactive.
            await page.evaluate('step(3)')
            await page.evaluate('paint()')
            return
        await page.wait_for_timeout(30)
    raise AssertionError('The entry gate did not open')


async def assert_chapter_focus(page):
    active = await page.locator('#story-canvas').get_attribute('data-active-chapter')
    assert await page.evaluate('document.activeElement.id') == f'title-{active}'
    assert not await page.evaluate('document.activeElement.closest("[inert]") !== null')


async def main(args):
    assert urlsplit(args.url).hostname in {'localhost', '127.0.0.1', '::1'}
    async with async_playwright() as p:
        cases = [('desktop', {'width':1440,'height':1000}, ''),
                 ('mobile', {'width':390,'height':844}, ''),
                 ('fragment', {'width':1440,'height':1000}, '#your-privacy')]
        for name, viewport, fragment in cases:
            if args.case != 'all' and name != args.case:
                continue
            browser = await p.chromium.launch(executable_path=args.chromium, args=[
                '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])
            context = await browser.new_context(viewport=viewport)
            await context.add_init_script(CLOCK)
            page = await context.new_page()
            page.set_default_timeout(120000)
            errors = []
            render_errors = []
            page.on('pageerror', lambda error: errors.append(type(error).__name__))
            page.on('console', lambda message: render_errors.append(message.text) if message.type=='error' else None)
            await page.goto(args.url + fragment)
            await focus_loader(page)
            await finish_loading(page)
            assert await page.locator('#about-tour').evaluate('root=>root.classList.contains("is-flight")'), render_errors[-4:]
            await assert_chapter_focus(page)
            if fragment:
                assert await page.locator('#story-canvas').get_attribute('data-active-chapter') == '10'
            print('PASS: normal loader focus', viewport, fragment, flush=True)

            # Loading must preserve focus when it belongs outside the loader.
            await page.emulate_media(reduced_motion='reduce')
            await page.wait_for_selector('.about-tour.is-reader')
            await page.emulate_media(reduced_motion='no-preference')
            await page.wait_for_selector('.about-tour.is-flight')
            outside = page.get_by_role('button', name='Outside the tour', exact=True)
            await page.evaluate('''() => {
                const button=document.createElement('button');
                button.id='outside-tour-focus';button.textContent='Outside the tour';
                document.body.append(button);button.focus();
            }''')
            await finish_loading(page)
            await expect(outside).to_be_focused()
            assert not errors, errors
            print('PASS: outside focus preserved', flush=True)

            # Start another load without advancing the controlled clock, then
            # exercise the renderer failure transition while Pause owns focus.
            await page.emulate_media(reduced_motion='reduce')
            await page.wait_for_selector('.about-tour.is-reader')
            await page.emulate_media(reduced_motion='no-preference')
            await focus_loader(page)
            await page.locator('#story-canvas').dispatch_event('webglcontextlost', {'cancelable':True})
            await page.wait_for_selector('.about-tour.is-reader')
            await assert_chapter_focus(page)
            assert not errors, errors
            print('PASS: reading fallback focus', flush=True)
            await context.close()
            await browser.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--url', default='http://127.0.0.1:8765/about')
    parser.add_argument('--chromium', default='/usr/bin/chromium')
    parser.add_argument('--case', choices=['all','desktop','mobile','fragment'], default='all')
    asyncio.run(main(parser.parse_args()))
