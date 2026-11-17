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
        await page.evaluate('step(1,1200)')
        assert 0<math.dist(await eye(),first['eye'])<math.dist(wide,first['eye'])
        await page.evaluate('step(1,1200)')
        assert math.dist(await eye(),first['eye'])<1e-5
        assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='false'
        print('PASS opening approach and first-stop landing',flush=True)
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
        # A 3200ms quintic transition accelerates gently, then brakes symmetrically.
        label=await page.locator('#panel-0 .next-link').text_content()
        assert label.strip()=='Next page',repr(label)
        await page.set_viewport_size({'width':393,'height':852});await settle(page)
        await page.locator('#panel-11 .back-link').evaluate('e=>e.click()')
        samples=[]
        for count in [11,10,10]:
            await page.evaluate('n=>step(n)',count)
            assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='true'
            samples.append(11-float(await page.locator('#story-three-canvas').get_attribute('data-progress')))
        early,middle,late=samples
        assert 0<early<.15 and abs(middle-.5)<.002 and .85<late<1,samples
        assert middle-early>early and abs((1-late)-early)<.002,samples
        await page.evaluate('step(10)');assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='false'
        assert abs(float(await page.locator('#story-three-canvas').get_attribute('data-progress'))-10)<.001
        assert await page.evaluate('document.activeElement.id')=='title-10'
        # A wheel gesture over text scrolls the document and camera, not the card.
        await page.mouse.move(180,250);await page.mouse.wheel(0,-900);await page.wait_for_timeout(300);await settle(page)
        canvas=page.locator('#story-three-canvas')
        paused=float(await canvas.get_attribute('data-progress'))
        assert 9.05<paused<9.95,paused
        await page.evaluate('step(45)')
        assert float(await canvas.get_attribute('data-progress'))==paused
        assert await page.locator('#about-tour').get_attribute('data-camera-moving')=='false'
        assert await page.locator('.story-panel').evaluate_all('ps=>ps.every(p=>p.scrollTop===0)')
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
