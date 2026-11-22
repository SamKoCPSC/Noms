"""Export only rebuilt signs from a local preview, preserving the world snapshot.

Build/serve the preview first, then run: export-signage.py http://127.0.0.1:PORT
"""
import asyncio, gzip, hashlib, json, sys
from pathlib import Path
from urllib.parse import urlsplit
from playwright.async_api import async_playwright

ROOT=Path(__file__).resolve().parents[3]
ASSET=ROOT/'assets/models/terraced-village'
DAY=ASSET/'daylight'
def is_sign(name):
    return (name.startswith(('Street plaque:', 'Painted sign ', 'Direction arrow', 'Road sign ', 'Welcome board '))
            or name in ['Street sign timber post','Noms village sign badge']
            or name.startswith('Lettering: ') and not name.removeprefix('Lettering: ').isdigit())

async def main(url):
    assert urlsplit(url).hostname in {'localhost','127.0.0.1','::1'}
    baseline=ASSET/'source/world-environment.json'
    original=json.loads(baseline.read_text())
    original_neighborhood=json.loads((ASSET/'source/neighborhood-verification.json').read_text())
    layout=json.loads((ASSET/'layout.json').read_text())
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        page=await browser.new_page(viewport={'width':900,'height':650});errors=[]
        page.on('pageerror',lambda e:errors.append(str(e)))
        await page.goto(url,timeout=240000)
        await page.wait_for_selector('canvas[data-ready="true"]',timeout=240000)
        current=await page.evaluate('village.neighborhood()')
        # Filter before serializing the large environment across the browser bridge.
        meshes=await page.evaluate('''()=>village.environment().filter(o =>
            /^(Street plaque:|Painted sign |Direction arrow|Road sign |Welcome board )/.test(o.name)
            || ['Street sign timber post','Noms village sign badge'].includes(o.name)
            || o.name.startsWith('Lettering: ') && !/^Lettering: [0-9]+$/.test(o.name))''')
        await browser.close()
    assert not errors,errors
    assert len(current['streets'])==11 and not current['camera_collisions']
    for a,b in zip(current['streets'],original_neighborhood['streets']):
        assert a['street']==b['street'] and a['route']==b['route']
        assert max(abs(x-y) for x,y in zip(a['position'],b['position']))<1e-5
        assert a['destination']==layout['stops'][a['route']+1]['title']
    for mount in current['sign_mounts']:
        assert all(v>0 for k,v in mount.items() if k.endswith('clearance')),mount
    assert len(current['sign_mounts'])==11+current['road_signs']+1
    for name in ['Road sign ochre rim','Road sign cream face','Welcome board frame','Welcome board green face','Direction arrow','Direction arrow reverse']:
        assert [m for m in original if m['name']==name]==[m for m in meshes if m['name']==name],name
    removed=sorted({m['name'] for m in original if is_sign(m['name'])})
    assert all(is_sign(m['name']) for m in meshes)
    result=dict(baseline_sha256=hashlib.sha256(baseline.read_bytes()).hexdigest(),remove_names=removed,meshes=meshes)
    out=DAY/'source/signage.json.gz';out.write_bytes(gzip.compress(json.dumps(result,separators=(',',':')).encode(),mtime=0))
    report=dict(sha256=hashlib.sha256(out.read_bytes()).hexdigest(),baseline_sha256=result['baseline_sha256'],
                replaced_meshes=len([m for m in original if is_sign(m['name'])]),meshes=len(meshes),
                triangles=sum(len(m['indices'])//3 for m in meshes),street_positions_preserved=True,
                non_sign_source_meshes_preserved=True,noticeboard_positions_preserved=True,direction_arrows_preserved=True,sign_mounts=current['sign_mounts'],
                street_destinations=current['streets'],names={s['id']:s['title'] for s in layout['stops']},
                destination_text_width=2.35,arrow_inner_edge=1.26,minimum_text_arrow_gap=.085)
    proof=DAY/'source/signage-verification.json';proof.write_text(json.dumps(report,indent=2)+'\n')
    # Rebuilding an existing layer must not downgrade a newer world revision.
    if layout.get('version',0)<=11:
        layout['version']=11;layout['stage']='Noms village place names and clear sign mounts with shared bright daylight'
    layout['signage']=dict(geometry=str(out.relative_to(ROOT)),verification=str(proof.relative_to(ROOT)))
    (ASSET/'layout.json').write_text(json.dumps(layout,indent=2)+'\n')
    print('SIGNAGE_EXPORTED',json.dumps({k:v for k,v in report.items() if k not in ['street_destinations','sign_mounts']}),flush=True)

if __name__=='__main__':asyncio.run(main(sys.argv[1]))
