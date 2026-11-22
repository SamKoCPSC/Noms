"""Read-only checks for the saved corporate-office master and standard GLB."""
from pathlib import Path
import bpy,json,math,struct
from mathutils import Vector
root=Path(__file__).resolve().parent.parent
scene=bpy.context.scene
bpy.context.view_layer.update()
groups=[o for o in scene.objects if o.type=='EMPTY' and 'assembly' in o]
assert len(groups)==15
members=[o for g in groups for o in g.children_recursive if o.type in ['MESH','FONT','CURVE']]
assert len(members)==len(set(members))
def bounds(items):
    points=[o.matrix_world@Vector(p) for o in items for p in o.bound_box]
    return ([min(p[k] for p in points) for k in range(3)],[max(p[k] for p in points) for k in range(3)])
for o in members:
    lo,hi=bounds([o]);assert all(math.isfinite(v) for v in lo+hi),o.name
    assert lo[0]>=-4.55 and hi[0]<=4.55,(o.name,lo,hi)
    assert lo[1]>=-3.95 and hi[1]<=3.95,(o.name,lo,hi)
    assert lo[2]>=-.27 and hi[2]<=3.65,(o.name,lo,hi)
work=[o for o in scene.objects if o.type=='EMPTY' and o.name.startswith('Workstation ')]
chairs=[o for o in scene.objects if o.type=='EMPTY' and o.name.startswith('Task chair ')]
lunch=[o for o in scene.objects if o.type=='EMPTY' and o.name.startswith('Lunch chair ')]
assert len(work)==3 and len(chairs)==3 and len(lunch)==6
assert len([o for o in members if o.name.startswith('Chair five star foot')])==15
assert len([o for o in members if o.name.startswith('Task chair lumbar support attachment')])==3
assert len([o for o in scene.objects if o.type=='EMPTY' and o.name.startswith('Layered sandwich wedge')])==8
# The sink is a genuine opening rather than a bowl embedded in a solid counter.
basin=bounds([bpy.data.objects['Sink basin floor']])
carcass=bounds([bpy.data.objects['Pantry cabinet carcass']])
assert carcass[1][2]<basin[0][2]
for n in ['Stone counter left of basin','Stone counter right of basin','Stone counter front sink rim','Stone counter rear sink rim']:
    lo,hi=bounds([bpy.data.objects[n]])
    assert not (lo[0]<1.1<hi[0] and lo[1]<3.25<hi[1]),n
# Verify working circulation between desks and table, and behind lunch chairs.
chair_edge=max(bounds(list(o.children_recursive))[1][0] for o in chairs)
table=bounds([bpy.data.objects['Rounded oak communal lunch table rounded top']])
side_aisle=table[0][0]-chair_edge
assert side_aisle>.80,side_aisle
north_edge=max(bounds(list(o.children_recursive))[1][1] for o in lunch)
pantry_front=bounds([bpy.data.objects['Pantry cabinet individual door']])[0][1]
assert pantry_front-north_edge>1.2
assert not [im for im in bpy.data.images if im.source=='FILE' and im.users and not im.packed_file]
raw=(root/'models/corporate-workspace-pantry.glb').read_bytes()
magic,ver,length=struct.unpack_from('<4sII',raw)
assert magic==b'glTF' and ver==2 and length==len(raw)
n=struct.unpack_from('<I',raw,12)[0];doc=json.loads(raw[20:20+n])
assert len(doc['scenes'][0]['nodes'])==15
assert not doc.get('cameras') and not doc.get('animations') and not doc.get('images')
assert not any('Studio' in o.get('name','') for o in doc['nodes'])
tri=sum(doc['accessors'][p['indices']]['count']//3 for m in doc['meshes'] for p in m['primitives'])
report=dict(status='PASS',assemblies=15,objects=len(members),workstations=3,task_chairs=3,lunch_chairs=6,sandwich_wedges=8,
            desk_to_lunch_table_aisle_m=side_aisle,pantry_working_aisle_m=pantry_front-north_edge,glb_bytes=len(raw),triangles=tri,
            checks=['finite bounds within platform','one assembly per object','sink opening and cabinet clearance',
                    'workstation and pantry circulation','three five-star chairs with attached lumbar supports',
                    'portable Blender assets','GLB excludes studio and retains assembly roots'])
(root/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
