"""Archive an assembled collection into a temporary library; never save the source.
blender -b -t 2 --python tools/about/village/prepare-master.py -- MODEL TEMP_DIR
"""
import bpy, hashlib, json, sys
from pathlib import Path
from mathutils import Vector

REPO = Path(__file__).resolve().parents[3]
key, temp = sys.argv[sys.argv.index('--') + 1:]
temp = Path(temp); temp.mkdir(parents=True, exist_ok=True)
entry = next(s for s in json.loads((REPO/'assets/models/terraced-village/layout.json').read_text())['stops'] if s['id']==key)
source = REPO/entry['master']; before = hashlib.sha256(source.read_bytes()).hexdigest()
bpy.ops.wm.open_mainfile(filepath=str(source), load_ui=False, use_scripts=False)
scene=bpy.context.scene;scene.frame_set(160);bpy.context.view_layer.update()
roots=[o for o in scene.objects if o.type=='EMPTY' and ('assembly' in o or o.name.startswith('ASM_'))]
assert roots, f'No assemblies in {key}'
keep=set(roots)
for root in roots: keep.update(root.children_recursive)
keep={o for o in keep if o.type not in {'LIGHT','CAMERA'} and not o.hide_render}
# Include current companions using the same placements as the production bake.
plush=REPO/'assets/models/about-penguin-plushes'
row=next(r for r in json.loads((plush/'placements.json').read_text())['models'] if r['model']==key)
for placement in row['plushes']:
    with bpy.data.libraries.load(str(plush/'models/penguini-plush.blend'),link=False) as (src,dst): dst.objects=src.objects
    root=next(o for o in dst.objects if o and o.get('variant')=='penguini')
    added={root,*root.children_recursive}
    for o in added: scene.collection.objects.link(o)
    x,y,z=placement['position']; root.location=(x,-z,y);root.scale=(placement['height'],)*3
    import math
    root.rotation_euler.z=math.radians(placement['yaw_degrees']);keep.update(added)
bpy.context.view_layer.update()
graph=bpy.context.evaluated_depsgraph_get()
geometry=[o for o in keep if o.type in {'MESH','CURVE','FONT'}]
points=[o.matrix_world@Vector(c) for o in geometry for c in o.evaluated_get(graph).bound_box]
lo=Vector([min(p[i] for p in points) for i in range(3)]);hi=Vector([max(p[i] for p in points) for i in range(3)])
transforms={o:o.matrix_world.copy() for o in keep}
# Freeze only the copied world collections at the assembled pose. Preserve mesh,
# shader and modifier data; original source files are byte-identical afterwards.
for o in keep: o.animation_data_clear()
for o in keep:
    o.parent=None
    for constraint in list(o.constraints):o.constraints.remove(constraint)
    o.matrix_world=transforms[o]
collection=bpy.data.collections.new('WORLD_SOURCE_'+key)
for o in keep:collection.objects.link(o)
collection.instance_offset=((lo.x+hi.x)/2,(lo.y+hi.y)/2,lo.z)
collection['source']=entry['master'];collection['source_sha256']=before;collection['assembled_frame']=160
# Namespace copied datablocks before combining twelve large libraries. Otherwise
# Blender's duplicate-name resolution becomes very slow on thousands of Cube/
# Bevel/Material names. This only changes identifiers in the temporary copy.
for datablocks in (bpy.data.objects,bpy.data.meshes,bpy.data.curves,bpy.data.materials,bpy.data.images,bpy.data.node_groups,bpy.data.fonts):
    for block in list(datablocks):
        original=block.name
        if block.is_property_readonly('name'):continue
        block.name=key+'::'+original[:32]+'::'+hashlib.sha1(original.encode()).hexdigest()[:8]
bpy.data.libraries.write(str(temp/f'{key}.blend'),{collection},fake_user=True,compress=True)
report=dict(id=key,source=entry['master'],sha256=before,objects=len(keep),bounds_min=list(lo),bounds_max=list(hi),collection=collection.name)
(temp/f'{key}.json').write_text(json.dumps(report,indent=2)+'\n')
assert hashlib.sha256(source.read_bytes()).hexdigest()==before
print('MASTER_PREPARED',key,len(keep),flush=True)
