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

async def rendered_pose(page, canvas):
    # The test clock advances Three.js separately from Chromium's compositor.
    await page.evaluate('paint()')
    return await canvas.screenshot()

async def check_chapter_layout(page, chapter, model, aspect, output, stem):
    for width, name in [(1440, 'desktop'), (390, 'mobile')]:
        await page.set_viewport_size({'width':width,'height':900})
        await chapter.evaluate("e => e.scrollIntoView({block:'center',behavior:'instant'})")
        await page.evaluate('paint()');await page.evaluate('step(3)')
        assert not await page.evaluate('document.documentElement.scrollWidth > innerWidth')
        assert await model.locator("canvas").evaluate("(c, ratio) => Math.abs(c.clientWidth / c.clientHeight - ratio) < .02", aspect)
        text_box = await chapter.locator('.landing-chapter-copy p').bounding_box()
        model_box = await model.bounding_box()
        image_box = await chapter.locator('.landing-app-window .landing-shot-button').bounding_box()
        assert model_box['y'] >= text_box['y'] + text_box['height']
        if name == 'desktop':
            if await chapter.evaluate("e => e.matches(':nth-child(even)')"):
                assert model_box['x'] >= image_box['x'] + image_box['width']
            else:
                assert image_box['x'] >= model_box['x'] + model_box['width']
        else:
            assert image_box['y'] >= model_box['y'] + model_box['height']
        await chapter.screenshot(path=str(output / f'{stem}-{name}.png'))

async def main(args):
    assert urlsplit(args.url).hostname in {'localhost','127.0.0.1','::1'}
    args.output.mkdir(parents=True,exist_ok=True)
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        context=await browser.new_context(viewport={'width':1440,'height':650},reduced_motion='no-preference')
        await context.add_init_script(CLOCK)
        page=await context.new_page();page.set_default_timeout(120000)
        errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
        model_requests = []
        page.on('request', lambda req: model_requests.append(req.url) if '.glb' in req.url else None)
        await page.goto(args.url);await ready(page)
        assert all('penguini-kitchen' in url for url in model_requests)
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
        assert sum('recipe-desk' in url for url in model_requests) == 1
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
        first_pose = await rendered_pose(page, desk_canvas)
        await page.evaluate('step(1,2500)')
        second_pose = await rendered_pose(page, desk_canvas)
        assert first_pose != second_pose, 'Desk characters did not move'
        for _ in range(2):
            await page.evaluate('step(1,23000)')
            assert await rendered_pose(page, desk_canvas) == first_pose, 'Desk pose did not repeat'
            await page.evaluate('step(1,2500)')
            assert await rendered_pose(page, desk_canvas) == second_pose, 'Desk stopped after repeating'
        assert int(await desk_canvas.get_attribute('data-cycles')) >= 2
        assert await desk_canvas.evaluate('c=>getComputedStyle(c).opacity') == '1'
        await page.evaluate('scrollTo(0,0)');await page.evaluate('paint()');await page.wait_for_timeout(100)
        away = await desk_canvas.get_attribute('data-time')
        await page.evaluate('step(5,500)')
        assert await desk_canvas.get_attribute('data-time') == away
        print('PASS lazy desk loading, independent playback, full loop, offscreen suspension, desktop and mobile', flush=True)
        chapter = page.locator('.landing-chapter').filter(has=page.get_by_role('heading', name='Discover new recipes.'))
        banquet = chapter.locator('.landing-chapter-copy #landing-pandacotta-banquet')
        banquet_canvas = banquet.locator('canvas')
        banquet_button = banquet.get_by_role('button')
        await page.set_viewport_size({'width':1440,'height':900})
        await chapter.evaluate("e => e.scrollIntoView({block:'center',behavior:'instant'})")
        await ready(page, '#landing-pandacotta-banquet')
        assert sum('pandacotta-banquet' in url for url in model_requests) == 1
        assert await banquet_canvas.get_attribute('data-duration') == '33'
        await expect(banquet_button).to_have_attribute('aria-label', 'Pause banquet animation')
        await expect(banquet_button).to_have_attribute('aria-controls', 'landing-pandacotta-banquet-canvas')
        # Jump to the cake reveal and verify the rendered pose repeats on the next cycle.
        await expect(banquet_canvas).to_be_in_viewport(ratio=.9)
        await page.evaluate('paint()');await page.evaluate('step(2)')
        elapsed = float(await banquet_canvas.get_attribute('data-time'))
        await page.evaluate('ms => step(1,ms)', ((22.6 - elapsed) % 33) * 1000)
        await expect(banquet_canvas).to_have_attribute('data-time', '22.600')
        reveal = await rendered_pose(page, banquet_canvas)
        await page.evaluate('step(1,3000)')
        assert await rendered_pose(page, banquet_canvas) != reveal
        await page.evaluate('step(1,30000)')
        assert await rendered_pose(page, banquet_canvas) == reveal
        await banquet_button.click()
        await page.evaluate('step(4)')
        held = await banquet_canvas.get_attribute('data-time')
        await page.evaluate('step(4)')
        assert await banquet_canvas.get_attribute('data-time') == held
        await expect(desk_button).to_have_attribute('aria-label', 'Pause recipe desk animation')
        await check_chapter_layout(page, chapter, banquet, 7/4, args.output, 'pandacotta-discover')
        print('PASS lazy banquet loading, looping reveal, independent pause, desktop and mobile framing', flush=True)
        cooking = page.locator('.landing-chapter').filter(has=page.get_by_role('heading', name='Make cooking your favorites convenient'))
        island = cooking.locator('.landing-chapter-copy #landing-owlfredo-kitchen-island')
        island_canvas = island.locator('canvas')
        island_button = island.get_by_role('button')
        await island.scroll_into_view_if_needed()
        await ready(page, '#landing-owlfredo-kitchen-island')
        assert sum('owlfredo-kitchen-island' in url for url in model_requests) == 1
        await expect(island_canvas).to_have_attribute('data-duration', '28')
        await expect(island_button).to_have_attribute('aria-label', 'Pause soup animation')
        await island_canvas.scroll_into_view_if_needed();await page.evaluate('paint()');await page.evaluate('step(2)')
        pose = await rendered_pose(page, island_canvas)
        await page.evaluate('step(1,28000)')
        assert await rendered_pose(page, island_canvas) == pose
        await page.evaluate('step(1,8300)')
        assert await rendered_pose(page, island_canvas) != pose
        await island_button.click();await page.evaluate('step(2)')
        held = await island_canvas.get_attribute('data-time')
        await page.evaluate('step(5)')
        assert await island_canvas.get_attribute('data-time') == held
        await check_chapter_layout(page, cooking, island, 4/3, args.output, 'owlfredo-island')
        print('PASS soup island placement, full loop, pause and responsive framing', flush=True)
        history = page.locator('.landing-chapter').filter(has=page.get_by_role('heading', name='Study how your recipes change.'))
        cookies = history.locator('.landing-chapter-copy #landing-terrimisu-cookie-study')
        await cookies.scroll_into_view_if_needed()
        await ready(page, '#landing-terrimisu-cookie-study')
        cookie_canvas = cookies.locator('canvas')
        cookie_button = cookies.get_by_role('button')
        await expect(cookie_canvas).to_have_attribute('data-duration', '24')
        await expect(cookie_button).to_have_attribute('aria-label', 'Pause cookie tasting animation')
        await expect(cookie_button).to_have_attribute('aria-controls', 'landing-terrimisu-cookie-study-canvas')
        await page.evaluate('paint()');await page.evaluate('step(2)')
        elapsed = float(await cookie_canvas.get_attribute('data-time'))
        await page.evaluate('ms => step(1,ms)', ((6.55 - elapsed) % 24) * 1000)
        skeptical = await rendered_pose(page, cookie_canvas)
        await page.evaluate('step(1,8325)')
        happy = await rendered_pose(page, cookie_canvas)
        assert happy != skeptical
        await page.evaluate('step(1,24000)')
        assert await rendered_pose(page, cookie_canvas) == happy
        await cookie_button.click();await page.evaluate('step(2)')
        held = await cookie_canvas.get_attribute('data-time')
        await page.evaluate('step(4,200)')
        assert await cookie_canvas.get_attribute('data-time') == held
        await check_chapter_layout(page, history, cookies, 8/7, args.output, 'terrimisu-cookie-study')
        await page.evaluate('window.oldCookies=document.querySelector("#landing-terrimisu-cookie-study-canvas")')
        print('PASS cookie tasting poses, loop, independent pause, and history chapter layout', flush=True)
        anywhere = page.get_by_role('region', name='Use anywhere')
        gallery = anywhere.locator('.landing-mobile-previews')
        for title in ('Browse recipes on mobile', 'Edit recipes on mobile', 'Cook on mobile'):
            thumbnail = gallery.get_by_role('button', name=f'Enlarge screenshot: {title}')
            await thumbnail.click()
            await expect(page.get_by_role('dialog', name=title)).to_be_visible()
            await page.keyboard.press('Escape')
            await expect(thumbnail).to_be_focused()
        await gallery.get_by_role('button', name='Pause cooking preview').click()
        await expect(gallery.locator('.landing-cooking-frames')).to_have_class('landing-cooking-frames is-paused')
        await expect(cooking.get_by_role('button', name='Pause cooking preview')).to_have_attribute('aria-pressed', 'false')
        picnic = anywhere.locator('#landing-cattatouille-picnic')
        picnic_canvas = picnic.locator('canvas')
        picnic_button = picnic.get_by_role('button')
        await picnic.scroll_into_view_if_needed()
        await ready(page, '#landing-cattatouille-picnic')
        await expect(picnic_button).to_have_attribute('aria-label', 'Pause picnic animation')
        duration = float(await picnic_canvas.get_attribute('data-duration'))
        assert abs(duration - 2111 / 24) < .001
        await page.evaluate('step(3)')
        start = float(await picnic_canvas.get_attribute('data-time'))
        await page.evaluate('ms => step(1, ms)', duration * 1000)
        assert abs(float(await picnic_canvas.get_attribute('data-time')) - start) < .005
        await picnic_button.click()
        await page.evaluate('step(2)')
        held = await picnic_canvas.get_attribute('data-time')
        await page.evaluate('step(4)')
        assert await picnic_canvas.get_attribute('data-time') == held
        await picnic_canvas.evaluate('c => window.oldPicnic = c')
        # A live preference change stops motion; explicit Play still works.
        await page.emulate_media(reduced_motion='reduce');await page.evaluate('paint()')
        await expect(control).to_have_attribute('aria-label','Play kitchen animation');await page.evaluate('step(3)')
        await expect(desk_button).to_have_attribute('aria-label','Play recipe desk animation')
        await page.evaluate('window.oldDesk=document.querySelector("#landing-recipe-desk-canvas")')
        await page.evaluate('window.oldIsland=document.querySelector("#landing-owlfredo-kitchen-island-canvas")')
        await page.evaluate('window.oldBanquet=document.querySelector("#landing-pandacotta-banquet-canvas")')
        # Route navigation releases the old renderer, and returning mounts exactly one.
        await page.evaluate('window.oldKitchen=document.querySelector("#landing-kitchen-canvas")')
        await page.locator('.landing-more-links a[href="/how-to"]').click();await page.wait_for_selector('#landing-kitchen',state='detached')
        assert await page.evaluate('oldKitchen.dataset.disposed')=='true'
        assert await page.evaluate('oldPicnic.dataset.disposed')=='true'
        assert await page.evaluate('oldCookies.dataset.disposed')=='true'
        assert await page.evaluate('oldIsland.dataset.disposed')=='true'
        assert await page.evaluate('oldDesk.dataset.disposed')=='true'
        assert await page.evaluate('oldBanquet.dataset.disposed')=='true'
        await page.go_back();await ready(page)
        assert await page.locator('#landing-kitchen-canvas').count()==1
        assert await page.locator('#landing-kitchen button').get_attribute('aria-label')=='Play kitchen animation'
        print('PASS offscreen suspension, reduced motion, route cleanup/remount',flush=True)
        previous_picnic_loads = sum('cattatouille-picnic' in url for url in model_requests)
        await picnic.scroll_into_view_if_needed()
        await expect(picnic_button).to_have_attribute('aria-label', 'Play picnic animation')
        await expect(picnic.locator('img')).to_be_visible()
        assert sum('cattatouille-picnic' in url for url in model_requests) == previous_picnic_loads
        await picnic_button.click()
        await ready(page, '#landing-cattatouille-picnic')
        assert sum('cattatouille-picnic' in url for url in model_requests) == previous_picnic_loads + 1
        previous_cookie_loads = sum('terrimisu-cookie-study' in url for url in model_requests)
        await cookies.scroll_into_view_if_needed()
        await expect(cookie_button).to_have_attribute('aria-label', 'Play cookie tasting animation')
        await expect(cookies.locator('img')).to_be_visible()
        assert sum('terrimisu-cookie-study' in url for url in model_requests) == previous_cookie_loads
        await cookie_button.click();await ready(page, '#landing-terrimisu-cookie-study')
        assert sum('terrimisu-cookie-study' in url for url in model_requests) == previous_cookie_loads + 1
        await banquet.scroll_into_view_if_needed();await ready(page, '#landing-pandacotta-banquet')
        await expect(banquet_button).to_have_attribute('aria-label', 'Play banquet animation')
        assert await banquet_canvas.get_attribute('data-time') == '22.600'
        await banquet_button.click();await page.evaluate('step(4)')
        assert 0 < float(await banquet_canvas.get_attribute('data-time')) < 1
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
        await page.route('**/*owlfredo-kitchen-island*.glb',lambda r:r.abort())
        await page.route('**/*recipe-desk*.glb',lambda r:r.abort())
        await page.route('**/*pandacotta-banquet*.glb',lambda r:r.abort())
        await page.route('**/*terrimisu-cookie-study*.glb',lambda r:r.abort())
        await page.route('**/*cattatouille-picnic*.glb',lambda r:r.abort())
        await page.route('**/*penguini-kitchen*.glb',lambda r:r.abort())
        await page.goto(args.url);await page.wait_for_selector('#landing-kitchen[data-state="fallback"]',timeout=120000)
        assert await page.locator('#landing-kitchen img').is_visible()
        assert await page.get_by_role('heading',name='The Ultimate Recipe Manager').is_visible()
        await page.locator('#landing-recipe-desk').scroll_into_view_if_needed()
        await expect(page.locator('#landing-recipe-desk')).to_have_attribute('data-state','fallback')
        await expect(page.locator('#landing-recipe-desk img')).to_be_visible()
        await page.locator('#landing-pandacotta-banquet').scroll_into_view_if_needed()
        await expect(page.locator('#landing-pandacotta-banquet')).to_have_attribute('data-state','fallback')
        await expect(page.locator('#landing-pandacotta-banquet img')).to_be_visible()
        await page.locator('#landing-owlfredo-kitchen-island').scroll_into_view_if_needed()
        await expect(page.locator('#landing-owlfredo-kitchen-island')).to_have_attribute('data-state','fallback')
        await expect(page.locator('#landing-owlfredo-kitchen-island img')).to_be_visible()
        await page.locator('#landing-terrimisu-cookie-study').scroll_into_view_if_needed()
        await expect(page.locator('#landing-terrimisu-cookie-study')).to_have_attribute('data-state','fallback')
        await expect(page.locator('#landing-terrimisu-cookie-study img')).to_be_visible()
        await page.locator('#landing-cattatouille-picnic').scroll_into_view_if_needed()
        await expect(page.locator('#landing-cattatouille-picnic')).to_have_attribute('data-state','fallback')
        await expect(page.locator('#landing-cattatouille-picnic img')).to_be_visible()
        await failed.close()
        nojs=await browser.new_context(java_script_enabled=False);page=await nojs.new_page();await page.goto(args.url)
        assert await page.locator('#landing-kitchen img').is_visible()
        assert await page.locator('#landing-recipe-desk img').is_visible()
        assert await page.locator('#landing-pandacotta-banquet img').is_visible()
        assert await page.locator('#landing-owlfredo-kitchen-island img').is_visible()
        assert await page.locator('#landing-terrimisu-cookie-study img').is_visible()
        assert await page.locator('#landing-cattatouille-picnic img').is_visible()
        assert await page.locator('.landing-recipe-search input').is_visible()
        print('PASS context loss, failed asset, and no-JavaScript fallback',flush=True)
        await browser.close()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--url',default='http://127.0.0.1:8765/');p.add_argument('--output',type=Path,required=True)
    asyncio.run(main(p.parse_args()))
