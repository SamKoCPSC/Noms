"""Shared settings for native world, room, terrain and vertex-lighting bakes."""
import bpy, json, math, gzip
from pathlib import Path
from mathutils import Matrix, Vector
REPO=Path(__file__).resolve().parents[4]
ASSET=REPO/'assets/models/terraced-village'
DAY=ASSET/'daylight'
CONFIG=json.loads((DAY/'lighting.json').read_text())
LAYOUT=json.loads((ASSET/'layout.json').read_text())
ANCHORS=json.loads((DAY/'source/anchors.json').read_text())
xyz=lambda p:(p[0],-p[2],p[1])
def path_revision():
    import hashlib
    return hashlib.sha256((DAY/'source/stone-paths.json.gz').read_bytes()).hexdigest() if LAYOUT.get('stone_paths') else None
def vegetation_revision():
    import hashlib
    return hashlib.sha256((DAY/'source/vegetation.json.gz').read_bytes()).hexdigest() if LAYOUT.get('vegetation') else None
def signage_revision():
    import hashlib
    return hashlib.sha256((DAY/'source/signage.json.gz').read_bytes()).hexdigest() if LAYOUT.get('signage') else None
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
    items=json.loads((ASSET/'source/world-environment.json').read_text())
    if LAYOUT.get('vegetation'):
        import hashlib
        plants=json.loads(gzip.decompress((DAY/'source/vegetation.json.gz').read_bytes()))
        assert hashlib.sha256((ASSET/'source/world-environment.json').read_bytes()).hexdigest()==plants['baseline_sha256']
        removed=set(plants['remove_indices'])
        items=[item for index,item in enumerate(items) if index not in removed]+plants['meshes']
    if LAYOUT.get('stone_paths'):
        paths=json.loads(gzip.decompress((DAY/'source/stone-paths.json.gz').read_bytes()))
        items=[item for item in items if item['name'] not in paths['remove_names']]+paths['meshes']
    if LAYOUT.get('signage'):
        import hashlib
        signs=json.loads(gzip.decompress((DAY/'source/signage.json.gz').read_bytes()))
        assert hashlib.sha256((ASSET/'source/world-environment.json').read_bytes()).hexdigest()==signs['baseline_sha256']
        items=[item for item in items if item['name'] not in signs['remove_names']]+signs['meshes']
    for i,item in enumerate(items):
        verts=item['vertices'];normals=item['normals'];indices=item['indices']
        inside=[True]*len(verts) if bounds is None else [bounds[0]<=v[0]<=bounds[1] and bounds[2]<=v[2]<=bounds[3] for v in verts]
        faces=[indices[j:j+3] for j in range(0,len(indices),3) if all(inside[k] for k in indices[j:j+3])]
        if not faces:continue
        used=sorted({k for f in faces for k in f});remap={old:new for new,old in enumerate(used)}
        mesh=bpy.data.meshes.new(item['name']);mesh.from_pydata([xyz(verts[k]) for k in used],[],[[remap[k] for k in f] for f in faces]);mesh.update()
        mesh.polygons.foreach_set('use_smooth',[True]*len(mesh.polygons));mesh.normals_split_custom_set_from_vertices([xyz(normals[k]) for k in used])
        vertex_tint='colors' in item
        if vertex_tint:
            tint=mesh.color_attributes.new(name='VegetationTint',type='FLOAT_COLOR',domain='POINT')
            tint.data.foreach_set('color',[c for k in used for c in [*item['colors'][k],1]])
        ground_texture=i==0 and bool(LAYOUT.get('vegetation'))
        color=tuple(item['color']);key=(color,vertex_tint,ground_texture)
        if key not in materials:
            mat=bpy.data.materials.new('Daylight matte '+str(len(materials)));mat.use_nodes=True;mat.diffuse_color=(*color,1)
            p=mat.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1);p.inputs['Roughness'].default_value=1;p.inputs['Specular IOR Level'].default_value=.04;materials[key]=mat
            if vertex_tint:
                node=mat.node_tree.nodes.new('ShaderNodeVertexColor');node.layer_name='VegetationTint'
                mat.node_tree.links.new(node.outputs['Color'],p.inputs['Base Color'])
            if ground_texture:
                nodes=mat.node_tree.nodes;links=mat.node_tree.links
                pos=nodes.new('ShaderNodeNewGeometry');noise=nodes.new('ShaderNodeTexNoise')
                noise.inputs['Scale'].default_value=1.3;noise.inputs['Detail'].default_value=2;noise.inputs['Roughness'].default_value=.65
                links.new(pos.outputs['Position'],noise.inputs['Vector'])
                ramp=nodes.new('ShaderNodeValToRGB')
                ramp.color_ramp.elements[0].position=.15;ramp.color_ramp.elements[0].color=(*(v*.87 for v in color),1)
                ramp.color_ramp.elements[1].position=.85;ramp.color_ramp.elements[1].color=(*(v*1.16 for v in color),1)
                links.new(noise.outputs['Fac'],ramp.inputs['Fac']);links.new(ramp.outputs['Color'],p.inputs['Base Color'])
        mesh.materials.append(materials[key]);obj=bpy.data.objects.new(item['name'],mesh);collection.objects.link(obj);obj['environment_index']=i
    from glass import apply_environment_glass
    apply_environment_glass(collection)
    return collection
def originals():
    import hashlib
    paths=[ASSET/'models/terraced-village-blockout.blend']+[REPO/s['master'] for s in LAYOUT['stops']]
    return {str(p.relative_to(REPO)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
