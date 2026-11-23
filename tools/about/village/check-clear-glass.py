"""Pixel regression: sharp detail behind physical glass, without alpha blending."""
import asyncio,functools,json,subprocess,sys,threading
from http.server import SimpleHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
from playwright.async_api import async_playwright
ROOT=Path(__file__).resolve().parents[3]
async def main(out):
 out.mkdir(parents=True,exist_ok=True)
 subprocess.run([str(ROOT/'node_modules/esbuild/bin/esbuild'),'tools/about/village/tests/clear-glass.js','--bundle','--format=esm',f'--outfile={out}/test.js'],cwd=ROOT,check=True)
 (out/'index.html').write_text('<!doctype html><meta charset="utf-8"><script type="module" src="test.js"></script>')
 server=ThreadingHTTPServer(('127.0.0.1',0),functools.partial(SimpleHTTPRequestHandler,directory=str(out)))
 thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
 try:
  async with async_playwright() as p:
   browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
   page=await browser.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
   await page.goto(f'http://127.0.0.1:{server.server_port}');await page.wait_for_function('window.results',timeout=120000)
   result=await page.evaluate('window.results');assert not errors,errors
   assert result['physical'] and result['transmission']==1 and result['ior']==1.45 and result['opacity']==1 and not result['alphaBlended']
   # Three applies fog after output conversion, so its linear transmission target
   # and direct sRGB output differ slightly. Check removal of the extra haze,
   # rather than requiring pixel equality between those different render paths.
   assert not result['fogAppliedTwice'],result
   assert result['fogError']['polished']<result['fogError']['standard']*.75,result
   assert result['fogError']['standard']-result['fogError']['polished']>10,result
   assert result['polished']['softPixels']<=result['uncovered']['softPixels']+2,result
   assert result['standard']['softPixels']>result['polished']['softPixels']+10,result
   assert result['polished']['range'][0]<10 and result['polished']['range'][1]>240,result
   (out/'verification.json').write_text(json.dumps(result,indent=2)+'\n')
   await page.screenshot(path=str(out/'clear-glass.png'))
   print('CLEAR_GLASS_PIXEL_CHECK_PASS',json.dumps({k:v['softPixels'] for k,v in result.items() if isinstance(v,dict) and 'softPixels' in v}),flush=True)
   await browser.close()
 finally:server.shutdown();server.server_close()
asyncio.run(main(Path(sys.argv[1]).resolve()))
