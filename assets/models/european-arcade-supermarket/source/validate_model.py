"""Validate the saved model, selected fixture clearances and review GLB."""
import bpy, json, math, struct
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parent.parent
scene=bpy.context.scene
bpy.context.view_layer.update()
groups=[o for o in scene.objects if o.type=='EMPTY' and 'assembly' in o]
assert len(groups)==17
members=[o for g in groups for o in g.children_recursive if o.type in ('MESH','CURVE','FONT')]
assert len(members)==len(set(members))
def bounds(objs):
    pts=[o.matrix_world@Vector(c) for o in objs for c in o.bound_box]
    return ([min(p[k] for p in pts) for k in range(3)],[max(p[k] for p in pts) for k in range(3)])
for o in members:
    lo,hi=bounds([o]);assert all(math.isfinite(v) for v in lo+hi),o.name
    assert lo[0]>=-6.65 and hi[0]<=6.65,(o.name,lo,hi)
    assert lo[1]>=-6.2 and hi[1]<=6.2,(o.name,lo,hi)
    assert lo[2]>=-.31 and hi[2]<=5.50,(o.name,lo,hi)
assert len([o for o in members if o.name.startswith('Solid arched plaster spandrel')])==3
assert len([o for o in members if o.name.startswith('Arcade supporting pier')])==4
assert len([o for o in scene.objects if o.name.startswith('Grocery gondola ') and o.type=='EMPTY'])==3
assert len([o for o in scene.objects if o.name.startswith('Compact checkout ') and o.type=='EMPTY'])==2
assert len([o for o in scene.objects if o.name.startswith('Open wire shopping cart ') and o.type=='EMPTY'])==2
# Named assemblies retain >=1.45m clear between grocery rows.
aisles=[bounds(list(bpy.data.objects['Grocery gondola '+str(i)].children_recursive)) for i in [1,2,3]]
clearances=[aisles[i+1][0][0]-aisles[i][1][0] for i in [0,1]]
assert min(clearances)>1.45,clearances
# Staff access strip between service ledge and preparation cupboards.
ledge=bounds([bpy.data.objects['Deli staff service ledge']])
prep=bounds([bpy.data.objects['Deli preparation stone top']])
service_gap=prep[0][1]-ledge[1][1]
assert service_gap>.90,service_gap
assert not [im for im in bpy.data.images if im.source=='FILE' and im.users and not im.packed_file]
raw=(root/'models/european-arcade-supermarket.glb').read_bytes()
magic,ver,length=struct.unpack_from('<4sII',raw)
assert magic==b'glTF' and ver==2 and length==len(raw)
n=struct.unpack_from('<I',raw,12)[0];doc=json.loads(raw[20:20+n])
assert len(doc['scenes'][0]['nodes'])==17
assert not doc.get('cameras') and not doc.get('animations') and not doc.get('images')
assert not any('Studio' in o.get('name','') for o in doc['nodes'])
tri=sum(doc['accessors'][p['indices']]['count']//3 for m in doc['meshes'] for p in m['primitives'])
report=dict(status='PASS',assemblies=17,objects=len(members),arches=3,aisles=3,checkouts=2,
            shopping_carts=2,aisle_clearance_m=clearances,deli_staff_clearance_m=service_gap,
            glb_bytes=len(raw),triangles=tri,external_textures=0,
            checks=['finite geometry inside platform footprint','one assembly per object',
                    'grocery aisle and deli staff clearances','native assets are portable',
                    'standard GLB excludes studio and retains seventeen assembly roots'])
(root/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
