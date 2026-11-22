"""Validate the delivered GLB and its independently movable assembly groups."""
import json,struct,math
from pathlib import Path
root=Path(__file__).resolve().parent.parent
p=root/'models/modern-baking-lab.glb';raw=p.read_bytes()
magic,version,length=struct.unpack_from('<4sII',raw)
assert magic==b'glTF' and version==2 and length==len(raw)
n,kind=struct.unpack_from('<II',raw,12);assert kind==0x4e4f534a
g=json.loads(raw[20:20+n]);scene=g['scenes'][g.get('scene',0)]
assert scene.get('extras',{}).get('revision')==3
roots=scene['nodes'];assert len(roots)==24
names=[n.get('name','')for n in g['nodes']]
for expected in ['Hollow inset stainless sink','Oven insulated side','Proofer full door glass','Angled clear front pane','Mixer beater shaft','Laminated butter croissant','Cinnamon spiral','Cooling rack square upright','Sheeter cream woven conveyor','Sheeter safety grille','Ingredient fridge upper door','Retail fridge clear door','Retail wall oak shelf','POS tablet body','Patisserie clear front','Front-of-shop chalk menu']:
 assert any(expected in name for name in names),expected
assert not any('Temporary basin aperture' in name or 'Studio ground' in name for name in names)
assert not g.get('cameras') and not g.get('animations')
triangles=0
for m in g['meshes']:
 for part in m['primitives']:
  assert part.get('mode',4)==4
  triangles+=g['accessors'][part['indices']]['count']//3
  a=g['accessors'][part['attributes']['POSITION']]
  assert a['count']>0 and all(math.isfinite(v)for v in a['min']+a['max'])
  assert all(lo<=hi for lo,hi in zip(a['min'],a['max']))
for i in roots:
 node=g['nodes'][i];assert 'assembly' in node.get('extras',{}) and node.get('children')
for img in g.get('images',[]):assert 'uri' not in img or img['uri'].startswith('data:')
report=dict(model='Noms Modern Baking Lab',revision=3,concept=10,format='glTF 2.0',glb_bytes=len(raw),triangles=triangles,meshes=len(g['meshes']),assembly_roots=len(roots),externally_referenced_images=False,studio_excluded=True,animation='Static review model with separate assembly roots',geometry='Evaluated Blender geometry, no decimation')
(root/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
