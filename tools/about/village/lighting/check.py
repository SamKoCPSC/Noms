"""Verify the portable lighting comparison with external asset requests blocked.
python tools/about/village/lighting/check.py http://127.0.0.1:8783/OPEN-KITCHEN-LIGHTING.html OUTPUT
"""
import asyncio, base64, hashlib, json, sys
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright

REPO=Path(__file__).resolve().parents[4]
async def main(url,out):
    assert urlsplit(url).hostname in {'127.0.0.1','localhost','::1'}
    out=Path(out);out.mkdir(parents=True,exist_ok=True)
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        page=await browser.new_page(viewport={'width':1200,'height':900});errors=[];requests=[]
        page.on('pageerror',lambda e:errors.append(str(e)))
        page.on('console',lambda m:errors.append(m.text) if m.type=='error' else None)
        async def isolate(route):
            if route.request.is_navigation_request():await route.continue_()
            else:requests.append(route.request.url);await route.abort()
        await page.route('http://**/*',isolate);await page.route('https://**/*',isolate)
        await page.goto(url,timeout=180000)
        await page.wait_for_selector('canvas[data-ready="true"]',timeout=180000)
        async def screenshot(name):
            cdp=await page.context.new_cdp_session(page)
            shot=await cdp.send('Page.captureScreenshot',{'format':'png','fromSurface':False,'captureBeyondViewport':False})
            (out/(name+'.png')).write_bytes(base64.b64decode(shot['data']));await cdp.detach()
        states=[]
        for view in ['arrival','path','wide']:
            await page.select_option('#view',view)
            for mode in ['before','after']:
                await page.click('#'+mode);await page.wait_for_timeout(500)
                state=await page.evaluate('lightingStudy.state()');assert state['mode']==mode and not state['shadows']
                states.append(dict(view=view,**state));await screenshot(view+'-'+mode)
            assert states[-1]['eye']==states[-2]['eye'] and states[-1]['target']==states[-2]['target']
        await page.select_option('#view','arrival')
        start=await page.evaluate('lightingStudy.state()');await page.click('#walk')
        await page.wait_for_timeout(2000);during=await page.evaluate('lightingStudy.state()')
        assert during['walking'] and during['eye']!=start['eye']
        await page.click('#walk');assert not (await page.evaluate('lightingStudy.state()'))['walking']
        await page.set_viewport_size({'width':393,'height':852});await page.select_option('#view','arrival')
        for mode in ['before','after']:
            await page.click('#'+mode);await page.wait_for_timeout(500);await screenshot('mobile-'+mode)
        assert abs((await page.evaluate('lightingStudy.state()'))['aspect']-393/852)<1e-8
        assert not errors,errors
        assert not requests,requests
        report=dict(views=states,identical_comparison_cameras=True,external_asset_requests=requests,browser_errors=errors,
            portrait_aspect_verified=True,walking_camera_advances=True,realtime_shadow_maps=False,android_content_url_tested=False)
        (out/'browser-verification.json').write_text(json.dumps(report,indent=2)+'\n');await browser.close()
    print('LIGHTING_STUDY_CHECK_PASS',flush=True)

if __name__=='__main__':asyncio.run(main(sys.argv[1],sys.argv[2]))
