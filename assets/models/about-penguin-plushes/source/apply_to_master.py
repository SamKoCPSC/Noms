"""Add the same plushes as separate editable objects to a native master copy.

blender -b INPUT.blend --python .../apply_to_master.py -- MODEL OUTPUT.blend
The existing master is never overwritten. Production GLBs use the append-only
export pass in tools/about/add_penguin_plushes.py to preserve their source data.
"""
import bpy
import json
import math
import sys
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
model,output=sys.argv[sys.argv.index('--')+1:]
output=Path(output).resolve()
assert output != Path(bpy.data.filepath).resolve(), 'Save a separate master copy'
row=next(r for r in json.loads((ROOT/'placements.json').read_text())['models'] if r['model']==model)
assert not any(o.get('noms_plush_overlay') for o in bpy.data.objects), 'Plushes already applied'
scene=bpy.context.scene
group=bpy.data.objects.new('99 Penguin plush companions ROOT',None)
scene.collection.objects.link(group)
group['assembly']='99 Penguin plush companions'
group['start_frame']=110;group['end_frame']=148;group['incoming_offset']=[.15,-.4,.55]
group['noms_plush_overlay']=True
positions=[]
for i,p in enumerate(row['plushes']):
    with bpy.data.libraries.load(str(ROOT/'models/penguin-plush-variants.blend'),link=False) as (src,dst):
        dst.objects=src.objects
    root=next(o for o in dst.objects if o and o.get('variant')==p['variant'])
    keep={root,*root.children_recursive}
    for o in dst.objects:
        if o in keep:scene.collection.objects.link(o)
        elif o:bpy.data.objects.remove(o,do_unlink=True)
    x,y,z=p['position'];root.location=(x,-z,y)
    root.scale=(p['height']/.98,)*3
    root.rotation_euler.z=math.radians(p['yaw_degrees'])
    root.parent=group;root.name=f'Penguin plush {i+1:02} - {p["label"]}'
    positions.append(Vector(root.location))
group['pivot']=list(sum(positions,Vector())/len(positions))
scene['penguin_plushes_added']=len(row['plushes'])
output.parent.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.save_as_mainfile(filepath=str(output))
