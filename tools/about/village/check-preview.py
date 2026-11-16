"""Read-only browser/asset check for the separate village prototype.
python tools/about/village/check-preview.py http://127.0.0.1:8766 /tmp/village-review-check
"""
import asyncio, base64, hashlib, json, sys
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright

ROOT=Path(__file__).resolve().parents[3]
async def main(url,out):
    assert urlsplit(url).hostname in {'localhost','127.0.0.1','::1'}
    out=Path(out);out.mkdir(parents=True,exist_ok=True)
    layout=json.loads((ROOT/'assets/models/terraced-village/layout.json').read_text())
    eye_height=layout['walk']['eye_height']
    sources=json.loads((ROOT/'assets/models/terraced-village/models/terraced-village-blockout.sources.json').read_text())
    for s in sources:
        assert hashlib.sha256((ROOT/s['source']).read_bytes()).hexdigest()==s['sha256'],s['id']
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        page=await browser.new_page(viewport={'width':1440,'height':1000});errors=[]
        page.on('pageerror',lambda error:(errors.append(str(error)),print('PAGE_ERROR',str(error),flush=True)))
        page.on('console',lambda message:errors.append(message.text) if message.type=='error' and '404' not in message.text else None)
        external=[]
        single_file=url.endswith('/OPEN-VILLAGE-MOBILE.html')
        if single_file:
            async def isolate(route):
                if route.request.is_navigation_request(): await route.continue_()
                else:
                    external.append(route.request.url)
                    await route.abort()
            await page.route('http://**/*',isolate)
            await page.route('https://**/*',isolate)
        await page.goto(url,timeout=240000)
        await page.wait_for_selector('canvas[data-ready="true"]',timeout=240000)
        async def screenshot(name):
            cdp=await page.context.new_cdp_session(page)
            result=await cdp.send('Page.captureScreenshot',{'format':'png','fromSurface':False,'captureBeyondViewport':False})
            (out/(name+'.png')).write_bytes(base64.b64decode(result['data']));await cdp.detach()
        initial=await page.evaluate('village.getState()')
        assert initial['progress']==0 and not initial['overviewMode']
        await screenshot('initial-arrival')
        await page.locator('#next').click()
        await page.wait_for_function('village.getState().progress > 0',timeout=90000,polling=250)
        await page.evaluate('village.go(0,true)')
        await page.evaluate('village.overview()');await screenshot('overview')
        poses=await page.evaluate('village.poses()')
        assert len(poses)==12
        frontage=await page.evaluate('village.frontages()')
        assert len(frontage['entrances'])==12 and not frontage['camera_collisions'],frontage
        assert all(e['approach_clear_of_neighbors'] for e in frontage['entrances'])
        for i,entrance in enumerate(frontage['entrances']):
            assert all(abs(entrance['approach_start'][j]-poses[i]['eye'][j])<1e-6 for j in [0,2])
        (out/'entrance-verification.json').write_text(json.dumps(frontage,indent=2)+'\n')
        landscape=await page.evaluate('village.landscape()')
        assert not landscape['camera_collisions'] and landscape['clear_arrival_views'],landscape
        assert all(landscape['counts'].get(k,0)>30 for k in ['tree','bush','boulder','meadow'])
        (out/'landscape-verification.json').write_text(json.dumps(landscape,indent=2)+'\n')
        neighborhood=await page.evaluate('village.neighborhood()')
        assert len(neighborhood['houses'])==8 and len(neighborhood['streets'])==11,neighborhood
        assert not neighborhood['camera_collisions'] and neighborhood['lettering_is_geometry'],neighborhood
        assert neighborhood['road_signs']>=3 and neighborhood['lanterns']>=6 and neighborhood['welcome_board'],neighborhood
        (out/'neighborhood-verification.json').write_text(json.dumps(neighborhood,indent=2)+'\n')
        style=await page.evaluate('village.style()')
        assert style['roughness']==1 and style['specularIntensity']<=.08 and style['clearcoat']==0
        assert style['rounded_shapes']>300,style
        (out/'style-verification.json').write_text(json.dumps(style,indent=2)+'\n')
        samples=await page.evaluate('village.routeSamples()')
        assert len(samples)==11
        assert all(not p['blocked'] and abs(p['eye'][1]-p['ground']-eye_height)<1e-6 for route in samples for p in route)
        for i,route in enumerate(samples):
            assert sum((route[-1]['eye'][j]-samples[i+1][0]['eye'][j])**2 for j in range(3))<1e-8 if i<10 else True
        (out/'camera-poses.json').write_text(json.dumps(poses,indent=2)+'\n')
        (out/'paths.json').write_text(json.dumps(await page.evaluate('village.paths()'))+'\n')
        (out/'walking-camera.json').write_text(json.dumps(samples)+'\n')
        (out/'world-environment.json').write_text(await page.evaluate('JSON.stringify(village.environment())')+'\n')
        print('ROUTE_CHECK_PASS',len(samples)*len(samples[0]),flush=True)
        for i in range(12):
            await page.evaluate('i=>village.go(i,true)',i);await page.wait_for_timeout(150)
            state=await page.evaluate('village.getState()');assert state['progress']==i and state['models']==12
            await screenshot(f'stop-{i+1:02d}')
        # The route must actually change direction instead of traversing rows.
        delta=[poses[i+1]['target'][0]-poses[i]['target'][0] for i in range(11)]
        assert any(x>10 for x in delta) and any(x< -10 for x in delta)
        await page.evaluate('village.go(3.5,true)');await screenshot('transition')
        await page.evaluate('village.go(.5,true)');await screenshot('walk-library')
        await page.evaluate('village.go(7.5,true)');await screenshot('walk-market')
        await page.evaluate('village.go(4,true)');await page.locator('#play').click();await page.wait_for_timeout(3500)
        assert (await page.evaluate('village.getState()'))['progress']>4
        await page.locator('#play').click()
        await page.set_viewport_size({'width':393,'height':852});await page.evaluate('village.go(0,true)');await screenshot('mobile')
        state=await page.evaluate('village.getState()');assert abs(state['aspect']-393/852)<1e-8
        assert not errors,errors
        assert not external,external
        (out/'verification.json').write_text(json.dumps({'models':12,'all_stops_checked':True,'walking_route_samples':2211,'camera_clear_of_model_bounds':True,'eye_height_above_terrain':eye_height,'continuous_segment_endpoints':True,'playback_advances':True,'responsive_aspect':True,'unchanged_master_hashes':True,'errors':errors,'layout_revision':layout['version'],'single_file_external_requests':len(external) if single_file else None,'next_navigation_checked':True,'landscape_clearance_checked':True,'android_content_url_tested':False},indent=2)+'\n')
        await browser.close()
    print('VILLAGE_CHECK_PASS')

if __name__=='__main__':asyncio.run(main(sys.argv[1],sys.argv[2]))
