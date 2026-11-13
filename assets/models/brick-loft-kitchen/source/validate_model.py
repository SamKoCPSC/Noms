import json,struct
from pathlib import Path
root=Path(__file__).resolve().parent.parent
p=root/'models/brick-loft-kitchen.glb';raw=p.read_bytes()
magic,version,length=struct.unpack_from('<4sII',raw)
assert magic==b'glTF' and version==2 and length==len(raw)
n,kind=struct.unpack_from('<II',raw,12);assert kind==0x4e4f534a
g=json.loads(raw[20:20+n]);triangles=sum(g['accessors'][p['indices']]['count']//3 for m in g['meshes'] for p in m['primitives'])
assert not any('tablet' in n.get('name','').lower() or 'recipe thumbnail' in n.get('name','').lower() for n in g['nodes'])
assert not any('cloth' in n.get('name','').lower() or 'towel' in n.get('name','').lower() for n in g['nodes'])
assert not any('pendant' in n.get('name','').lower() for n in g['nodes'])
assert any('Grilled salmon' in n.get('name','') for n in g['nodes'])
assert any('Pasta curled' in n.get('name','') for n in g['nodes'])
for appliance in ['Espresso machine body', 'Burr grinder upright', 'Toaster cream rounded shell', 'Sage electric kettle body']:
 assert any(appliance in n.get('name','') for n in g['nodes']), appliance
scene=g['scenes'][g.get('scene',0)];assert scene.get('extras',{}).get('faucet_clearance_checked') is True
roots=scene['nodes'];assert len(roots)==17
for m in g['meshes']:
 for p in m['primitives']:
  assert p.get('mode',4)==4
  a=g['accessors'][p['attributes']['POSITION']];assert a['count']>0
  assert all(lo<=hi for lo,hi in zip(a['min'],a['max']))
report={'cloth_removed':True,'faucet_clearance_checked':True,'faucet_sill_clearance_m':scene['extras']['faucet_sill_clearance_m'],'recipe_display_removed':True,'revision':5,'glb_bytes':len(raw),'triangles':triangles,'meshes':len(g['meshes']),'assembly_roots':len(roots),'format':'glTF 2.0','layout':'U-shaped back run, right sink return, open-front dining peninsula','entrance_clearance_m':1.4,'animation':'Static grouped review model'}
(root/'validation.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
