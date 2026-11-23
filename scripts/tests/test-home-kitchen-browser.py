#!/usr/bin/env python3
"""Exercise the actual landing component and Three renderer (local app URL only)."""
import argparse, asyncio, json
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright, expect

CLOCK = '''(()=>{const native=requestAnimationFrame.bind(window);window.paint=()=>new Promise(r=>native(()=>native(r)));
let q=new Map(),id=0,t=performance.now();requestAnimationFrame=f=>{q.set(++id,f);return id};cancelAnimationFrame=i=>q.delete(i);
window.step=async (n,delta=100)=>{for(let i=0;i<n;i++){t+=delta;const a=[...q.values()];q.clear();for(const f of a)f(t);await new Promise(r=>setTimeout(r,0));}}})();'''

async def ready(page, host="#landing-kitchen"):
    for _ in range(1200):
        await page.evaluate('step(1)'); await page.wait_for_timeout(50)
        if await page.locator(host).get_attribute('data-state') == 'ready':
            await page.evaluate('step(3)');return
    raise AssertionError(f'{host} did not load')

async def main(args):
    assert urlsplit(args.url).hostname in {'localhost','127.0.0.1','::1'}
    args.output.mkdir(parents=True,exist_ok=True)
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        context=await browser.new_context(viewport={'width':1440,'height':650},reduced_motion='no-preference')
        await context.add_init_script(CLOCK)
        page=await context.new_page();page.set_default_timeout(120000)
        errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
        desk_requests = []
        page.on('request', lambda req: desk_requests.append(req.url) if 'recipe-desk' in req.url and '.glb' in req.url else None)
        await page.goto(args.url);await ready(page)
        assert not desk_requests, 'Desk download started before scrolling toward the mission'
        canvas=page.locator('#landing-kitchen-canvas');control=page.locator('#landing-kitchen button')
        assert await canvas.get_attribute('data-renderer')=='three'
        assert await canvas.get_attribute('data-baked-shadows')=='true'
        manifest = Path(__file__).resolve().parents[2] / 'assets/landing/kitchen/lighting.json'
        duration = json.loads(manifest.read_text())['duration_seconds']
        assert duration > 0 and abs(float(await canvas.get_attribute('data-duration')) - duration) < .01
        assert await control.get_attribute('aria-label')=='Pause kitchen animation'
        await page.evaluate('step(25)')
        await control.click();await page.evaluate('step(2)')
        time=await canvas.get_attribute('data-time');await page.evaluate('step(10)')
        assert await canvas.get_attribute('data-time')==time
        assert await canvas.evaluate('c=>getComputedStyle(c).opacity')=='1'
        await page.screenshot(path=str(args.output/'desktop.png'))
        await page.set_viewport_size({'width':393,'height':852});await page.evaluate('paint()');await page.evaluate('step(3)')
        assert not await page.evaluate('document.documentElement.scrollWidth>innerWidth')
        assert await canvas.evaluate('c=>Math.abs(c.width/c.height-c.clientWidth/c.clientHeight)<.01')
        await page.locator('.landing-hero').screenshot(path=str(args.output/'mobile.png'))
        await page.evaluate('document.documentElement.classList.add("dark")');await page.locator('.landing-hero').screenshot(path=str(args.output/'mobile-dark.png'))
        await page.evaluate('document.documentElement.classList.remove("dark")')
        # Repeat the full sequence twice and confirm it progresses after both resets.
        await control.click();await page.evaluate('step(1)')
        await page.evaluate('ms => step(32, ms)', 2 * (duration + .4) * 1000 / 32)
        assert int(await canvas.get_attribute('data-cycles'))>=2
        t=float(await canvas.get_attribute('data-time'));await page.evaluate('step(8)')
        assert float(await canvas.get_attribute('data-time'))>t
        print('PASS autoplay, pause/resume, two complete loops, responsive framing',flush=True)
        # No offscreen animation work; resume from the held pose when scrolled back.
        await page.evaluate('scrollTo(0,document.body.scrollHeight)');await page.evaluate('paint()');await page.wait_for_timeout(100);await page.evaluate('step(2)')
        t=await canvas.get_attribute('data-time');await page.evaluate('step(20)');assert await canvas.get_attribute('data-time')==t
        await page.evaluate('scrollTo(0,0)');await page.evaluate('paint()');await page.wait_for_timeout(100);await page.evaluate('step(10)')
        assert await canvas.get_attribute('data-time')!=t
        # Exercise the second scene after scrolling to it, independently of the hero.
        mission = page.locator('.landing-mission')
        desk = mission.locator('#landing-recipe-desk')
        desk_canvas = desk.locator('canvas')
        desk_button = desk.get_by_role('button')
        await page.set_viewport_size({'width':1440, 'height':900})
        await desk.scroll_into_view_if_needed()
        await ready(page, '#landing-recipe-desk')
        assert len(desk_requests) == 1
        assert await desk_canvas.get_attribute('data-duration') == '25.5'
        assert await desk_button.get_attribute('aria-controls') == await desk_canvas.get_attribute('id')
        await page.evaluate('step(10,1950)')
        await desk_button.click()
        await page.evaluate('step(2)')
        held = await desk_canvas.get_attribute('data-time')
        await page.evaluate('step(6)')
        assert await desk_canvas.get_attribute('data-time') == held
        assert await control.get_attribute('aria-label') == 'Pause kitchen animation'
        await mission.screenshot(path=str(args.output/'recipe-desk-desktop.png'))
        await page.set_viewport_size({'width':390,'height':844})
        await desk.scroll_into_view_if_needed()
        await page.evaluate('paint()');await page.evaluate('step(3)')
        assert not await page.evaluate('document.documentElement.scrollWidth > innerWidth')
        assert await desk_canvas.evaluate('c=>Math.abs(c.width/c.height-c.clientWidth/c.clientHeight)<.01')
        await mission.screenshot(path=str(args.output/'recipe-desk-mobile.png'))
        await desk_button.click()
        await page.evaluate('step(2)')
        # Compare rendered poses across two restarts. A ticking clock alone can
        # hide a character that has stopped animating.
        first_pose = await desk_canvas.screenshot()
        await page.evaluate('step(1,2500)')
        second_pose = await desk_canvas.screenshot()
        assert first_pose != second_pose, 'Desk characters did not move'
        for _ in range(2):
            await page.evaluate('step(1,23000)')
            assert await desk_canvas.screenshot() == first_pose, 'Desk pose did not repeat'
            await page.evaluate('step(1,2500)')
            assert await desk_canvas.screenshot() == second_pose, 'Desk stopped after repeating'
        assert int(await desk_canvas.get_attribute('data-cycles')) >= 2
        assert await desk_canvas.evaluate('c=>getComputedStyle(c).opacity') == '1'
        await page.evaluate('scrollTo(0,0)');await page.evaluate('paint()');await page.wait_for_timeout(100)
        away = await desk_canvas.get_attribute('data-time')
        await page.evaluate('step(5,500)')
        assert await desk_canvas.get_attribute('data-time') == away
        print('PASS lazy desk loading, independent playback, full loop, offscreen suspension, desktop and mobile', flush=True)
        # A live preference change stops motion; explicit Play still works.
        await page.emulate_media(reduced_motion='reduce');await page.evaluate('paint()')
        await expect(control).to_have_attribute('aria-label','Play kitchen animation');await page.evaluate('step(3)')
        await expect(desk_button).to_have_attribute('aria-label','Play recipe desk animation')
        await page.evaluate('window.oldDesk=document.querySelector("#landing-recipe-desk-canvas")')
        # Route navigation releases the old renderer, and returning mounts exactly one.
        await page.evaluate('window.oldKitchen=document.querySelector("#landing-kitchen-canvas")')
        await page.locator('.landing-more-links a[href="/how-to"]').click();await page.wait_for_selector('#landing-kitchen',state='detached')
        assert await page.evaluate('oldKitchen.dataset.disposed')=='true'
        assert await page.evaluate('oldDesk.dataset.disposed')=='true'
        await page.go_back();await ready(page)
        assert await page.locator('#landing-kitchen-canvas').count()==1
        assert await page.locator('#landing-kitchen button').get_attribute('aria-label')=='Play kitchen animation'
        print('PASS offscreen suspension, reduced motion, route cleanup/remount',flush=True)
        await desk.scroll_into_view_if_needed();await ready(page, '#landing-recipe-desk')
        await expect(desk_button).to_have_attribute('aria-label','Play recipe desk animation')
        await desk_canvas.evaluate("c=>c.getContext('webgl2').getExtension('WEBGL_lose_context').loseContext()")
        await expect(desk).to_have_attribute('data-state','fallback')
        await expect(desk.locator('img')).to_be_visible()
        await page.evaluate('document.querySelector("#landing-kitchen-canvas").getContext("webgl2").getExtension("WEBGL_lose_context").loseContext()')
        await page.wait_for_selector('#landing-kitchen[data-state="fallback"]')
        assert await page.locator('#landing-kitchen img').is_visible()
        await page.screenshot(path=str(args.output/'fallback.png'))
        assert not errors,errors
        await context.close()
        failed=await browser.new_context();page=await failed.new_page()
        await page.route('**/*recipe-desk*.glb',lambda r:r.abort())
        await page.route('**/*penguini-kitchen*.glb',lambda r:r.abort())
        await page.goto(args.url);await page.wait_for_selector('#landing-kitchen[data-state="fallback"]',timeout=120000)
        assert await page.locator('#landing-kitchen img').is_visible()
        assert await page.get_by_role('heading',name='The Ultimate Recipe Manager').is_visible()
        await page.locator('#landing-recipe-desk').scroll_into_view_if_needed()
        await expect(page.locator('#landing-recipe-desk')).to_have_attribute('data-state','fallback')
        await expect(page.locator('#landing-recipe-desk img')).to_be_visible()
        await failed.close()
        nojs=await browser.new_context(java_script_enabled=False);page=await nojs.new_page();await page.goto(args.url)
        assert await page.locator('#landing-kitchen img').is_visible()
        assert await page.locator('#landing-recipe-desk img').is_visible()
        assert await page.locator('.landing-recipe-search input').is_visible()
        print('PASS context loss, failed asset, and no-JavaScript fallback',flush=True)
        await browser.close()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--url',default='http://127.0.0.1:8765/');p.add_argument('--output',type=Path,required=True)
    asyncio.run(main(p.parse_args()))
