#!/usr/bin/env python3
"""Check delayed mobile layout and idle canvas resizing without scrolling.

Run against a local /about page, or an extracted standalone About preview.
Requires Playwright and Chromium; no account or database writes are needed.
"""
import argparse
import asyncio
from pathlib import Path
from urllib.parse import urlsplit

SETUP = """(() => {
    let frames = new Map(), id = 0, time = performance.now();
    requestAnimationFrame = callback => { frames.set(++id, callback); return id; };
    cancelAnimationFrame = id => frames.delete(id);
    window.tourStep = async count => {
        for (let i = 0; i < count; i++) {
            time += 80;
            const callbacks = [...frames.values()]; frames.clear();
            callbacks.forEach(callback => callback(time));
            await new Promise(resolve => setTimeout(resolve, 0));
        }
    };
    window.tourScrollEvents = 0;
    addEventListener('scroll', () => window.tourScrollEvents++);
    const observer = new MutationObserver(() => {
        if (!document.head) return;
        const style = document.createElement('style');
        style.id = 'delayed-tour-layout';
        style.textContent = '#story-canvas { width:900px !important; }';
        document.head.append(style); observer.disconnect();
    });
    observer.observe(document, {childList:true, subtree:true});
})();"""


async def main(args):
    from playwright.async_api import async_playwright

    assert urlsplit(args.base_url).hostname in {'localhost', '127.0.0.1', '::1'}
    args.output.mkdir(parents=True, exist_ok=True)
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path=args.chromium, args=[
            '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])
        context = await browser.new_context(viewport={'width':393, 'height':852},
                                            is_mobile=True, has_touch=True)
        await context.add_init_script(SETUP)
        page = await context.new_page()
        errors = []
        page.on('pageerror', lambda error: errors.append(str(error)))
        if args.html:
            await page.route(args.base_url.rstrip('/') + '/**',
                             lambda route: route.fulfill(path=str(args.html), content_type='text/html'))
        await page.goto(args.base_url.rstrip('/') + '/about')
        await page.wait_for_function("document.querySelector('#about-tour')?.dataset.renderer === 'webgl'", timeout=120000, polling=100)
        initial = await page.locator('#story-canvas').evaluate('c => c.width')
        assert initial >= 900, initial
        # Simulate the final stylesheet/layout arriving after onmounted. No resize
        # or scroll event is sent: the canvas box alone changes.
        await page.evaluate("document.getElementById('delayed-tour-layout').remove()")
        await page.evaluate('tourStep(2)')

        async def check_aspect(label):
            size = await page.locator('#story-canvas').evaluate('''c => {
                const r = c.getBoundingClientRect();
                return {width:c.width, height:c.height, cssWidth:r.width, cssHeight:r.height};
            }''')
            assert abs(size['width']/size['height'] - size['cssWidth']/size['cssHeight']) < .002, (label, size)
            assert await page.evaluate('scrollY === 0 && tourScrollEvents === 0')
            print('PASS:', label, size, flush=True)

        await check_aspect('layout settles before first scroll')
        for _ in range(400):
            await page.evaluate('tourStep(1)')
            await page.wait_for_timeout(40)
            if await page.locator('#about-tour').get_attribute('data-entry-state') == 'ready':
                break
        else:
            raise AssertionError('Tour never became ready')
        await page.evaluate('tourStep(50)')
        await check_aspect('initial entrance complete')
        await page.screenshot(path=str(args.output / 'initial-mobile.png'), timeout=60000)
        before = await page.locator('#story-canvas').get_attribute('data-render-count')
        # The phone tour is paused at rest: ResizeObserver must wake it up.
        await page.locator('#story-canvas').evaluate("c => c.style.height = '620px'")
        await page.wait_for_timeout(100)
        await page.evaluate('tourStep(3)')
        await check_aspect('idle CSS-only resize')
        after = await page.locator('#story-canvas').get_attribute('data-render-count')
        assert int(after) > int(before), (before, after)
        await page.locator('#story-canvas').evaluate("c => c.style.removeProperty('height')")
        await page.wait_for_timeout(100)
        await page.evaluate('tourStep(3)')
        await check_aspect('restored layout')
        assert not errors, errors
        await browser.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-url', default='http://localhost:8765')
    parser.add_argument('--html', type=Path)
    parser.add_argument('--chromium', default='/usr/bin/chromium')
    parser.add_argument('--output', type=Path, default=Path('/tmp/about-resize-browser'))
    asyncio.run(main(parser.parse_args()))
