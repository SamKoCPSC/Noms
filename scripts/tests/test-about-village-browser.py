#!/usr/bin/env python3
"""Exercise the actual Rust About component, navigation and full-viewport village.
Run against a local web build: python scripts/tests/test-about-village-browser.py URL OUTPUT
The controlled RAF measures animation time independently of software GPU speed.
"""
import asyncio, base64, json, math, sys
from pathlib import Path
from urllib.parse import urljoin, urlsplit
from playwright.async_api import async_playwright
CLOCK = '''(()=>{const nativeRAF=requestAnimationFrame.bind(window);window.paint=()=>new Promise(r=>nativeRAF(()=>nativeRAF(r)));
let q=new Map(),id=0,t=performance.now();requestAnimationFrame=f=>{q.set(++id,f);return id};cancelAnimationFrame=i=>q.delete(i);
window.step=async(n,dt=80)=>{for(let i=0;i<n;i++){t+=dt;const a=[...q.values()];q.clear();for(const f of a)f(t);await new Promise(r=>setTimeout(r,0));}}})();'''
async def ready(page):
    await page.wait_for_selector('.about-tour.is-flight')
    for _ in range(900):
        await page.evaluate('step(1)'); await page.wait_for_timeout(100)
        if await page.locator('#about-tour').get_attribute('data-entry-state')=='ready' and await page.locator('#story-three-canvas').get_attribute('data-ready')=='true':
            await page.evaluate('paint()'); return
        if await page.locator('#about-tour').get_attribute('data-renderer')=='fallback':
            raise AssertionError('Unexpected reading fallback')
    raise AssertionError('Village did not load')
async def screenshot(page,path):
    cdp=await page.context.new_cdp_session(page)
    result=await cdp.send('Page.captureScreenshot',{'format':'png','fromSurface':False,'captureBeyondViewport':False})
    path.write_bytes(base64.b64decode(result['data'])); await cdp.detach()
async def settle(page):
    await page.evaluate('paint()'); await page.evaluate('step(22)'); await page.evaluate('paint()')
async def geometry(page,i):
    return await page.evaluate('''i=>{const c=document.querySelector('#story-three-canvas'),p=document.querySelector(`#panel-${i}`),s=document.querySelector('#tour-scene-framing');const b=e=>{let r=e.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]};return {canvas:b(c),panel:b(p),slot:b(s),model:JSON.parse(c.dataset.modelBounds),progress:+c.dataset.progress,overflow:p.scrollHeight>p.clientHeight+1,frame:[document.documentElement.clientWidth,innerHeight],scroll:scrollY}}''',i)
async def check_look_around(page):
    root=page.locator('#about-tour')
    canvas=page.locator('#story-three-canvas')
    toggle=page.locator('#village-look-toggle')
    async def pose():
        return await canvas.evaluate('c=>[c.dataset.progress,c.dataset.cameraEye,c.dataset.projection,c.dataset.modelBounds]')
    before=await pose()
    await toggle.click()
    await page.evaluate('step(2)')
    assert await toggle.inner_text()=='Resume tour'
    assert await page.locator('#story').evaluate('s=>s.inert')
    assert await page.locator('#village-look-surface').is_visible()
    centered=await pose()
    mobile=page.viewport_size['width']<=700
    if mobile:
        session=await page.context.new_cdp_session(page)
        await session.send('Emulation.setTouchEmulationEnabled',{'enabled':True})
        for kind,points in [('touchStart',[{'x':190,'y':470}]),('touchMove',[{'x':270,'y':495}]),('touchEnd',[])]:
            await session.send('Input.dispatchTouchEvent',{'type':kind,'touchPoints':points})
        await session.detach()
    else:
        await page.mouse.move(190,470)
        await page.mouse.down()
        await page.mouse.move(270,495)
        await page.mouse.up()
    await page.evaluate('step(2)')
    dragged=await pose()
    assert dragged[:2]==before[:2] and dragged[3]!=centered[3]
    await page.keyboard.press('ArrowLeft')
    await page.mouse.wheel(0,400)
    await page.evaluate('step(2)')
    turned=await pose()
    assert turned[:2]==before[:2] and turned[3]!=dragged[3]
    if mobile:
        await toggle.click()
    else:
        await page.keyboard.press('Escape')
    await page.evaluate('step(1)')
    await page.evaluate('step(1,600)')
    halfway=await pose()
    assert halfway[3]!=turned[3] and halfway[3]!=before[3]
    await page.evaluate('step(1,600)')
    assert await pose()==before
    assert await toggle.inner_text()=='Look around'
    assert not await page.locator('#story').evaluate('s=>s.inert')
    assert await root.get_attribute('data-looking') is None
    print('PASS look around, drag, keyboard and return at the same path position',flush=True)

async def check_explore(page, out):
    canvas = page.locator('#story-three-canvas')
    button = page.locator('#village-look-toggle')
    # Enter midway through the walk to check that the tour can resume between stops.
    await page.locator('.waypoint').first.evaluate('e=>window.scrollBy(0,e.getBoundingClientRect().top+e.clientHeight*.55)')
    await settle(page)
    async def pose():
        return await canvas.evaluate('e=>[JSON.parse(e.dataset.cameraEye),e.dataset.modelBounds,e.dataset.projection,e.dataset.progress]')
    saved = await pose()
    assert abs(float(saved[3])-.55)<.002
    assert await button.inner_text()=='Explore village'
    await button.click()
    await page.evaluate('step(1)')
    await page.evaluate('step(1,1200)')
    lowered = await pose()
    assert lowered[0][1]<saved[0][1]-2 and lowered[3]==saved[3]
    mobile = page.viewport_size['width']<=700
    assert await page.locator('.touch-walk-help' if mobile else '.desktop-walk-help').is_visible()
    if mobile:
        session = await page.context.new_cdp_session(page)
        await session.send('Emulation.setTouchEmulationEnabled',{'enabled':True})
        box = await page.locator('#village-walk-stick').bounding_box()
        finger = {'x':box['x']+box['width']/2,'y':box['y']+box['height']*.2,'id':1}
        async def touch(kind,points):
            await session.send('Input.dispatchTouchEvent',{'type':kind,'touchPoints':points})
        await touch('touchStart',[finger])
        await touch('touchStart',[finger,{'x':240,'y':380,'id':2}])
        await touch('touchMove',[finger,{'x':260,'y':370,'id':2}])
        await page.evaluate('step(7)')
        await touch('touchEnd',[])
        await session.detach()
    else:
        await page.mouse.click(580,350)
        await page.wait_for_function("document.pointerLockElement?.id === 'village-look-surface'",polling=50)
        async def check_speed(expected, ramp_frames):
            await page.evaluate('n=>step(n,50)',ramp_frames)
            start = (await pose())[0]
            await page.evaluate('step(1,50)')
            end = (await pose())[0]
            speed = math.hypot(end[0]-start[0], end[2]-start[2]) / .05
            assert abs(speed-expected)<.05,(speed,expected)
        # Hold Shift before W, release it while walking, then sprint mid-stride.
        await page.keyboard.down('ShiftLeft')
        await page.keyboard.down('w')
        await check_speed(5.,16)
        await page.keyboard.up('ShiftLeft')
        await check_speed(2.5,8)
        await page.keyboard.down('ShiftRight')
        await check_speed(5.,8)
        await page.keyboard.up('ShiftRight')
        await page.mouse.move(600,350)
        await page.keyboard.press('Escape')
        await page.keyboard.up('w')
        await page.wait_for_function('!document.pointerLockElement',polling=50)
    await page.evaluate('step(8)')
    walking = await pose()
    assert math.dist(walking[0],lowered[0])>.2 and walking[3]==saved[3]
    await page.evaluate('step(4)')
    assert await pose()==walking
    await screenshot(page,out/'first-person.png')
    await button.click()
    await page.evaluate('step(1)')
    await page.evaluate('step(1,1201)')
    assert await pose()==saved
    assert await button.inner_text()=='Explore village'
    print('PASS first-person movement and return to fractional tour pose',flush=True)

async def main(url,out):
    assert urlsplit(url).hostname in ['127.0.0.1','localhost'];out.mkdir(parents=True,exist_ok=True)
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        initial_viewport={'width':393,'height':852} if '--mobile' in sys.argv else {'width':1280,'height':900}
        context=await browser.new_context(viewport=initial_viewport,device_scale_factor=1)
        await context.add_init_script(CLOCK);page=await context.new_page();page.set_default_timeout(120000)
        errors=[];requests=[];page.on('pageerror',lambda e:errors.append(str(e)));page.on('request',lambda r:requests.append(r.url))
        await page.goto(url);await ready(page)
        assert await page.locator('#story-three-canvas').get_attribute('data-loaded-models')=='13'
        assert not any('/assets/about/' in u and '.glb' in u and '/village/' not in u for u in requests),'Legacy scenes fetched'
        # The loading curtain reveals a distant kitchen, then the camera
        # reaches exactly the ordinary first-stop pose without scrolling.
        layout_url=await page.locator('#about-tour').get_attribute('data-village-layout-url')
        layout=await (await page.request.get(urljoin(page.url,layout_url))).json()
        first=layout['stops'][0]['pose']
        async def eye():
            return json.loads(await page.locator('#story-three-canvas').get_attribute('data-camera-eye'))
        first_waypoint_top=await page.locator('.waypoint').first.evaluate('e=>e.getBoundingClientRect().top')
        assert abs(first_waypoint_top)<2,first_waypoint_top
        wide=await eye()
        assert math.dist(wide,first['target'])>2*math.dist(first['eye'],first['target'])
        await screenshot(page,out/'opening-wide.png')
        await page.evaluate('step(1)')
        await page.evaluate('step(1,2400)')
        assert 0<math.dist(await eye(),first['eye'])<math.dist(wide,first['eye'])
        await page.evaluate('step(1,2400)')
        assert math.dist(await eye(),first['eye'])<1e-5
        assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='false'
        print('PASS opening approach and first-stop landing',flush=True)
        await check_explore(page,out)
        await page.locator(".waypoint").first.evaluate("e=>e.scrollIntoView({behavior:'instant'})")
        await settle(page)
        results=[]
        for width,height in ([(393,852),(360,640)] if '--mobile' in sys.argv else [(1280,900),(393,852),(360,640)]):
            await page.set_viewport_size({'width':width,'height':height});await settle(page)
            for i in ([0,7,11] if '--smoke' in sys.argv else [0,2,4,6,8,10,1,3,5,7,9,11]):
                await page.evaluate('i=>{location.hash=document.querySelectorAll(".waypoint")[i].id;dispatchEvent(new HashChangeEvent("hashchange"))}',i)
                await page.evaluate('step(1)');await page.evaluate('step(1,1000)');await page.evaluate('paint()');g=await geometry(page,i);results.append(g)
                assert abs(g['progress']-i)<.002,g
                assert g['canvas']==[0,0,g['frame'][0],height],g
                assert not g['overflow'],g
                assert g['panel'][1]>=70 and g['panel'][3]<height-30,g
                if width>700: assert g['panel'][2]<g['model'][0],g
                else: assert g['panel'][3]<g['model'][1],g
                assert g['model'][0]>=-1 and g['model'][2]<=width+1 and g['model'][3]<=height+1,g
                if (width,height) in [(1280,900),(393,852)] and i in [0,1,7,11]: await screenshot(page,out/f'{width}-stop-{i}.png')
                print('PASS framing',width,height,i,flush=True)
        # A 6400ms quintic transition accelerates gently, then brakes symmetrically.
        label=await page.locator('#panel-0 .next-link').text_content()
        assert label.strip()=='Next page',repr(label)
        await page.set_viewport_size({'width':393,'height':852});await settle(page)
        await page.locator('#panel-11 .back-link').evaluate('e=>e.click()')
        samples=[]
        await page.evaluate('step(1)')
        for _ in range(3):
            await page.evaluate('step(1,1600)')
            assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='true'
            samples.append(11-float(await page.locator('#story-three-canvas').get_attribute('data-progress')))
        early,middle,late=samples
        assert 0<early<.15 and abs(middle-.5)<.002 and .85<late<1,samples
        assert middle-early>early and abs((1-late)-early)<.002,samples
        await page.evaluate('step(1,1600)');assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='false'
        assert abs(float(await page.locator('#story-three-canvas').get_attribute('data-progress'))-10)<.001
        assert await page.evaluate('document.activeElement.id')=='title-10'
        # A wheel gesture over text scrolls the document and camera, not the card.
        await page.mouse.move(180,250);await page.mouse.wheel(0,-900);await page.wait_for_timeout(300);await settle(page)
        canvas=page.locator('#story-three-canvas')
        paused=float(await canvas.get_attribute('data-progress'))
        assert abs(paused-(10-900/(3.4*852)))<.005,paused
        await page.evaluate('step(45)')
        assert float(await canvas.get_attribute('data-progress'))==paused
        assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='false'
        assert await page.locator('.story-panel').evaluate_all('ps=>ps.every(p=>p.scrollTop===0)')
        await check_look_around(page)
        await page.locator('#village-view-toggle').click();assert await page.locator('#about-tour').get_attribute('data-renderer')=='fallback'
        await page.locator('#village-view-toggle').click();await settle(page);assert await page.locator('#about-tour').get_attribute('data-renderer')=='three-village'
        await page.locator('#story-three-canvas').evaluate('c=>c.dispatchEvent(new Event("webglcontextlost",{cancelable:true}))')
        assert await page.locator('#about-tour').get_attribute('data-renderer')=='fallback'
        assert await page.locator('.story-panel[inert]').count()==0
        assert not errors,errors
        print('PASS button timing, wheel over text, reading return, context loss',flush=True)
        (out/'geometry.json').write_text(json.dumps(results,indent=2))
        await context.close()
        for case in ['reduced','failure','loading-exit']:
            ctx=await browser.new_context(viewport={'width':393,'height':852},reduced_motion='reduce' if case=='reduced' else 'no-preference')
            tab=await ctx.new_page();seen=[];tab.on('request',lambda r:seen.append(r.url))
            if case=='failure': await ctx.route('**/assets/about/village/layout.json',lambda r:r.abort())
            release=asyncio.Event()
            if case=='loading-exit':
                async def stall(route):
                    await release.wait()
                    try:await route.abort()
                    except Exception:pass
                await ctx.route('**/assets/about/village/*.glb',stall)
            await tab.goto(url)
            if case=='loading-exit':
                await tab.wait_for_selector('#tour-loader-read',state='visible')
                await tab.locator('#tour-loader-read').evaluate('e=>e.click()')
                release.set()
            await tab.wait_for_selector('.about-tour.is-reader')
            assert await tab.locator('.story-panel[inert]').count()==0
            if case=='reduced':assert not any('/village/' in u and '.glb' in u for u in seen)
            print('PASS',case,flush=True);await ctx.close()
        await browser.close()
asyncio.run(main(sys.argv[1],Path(sys.argv[2])))
