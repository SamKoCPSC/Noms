"""Package the already-built folder as a standalone HTML and a local-server ZIP."""
from pathlib import Path
import base64, json, sys, zipfile

folder=Path(sys.argv[1]);output=Path(sys.argv[2]);output.mkdir(parents=True,exist_ok=True)
layout=json.loads((folder/'layout.json').read_text())
html=(folder/'index.html').read_text()
destination=output/'noms-village-layout-preview.html'
with destination.open('w') as f:
    f.write(html.replace('<script type="module" src="preview.js"></script></html>',''))
    f.write('<script type="text/plain" id="droid-font-notice">'+(folder/'DROID-FONT-NOTICE.txt').read_text().replace('</',r'<\/')+'</script>\n')
    f.write('<script>globalThis.__VILLAGE_LAYOUT__='+json.dumps(layout).replace('</',r'<\/')+';</script>\n')
    for s in layout['stops']:
        f.write(f'<script type="application/octet-stream" id="asset-{s["id"]}">')
        f.write(base64.b64encode((folder/s['model']).read_bytes()).decode('ascii'))
        f.write('</script>\n')
    f.write('<script type="module">'+(folder/'preview.js').read_text().replace('</script',r'<\/script')+'</script></html>')
readme='''Noms village layout review

Open noms-village-layout-preview.html directly in Chrome or Edge. It contains
all twelve original production models and works without a server or internet.
The initial file is large because full model geometry and textures are preserved.

Alternatively, extract noms-village-preview-server.zip and run START.py with
Python 3. This opens http://localhost:8766 using a server bound to your own machine.

Controls: Play tour / Pause, Overview, previous / next, choose a stop, or scrub
the timeline. Drag to orbit, scroll/pinch to zoom. Keyboard arrows change stops;
space starts/stops playback when a form control is not focused.

This is a separate layout review, not the deployed About page. All twelve models
are resident for the overview, so desktop is recommended for the initial review.
The terrain, elevated paths, backdrop and sky are placeholders. Existing scene
lighting is retained; no shared world lighting bake has been performed yet.
'''
(output/'READ-ME.txt').write_text(readme)
(folder/'READ-ME.txt').write_text(readme)
with zipfile.ZipFile(output/'noms-village-preview-server.zip','w',zipfile.ZIP_DEFLATED,compresslevel=4) as z:
    for path in sorted(folder.rglob('*')):
        if path.is_file() and not path.is_symlink():z.write(path,path.relative_to(folder))
print(destination)
