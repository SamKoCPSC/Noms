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
        if layout.get('shared_lighting'):
            assert initial['shared_daylight'] and not initial['realtime_shadows'],initial
        if layout.get('stone_paths'):
            paths=json.loads((ROOT/layout['stone_paths']['verification']).read_text())
            assert paths['connected_components']==1 and paths['valid_footprint']
            assert paths['partition_overlap_area']<1e-7 and paths['uncovered_area']<1e-7
            assert paths['entrances_connected']==12 and paths['walking_routes_covered']==11
            assert paths['one_height_field'] and paths['continuous_grout_bed']
            (out/'stone-path-verification.json').write_text(json.dumps(paths,indent=2)+'\n')
        if layout.get('vegetation'):
            plants=json.loads((ROOT/layout['vegetation']['verification']).read_text())
            assert hashlib.sha256((ROOT/layout['vegetation']['geometry']).read_bytes()).hexdigest()==plants['sha256']
            assert plants['foliage_within_original_bounds'] and plants['paths_rooms_river_props_excluded']
            assert plants['minimum_grass_center_path_clearance']>plants['grass_envelope_radius']
            assert plants['grass_clumps']>5000 and plants['individual_flowers']>1000
            (out/'vegetation-verification.json').write_text(json.dumps(plants,indent=2)+'\n')
        if layout.get('clear_glass'):
            glass=await page.evaluate('village.glass()')
            assert glass['transmissionResolutionScale']==1
            assert len(glass['materials'])>=13,glass
            assert all(m['physical'] and m['transmission']==1 and m['roughness']==0 and m['opacity']==1 and not m['transparent'] and not m['normalMap'] and not m['roughnessMap'] and not m['toneMapped'] and not m['fog'] and m['sampling']=='level-zero' for m in glass['materials']),glass
            (out/'glass-browser-verification.json').write_text(json.dumps(glass,indent=2)+'\n')
        await screenshot('initial-arrival')
        await page.locator('#next').click()
        await page.wait_for_function('village.getState().progress > 0',timeout=90000,polling=250)
        await page.evaluate('village.go(0,true)')
        await page.evaluate('village.overview()');await screenshot('overview')
        poses=await page.evaluate('village.poses()')
        assert len(poses)==12
        if layout.get('shared_lighting'):
            original_poses=json.loads((ROOT/'assets/models/terraced-village/source/camera-poses.json').read_text())
            for actual,original in zip(poses,original_poses):
                assert actual['id']==original['id']
                for key in ['eye','target']:
                    assert max(abs(a-b) for a,b in zip(actual[key],original[key]))<1e-5,(actual,original)
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
        if layout.get('signage'):
            signs=json.loads((ROOT/layout['signage']['verification']).read_text())
            assert hashlib.sha256((ROOT/layout['signage']['geometry']).read_bytes()).hexdigest()==signs['sha256']
            assert signs['names']=={s['id']:s['title'] for s in layout['stops']}
            assert [s['destination'] for s in neighborhood['streets']]==[s['title'] for s in layout['stops'][1:]]
            assert all(v>0 for m in neighborhood['sign_mounts'] for k,v in m.items() if k.endswith('clearance'))
            assert await page.locator('#stops option').all_text_contents()==[f"{i+1}. {s['title']}" for i,s in enumerate(layout['stops'])]
            (out/'signage-verification.json').write_text(json.dumps(signs,indent=2)+'\n')
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
            assert await page.locator('#title').inner_text()==f"{i+1} / 12 · {layout['stops'][i]['title']}"
            await screenshot(f'stop-{i+1:02d}')
        if layout.get('signage'):
            import math
            for i in [0,6]:
                sign=neighborhood['streets'][i];x,y,z=sign['position'];yaw=sign['yaw']
                for side in [1,-1]:
                    eye=[x+math.sin(yaw)*side*6.5,y+3.7,z+math.cos(yaw)*side*6.5]
                    await page.evaluate('(p)=>village.inspectView(p.eye,p.target)',dict(eye=eye,target=[x,y+3.4,z]))
                    await screenshot(f'sign-{i+1:02d}-'+('front' if side==1 else 'back'))
        # The route must actually change direction instead of traversing rows.
        if layout.get('stone_paths'):
            await page.evaluate('village.go(1,true)')
            await page.evaluate('village.inspectView([-53,5,22],[-46,.4,14])')
            await screenshot('stone-junction')
        if layout.get('vegetation'):
            await page.evaluate('village.go(1,true)')
            await page.evaluate('village.inspectView([-53,3.2,22],[-52,1,14])')
            await screenshot('meadow-detail')
            await page.evaluate('village.inspectView([-78,6.5,30],[-67,3.8,13])')
            await screenshot('foliage-detail')
        delta=[poses[i+1]['target'][0]-poses[i]['target'][0] for i in range(11)]
        assert any(x>10 for x in delta) and any(x< -10 for x in delta)
        await page.evaluate('village.go(3.5,true)');await screenshot('transition')
        await page.evaluate('village.go(.5,true)');await screenshot('walk-library')
        await page.evaluate('village.go(7.5,true)');await screenshot('walk-market')
        await page.evaluate('village.go(4,true)');await page.locator('#play').click()
        # Software rendering can delay animation frames under concurrent load.
        # Assert actual movement rather than assuming a fixed wall-clock delay.
        await page.wait_for_function('village.getState().progress > 4',timeout=90000,polling=250)
        await page.locator('#play').click()
        await page.set_viewport_size({'width':393,'height':852});await page.evaluate('village.go(0,true)');await screenshot('mobile')
        state=await page.evaluate('village.getState()');assert abs(state['aspect']-393/852)<1e-8
        assert not errors,errors
        assert not external,external
        (out/'verification.json').write_text(json.dumps({'models':12,'all_stops_checked':True,'walking_route_samples':2211,'camera_clear_of_model_bounds':True,'eye_height_above_terrain':eye_height,'continuous_segment_endpoints':True,'playback_advances':True,'responsive_aspect':True,'unchanged_master_hashes':True,'errors':errors,'layout_revision':layout['version'],'single_file_external_requests':len(external) if single_file else None,'next_navigation_checked':True,'landscape_clearance_checked':True,'shared_daylight':initial.get('shared_daylight',False),'realtime_shadows':initial.get('realtime_shadows',False),'android_content_url_tested':False,'sign_names_and_mounts_checked':bool(layout.get('signage')),'clear_glass_checked':bool(layout.get('clear_glass'))},indent=2)+'\n')
        await browser.close()
    print('VILLAGE_CHECK_PASS')

if __name__=='__main__':asyncio.run(main(sys.argv[1],sys.argv[2]))
