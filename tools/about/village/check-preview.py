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
    sources=json.loads((ROOT/'assets/models/terraced-village/models/terraced-village-blockout.sources.json').read_text())
    for s in sources:
        assert hashlib.sha256((ROOT/s['source']).read_bytes()).hexdigest()==s['sha256'],s['id']
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        page=await browser.new_page(viewport={'width':1440,'height':1000});errors=[]
        page.on('pageerror',lambda error:errors.append(str(error)))
        page.on('console',lambda message:errors.append(message.text) if message.type=='error' and '404' not in message.text else None)
        await page.goto(url,timeout=240000)
        await page.wait_for_selector('canvas[data-ready="true"]',timeout=240000)
        async def screenshot(name):
            cdp=await page.context.new_cdp_session(page)
            result=await cdp.send('Page.captureScreenshot',{'format':'png','fromSurface':False,'captureBeyondViewport':False})
            (out/(name+'.png')).write_bytes(base64.b64decode(result['data']));await cdp.detach()
        await screenshot('overview')
        poses=await page.evaluate('village.poses()')
        assert len(poses)==12
        for i in range(12):
            await page.evaluate('i=>village.go(i,true)',i);await page.wait_for_timeout(150)
            state=await page.evaluate('village.getState()');assert state['progress']==i and state['models']==12
            await screenshot(f'stop-{i+1:02d}')
        # The route must actually change direction instead of traversing rows.
        delta=[poses[i+1]['target'][0]-poses[i]['target'][0] for i in range(11)]
        assert any(x>10 for x in delta) and any(x< -10 for x in delta)
        await page.evaluate('village.go(3.5,true)');await screenshot('transition')
        await page.evaluate('village.go(4,true)');await page.locator('#play').click();await page.wait_for_timeout(3500)
        assert (await page.evaluate('village.getState()'))['progress']>4
        await page.locator('#play').click()
        await page.set_viewport_size({'width':393,'height':852});await page.evaluate('village.go(0,true)');await screenshot('mobile')
        state=await page.evaluate('village.getState()');assert abs(state['aspect']-393/852)<1e-8
        assert not errors,errors
        (out/'verification.json').write_text(json.dumps({'models':12,'all_stops_checked':True,'playback_advances':True,'responsive_aspect':True,'unchanged_master_hashes':True,'errors':errors},indent=2)+'\n')
        await browser.close()
    print('VILLAGE_CHECK_PASS')

if __name__=='__main__':asyncio.run(main(sys.argv[1],sys.argv[2]))
