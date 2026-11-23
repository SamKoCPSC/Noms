"""Run with Blender to check native assembly bounds and standalone GLB import."""
import bpy,json,math
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parent.parent
bpy.ops.wm.open_mainfile(filepath=str(root/'models/modern-baking-lab.blend'))
groups=[o for o in bpy.data.objects if o.type=='EMPTY' and 'assembly' in o]
assert len(groups)==24
assert bpy.context.scene.camera and bpy.context.scene.camera.data.type=='ORTHO'
def bounds(name):
 o=bpy.data.objects[name].evaluated_get(bpy.context.evaluated_depsgraph_get());p=[o.matrix_world@Vector(v)for v in o.bound_box]
 return [[min(v[i]for v in p)for i in range(3)],[max(v[i]for v in p)for i in range(3)]]
case=bounds('Bread case counter');island=bounds('Thick pale oak island edge')
clearance=island[0][0]-case[1][0];assert clearance>.65,clearance
assert abs((island[0][0]+island[1][0])/2-1.87)<.001
rack=bounds('Cooling sheet baking sheet');rack_clearance=rack[0][0]-island[1][0];assert rack_clearance>.80,rack_clearance
print('RACK_CLEARANCE',round(rack_clearance,3),flush=True)
left=bounds('Left oak cap');back=bounds('Rear oak cap');assert left[1][1]<=back[0][1]+.0001
assert not any('Temporary' in o.name for o in bpy.data.objects)
front_case=bounds('Chilled display metal worktop');service_aisle=island[0][1]-front_case[1][1];assert service_aisle>.95,service_aisle
wall_shelves=[o for o in bpy.data.objects if o.name.startswith('Retail wall oak shelf')]
for o in wall_shelves:assert bounds(o.name)[0][2]>bounds('Display clear glass roof')[1][2]+.09
print('SERVICE_AISLE',round(service_aisle,3),flush=True)
print('NATIVE_OK',len(groups),'groups; aisle',round(clearance,3),flush=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(root/'models/modern-baking-lab.glb'))
assert len([o for o in bpy.data.objects if o.type=='EMPTY' and 'assembly' in o])==24
meshes=[o for o in bpy.data.objects if o.type=='MESH'];assert meshes
for o in meshes:
 assert all(math.isfinite(c)for v in o.data.vertices for c in v.co)
assert not any(o.type in ['CAMERA','LIGHT'] for o in bpy.data.objects)
print('GLB_IMPORT_OK',len(meshes),'mesh objects',flush=True)
