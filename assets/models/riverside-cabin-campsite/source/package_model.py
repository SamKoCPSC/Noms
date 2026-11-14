"""Create and integrity-check the versioned review bundle stored alongside the model."""
from pathlib import Path
import zipfile,hashlib,json
root=Path(__file__).resolve().parents[1];name=root.name
report=json.loads((root/'models/validation.json').read_text())
assert report['fresh_blender_glb_import']=='passed'
assert hashlib.sha256((root/'models'/f'{name}.glb').read_bytes()).hexdigest()==report['sha256']
required=[f'models/{name}.blend',f'models/{name}.glb',f'previews/{name}.png','source/build_scene.py','source/validate_model.py','README.md']
required+=['previews/prime-rib-detail.png','previews/plated-dinner-detail.png']if 'kitchen'in name else ['previews/campsite-details.png']
for p in required:assert (root/p).is_file(),p
out=root/'downloads';out.mkdir(exist_ok=True);dest=out/f'{name}-v4.zip'
files=sorted(p for p in root.rglob('*')if p.is_file()and 'downloads'not in p.relative_to(root).parts and '__pycache__'not in p.parts and not p.name.endswith('.blend1'))
with zipfile.ZipFile(dest,'w',zipfile.ZIP_DEFLATED,compresslevel=6)as z:
 for p in files:z.write(p,Path(name)/p.relative_to(root))
 sums=''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(root))+'\n'for p in files)
 z.writestr(name+'/SHA256SUMS',sums)
with zipfile.ZipFile(dest)as z:assert z.testzip() is None
print('VERIFIED_BUNDLE',dest,dest.stat().st_size)
