"""Build the native walking world from preserved collections and browser geometry.
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/build-master.py -- TEMP_DIR OUTPUT.blend
An existing OUTPUT is reused to avoid re-appending all native model libraries.
"""
import bpy, json, math, sys
from pathlib import Path
from mathutils import Vector
REPO=Path(__file__).resolve().parents[3]
temp,output=map(Path,sys.argv[sys.argv.index('--')+1:]);output.parent.mkdir(parents=True,exist_ok=True)
layout=json.loads((REPO/'assets/models/terraced-village/layout.json').read_text())
source=REPO/'assets/models/terraced-village/source'
environment=json.loads((source/'world-environment.json').read_text())
walks=json.loads((source/'walking-camera.json').read_text())
receipts=[]
if output.exists():
    bpy.ops.wm.open_mainfile(filepath=str(output),load_ui=False,use_scripts=False)
    # Preserve native model collections and their shaders, replace world-only data.
    old_terrain=bpy.data.collections.get('World terrain — blockout')
    if old_terrain:
        for obj in list(old_terrain.all_objects):bpy.data.objects.remove(obj,do_unlink=True)
        bpy.data.collections.remove(old_terrain)
    for obj in list(bpy.context.scene.objects):
        if obj.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(obj,do_unlink=True)
    bpy.context.scene.timeline_markers.clear()
else:
    bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene
terrain=bpy.data.collections.new('World terrain — blockout');scene.collection.children.link(terrain)
models=bpy.data.collections.get('Twelve preserved model collections')
if models is None:models=bpy.data.collections.new('Twelve preserved model collections');scene.collection.children.link(models)
def xyz(p):return (p[0],-p[2],p[1])
for i,s in enumerate(layout['stops']):
    report=json.loads((temp/f"{s['id']}.json").read_text());receipts.append(report)
    col=bpy.data.collections.get(report['collection'])
    if col is None:
        print('APPENDING',s['id'],flush=True)
        with bpy.data.libraries.load(str(temp/f"{s['id']}.blend"),link=False) as (src,dst):dst.collections=[report['collection']]
        col=dst.collections[0]
    root=next((o for o in models.objects if o.get('stop')==i+1),None)
    if root is None:
        root=bpy.data.objects.new(f"{i+1:02d} {s['title']}",None);models.objects.link(root)
    root.instance_type='COLLECTION';root.instance_collection=col
    size=Vector(report['bounds_max'])-Vector(report['bounds_min']);scale=s['width']/max(size.x,size.y)
    root.location=xyz(s['position']);root.scale=(scale,)*3;root.rotation_euler.z=s['yaw'];root['stop']=i+1;root['source_sha256']=report['sha256']
materials={}
for item in environment:
    key=tuple(item['color'])
    if key not in materials:
        material=bpy.data.materials.new('Walking landscape '+str(len(materials)));material.use_nodes=True
        material.diffuse_color=(*key,1)
        material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value=(*key,1)
        material.node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.95
        materials[key]=material
    indices=item['indices'];faces=[tuple(indices[i:i+3]) for i in range(0,len(indices),3)]
    mesh=bpy.data.meshes.new(item['name']);mesh.from_pydata([xyz(v) for v in item['vertices']],[],faces);mesh.update()
    obj=bpy.data.objects.new(item['name'],mesh);mesh.materials.append(materials[key]);terrain.objects.link(obj)
world=bpy.data.worlds.new('Outdoor daytime sky — provisional');scene.world=world;world.use_nodes=True
nodes=world.node_tree.nodes;sky=nodes.new('ShaderNodeTexSky');sky.sky_type='NISHITA';sky.sun_elevation=math.radians(40);sky.sun_rotation=math.radians(135)
world.node_tree.links.new(sky.outputs['Color'],nodes.get('Background').inputs['Color']);nodes.get('Background').inputs['Strength'].default_value=.25
ld=bpy.data.lights.new('Shared sun — provisional','SUN');ld.energy=2;ld.angle=.15;sun=bpy.data.objects.new(ld.name,ld);scene.collection.objects.link(sun);sun.rotation_euler=(.55,-.4,-.65)
camdata=bpy.data.cameras.new('Village walking camera');cam=bpy.data.objects.new('Village walking camera',camdata);scene.collection.objects.link(cam)
cam.rotation_mode='QUATERNION';camdata.sensor_fit='VERTICAL';camdata.sensor_height=24
camdata.lens=24/(2*math.tan(math.radians(60)/2));camdata.clip_end=700
scene.render.fps=30;frame=1;previous=None
# Sample the same ground-following path and look-ahead targets as the browser.
def camera_key(sample,frame):
    global previous
    cam.location=xyz(sample['eye']);direction=Vector(xyz(sample['target']))-cam.location
    rotation=direction.to_track_quat('-Z','Y')
    if previous is not None and rotation.dot(previous)<0:rotation.negate()
    cam.rotation_quaternion=rotation;previous=rotation.copy()
    cam.keyframe_insert('location',frame=frame);cam.keyframe_insert('rotation_quaternion',frame=frame)
for i,route in enumerate(walks):
    scene.timeline_markers.new(f"{i+1:02d} {layout['stops'][i]['title']}",frame=frame)
    camera_key(route[0],frame);camera_key(route[0],frame+75);frame+=75
    length=sum((Vector(b['eye'])-Vector(a['eye'])).length for a,b in zip(route,route[1:]))
    duration=max(1,round(length/layout['walk']['speed']*scene.render.fps))
    for j,sample in enumerate(route[1:],1):camera_key(sample,frame+duration*j/(len(route)-1))
    frame+=duration
scene.timeline_markers.new('12 Garden cookout',frame=frame);camera_key(walks[-1][-1],frame+75);scene.frame_end=frame+75
for curve in cam.animation_data.action.fcurves:
    for point in curve.keyframe_points:point.interpolation='LINEAR'
overview_data=camdata.copy();overview=bpy.data.objects.new('Village overview — static',overview_data);scene.collection.objects.link(overview)
overview_data.lens=24/(2*math.tan(math.radians(43)/2))
overview.location=xyz((85,110,115));overview.rotation_euler=(Vector(xyz((0,0,-25)))-overview.location).to_track_quat('-Z','Y').to_euler()
scene.camera=cam;scene.frame_set(1)
scene.render.engine='CYCLES';scene.cycles.samples=16;scene.cycles.use_denoising=False
scene.render.resolution_x=1600;scene.render.resolution_y=1100;scene.render.resolution_percentage=100
scene.view_settings.view_transform='AgX';scene['stage']=layout['stage'];scene['originals_unchanged']=True;scene['walk_eye_height']=layout['walk']['eye_height']
bpy.context.preferences.filepaths.save_version=0
bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(output),compress=True)
output.with_suffix('.sources.json').write_text(json.dumps(receipts,indent=2)+'\n')
print('WORLD_MASTER_SAVED',str(output),flush=True)
