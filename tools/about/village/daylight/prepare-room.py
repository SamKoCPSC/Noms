"""Prepare a room in its village position with nearby shadow/bounce contributors."""
import bpy, json, sys
from pathlib import Path
from mathutils import Matrix, Vector
sys.path.insert(0,str(Path(__file__).resolve().parent))
from common import *
from glass import apply_room_glass, glass_revision
key,out=sys.argv[sys.argv.index('--')+1:sys.argv.index('--')+3];out=Path(out);out.mkdir(parents=True,exist_ok=True)
if '--glass-source' in sys.argv:
    source=Path(sys.argv[sys.argv.index('--glass-source')+1]);glass_report=json.loads(source.with_suffix('.json').read_text())
    assert glass_report['revision']==glass_revision()
    import hashlib
    assert glass_report['cached_blend_sha256']==hashlib.sha256(source.read_bytes()).hexdigest()
    bpy.ops.wm.open_mainfile(filepath=str(source),load_ui=False,use_scripts=False)
else:
    bpy.ops.wm.open_mainfile(filepath=str(ASSET/'models/terraced-village-blockout.blend'),load_ui=False,use_scripts=False)
    glass_report=apply_room_glass()
scene=bpy.data.scenes.new('Daylight room');bpy.context.window.scene=scene
entry=next(s for s in LAYOUT['stops'] if s['id']==key);x,_,z=entry['position'];r=CONFIG['context_radius']
near=[s['id'] for s in LAYOUT['stops'] if max(abs(s['position'][0]-x),abs(s['position'][2]-z))<=r]
print('SELECT_CONTEXT',key,near,flush=True)
# Opening the packed world once avoids repeatedly appending thousands of
# interdependent datablocks. Only the selected neighborhood survives below.
for col in [bpy.data.collections['WORLD_SOURCE_'+k] for k in near]:
    k=col.name.removeprefix('WORLD_SOURCE_')
    if k==key:
        scene.collection.children.link(col);objects=[o for o in col.all_objects if o.type in {'MESH','CURVE','FONT'} and not o.hide_render]
        bpy.ops.object.select_all(action='DESELECT')
        for o in objects:o.select_set(True)
        bpy.context.view_layer.objects.active=objects[0]
        if any(o.type!='MESH' or o.modifiers for o in objects):
            bpy.ops.object.convert(target='MESH');objects=list(bpy.context.selected_objects)
        # Reading dimensions after every transform forces thousands of global
        # dependency-graph updates. Snapshot them before changing any object.
        prepared=[(o,'architecture' if max(o.dimensions)>.85 else 'details',o.matrix_world.copy()) for o in objects]
        placement=anchor(k)
        print('PLACE_OBJECTS',key,len(prepared),flush=True)
        for o,group,matrix in prepared:
            o['bake_group']=group;o['source_object']=o.name
            o.matrix_world=placement@matrix;o.animation_data_clear()
        for o in list(col.objects):
            if o not in objects:bpy.data.objects.remove(o,do_unlink=True)
    else:
        instance=bpy.data.objects.new('Context '+k,None);instance.instance_type='COLLECTION';instance.instance_collection=col;instance.matrix_world=anchor(k)@Matrix.Translation(col.instance_offset);scene.collection.objects.link(instance)
for other in list(bpy.data.scenes):
    if other!=scene:bpy.data.scenes.remove(other)
bpy.data.orphans_purge(do_recursive=True)
print('PREPARE_TERRAIN',key,flush=True)
environment(scene,[x-r,x+r,z-r,z+r]);lighting(scene);camera(scene,key)
scene['scope']='Full village daylight bake: '+key
bpy.context.preferences.filepaths.save_version=0;bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(out/'scene.blend'),compress=True)
meta=dict(glass_revision=glass_revision(),glass=glass_report['rooms'][key],model=key,lighting=CONFIG,anchor=ANCHORS[key],source_hashes=originals(),context=near,context_radius=r,objects=len(objects))
(out/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n')
if '--reference' in sys.argv:
    scene.cycles.samples=32;scene.render.filepath=str(out/'reference.png');bpy.ops.render.render(write_still=True)
print('ROOM_PREPARED',key,flush=True)
