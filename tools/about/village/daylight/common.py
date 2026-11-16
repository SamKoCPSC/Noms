"""Shared settings for native world, room, terrain and vertex-lighting bakes."""
import bpy, json, math
from pathlib import Path
from mathutils import Matrix, Vector
REPO=Path(__file__).resolve().parents[4]
ASSET=REPO/'assets/models/terraced-village'
DAY=ASSET/'daylight'
CONFIG=json.loads((DAY/'lighting.json').read_text())
LAYOUT=json.loads((ASSET/'layout.json').read_text())
ANCHORS=json.loads((DAY/'source/anchors.json').read_text())
xyz=lambda p:(p[0],-p[2],p[1])
def anchor(key):
    a=ANCHORS[key];lo,hi=a['min'],a['max'];center=[(lo[i]+hi[i])/2 for i in range(3)]
    return Matrix.Translation(xyz(a['position']))@Matrix.Rotation(a['yaw'],4,'Z')@Matrix.Scale(a['scale'],4)@Matrix.Translation((-center[0],center[2],-lo[1]))
def lighting(scene):
    for obj in list(scene.objects):
        if obj.type=='LIGHT':bpy.data.objects.remove(obj,do_unlink=True)
    world=bpy.data.worlds.new('Village shared bright daylight');world.use_nodes=True;scene.world=world
    bg=world.node_tree.nodes.get('Background');bg.inputs['Color'].default_value=(*CONFIG['sky_color'],1);bg.inputs['Strength'].default_value=CONFIG['sky_strength']
    data=bpy.data.lights.new('Village shared warm sun','SUN');data.energy=CONFIG['sun_energy'];data.color=CONFIG['sun_color'];data.angle=math.radians(CONFIG['sun_angle_degrees'])
    light=bpy.data.objects.new(data.name,data);scene.collection.objects.link(light)
    light.rotation_euler=(-Vector(xyz(CONFIG['sun_direction_y_up']))).to_track_quat('-Z','Y').to_euler()
    scene.render.engine='CYCLES';scene.cycles.use_denoising=False;scene.cycles.samples=CONFIG['room_samples']
    scene.view_settings.view_transform=CONFIG['view_transform'];scene.view_settings.look=CONFIG['look'];scene.view_settings.exposure=CONFIG['exposure']
    scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_mode='RGB';scene.render.image_settings.color_depth='8'
    scene['shared_daylight']=json.dumps(CONFIG,sort_keys=True)
def camera(scene,key):
    pose=next(p for p in json.loads((ASSET/'source/camera-poses.json').read_text()) if p['id']==key)
    data=bpy.data.cameras.new(key+' walking camera');cam=bpy.data.objects.new(data.name,data);scene.collection.objects.link(cam)
    cam.location=xyz(pose['eye']);cam.rotation_euler=(Vector(xyz(pose['target']))-cam.location).to_track_quat('-Z','Y').to_euler()
    data.sensor_fit='VERTICAL';data.sensor_height=24;data.lens=24/(2*math.tan(math.radians(60)/2));data.clip_end=650;scene.camera=cam
    scene.render.resolution_x=1000;scene.render.resolution_y=750;scene.render.resolution_percentage=100
def environment(scene,bounds=None):
    collection=bpy.data.collections.new('Daylight terrain');scene.collection.children.link(collection);materials={}
    for i,item in enumerate(json.loads((ASSET/'source/world-environment.json').read_text())):
        verts=item['vertices'];normals=item['normals'];indices=item['indices']
        inside=[True]*len(verts) if bounds is None else [bounds[0]<=v[0]<=bounds[1] and bounds[2]<=v[2]<=bounds[3] for v in verts]
        faces=[indices[j:j+3] for j in range(0,len(indices),3) if all(inside[k] for k in indices[j:j+3])]
        if not faces:continue
        used=sorted({k for f in faces for k in f});remap={old:new for new,old in enumerate(used)}
        mesh=bpy.data.meshes.new(item['name']);mesh.from_pydata([xyz(verts[k]) for k in used],[],[[remap[k] for k in f] for f in faces]);mesh.update()
        mesh.polygons.foreach_set('use_smooth',[True]*len(mesh.polygons));mesh.normals_split_custom_set_from_vertices([xyz(normals[k]) for k in used])
        key=tuple(item['color'])
        if key not in materials:
            mat=bpy.data.materials.new('Daylight matte '+str(len(materials)));mat.use_nodes=True;mat.diffuse_color=(*key,1)
            p=mat.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*key,1);p.inputs['Roughness'].default_value=1;p.inputs['Specular IOR Level'].default_value=.04;materials[key]=mat
        mesh.materials.append(materials[key]);obj=bpy.data.objects.new(item['name'],mesh);collection.objects.link(obj);obj['environment_index']=i
    return collection
def originals():
    import hashlib
    paths=[ASSET/'models/terraced-village-blockout.blend']+[REPO/s['master'] for s in LAYOUT['stops']]
    return {str(p.relative_to(REPO)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
