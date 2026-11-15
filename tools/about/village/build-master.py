"""Assemble portable native model collections and rough terrain into a Blender world.
blender -b -t 2 --python tools/about/village/build-master.py -- TEMP_DIR OUTPUT.blend
"""
import bpy, bmesh, json, math, sys
from pathlib import Path
from mathutils import Vector
REPO=Path(__file__).resolve().parents[3]
temp,output=map(Path,sys.argv[sys.argv.index('--')+1:]);output.parent.mkdir(parents=True,exist_ok=True)
layout=json.loads((REPO/'assets/models/terraced-village/layout.json').read_text())
bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene
terrain=bpy.data.collections.new('World terrain — blockout');scene.collection.children.link(terrain)
models=bpy.data.collections.new('Twelve preserved model collections');scene.collection.children.link(models)
materials={}
def material(name,color):
    if name not in materials:
        m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True
        m.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value=(*color,1)
        m.node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.9;materials[name]=m
    return materials[name]
stone=material('Warm retaining stone',(.57,.49,.37));grass=material('Sage terrain',(.34,.40,.24));paving=material('Sandstone paths',(.68,.59,.44));water=material('River placeholder',(.22,.46,.55))
def xyz(p):return (p[0],-p[2],p[1])
def cube(name,p,size,mat):
    mesh=bpy.data.meshes.new(name);bm=bmesh.new();bmesh.ops.create_cube(bm,size=1);bm.to_mesh(mesh);bm.free()
    o=bpy.data.objects.new(name,mesh);o.location=xyz(p);o.scale=(size[0],size[2],size[1]);o.data.materials.append(mat)
    terrain.objects.link(o);return o
def path(name,points,radius,mat):
    curve=bpy.data.curves.new(name,'CURVE');curve.dimensions='3D';curve.bevel_depth=radius;curve.bevel_resolution=2
    spline=curve.splines.new('BEZIER');spline.bezier_points.add(len(points)-1)
    for b,p in zip(spline.bezier_points,points):b.co=xyz(p);b.handle_left_type=b.handle_right_type='AUTO'
    o=bpy.data.objects.new(name,curve);terrain.objects.link(o);o.data.materials.append(mat);return o
# Low common ground supports every terrace; pads are deliberately undecorated.
outline=layout['ground_outline'];n=len(outline)
verts=[(x,-z,y) for y in (-3,-1) for x,z in outline]
faces=[tuple(reversed(range(n))),tuple(range(n,2*n))]+[(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]
mesh=bpy.data.meshes.new('Continuous land');mesh.from_pydata(verts,[],faces);mesh.update();ground=bpy.data.objects.new('Continuous village ground',mesh);terrain.objects.link(ground);ground.data.materials.append(grass)
receipts=[];route=[]
libraries={}
for s in layout['stops']:
    report=json.loads((temp/f"{s['id']}.json").read_text())
    print('APPENDING',s['id'],flush=True)
    with bpy.data.libraries.load(str(temp/f"{s['id']}.blend"),link=False) as (src,dst):dst.collections=[report['collection']]
    libraries[s['id']]=dst.collections[0]
for i,s in enumerate(layout['stops']):
    report=json.loads((temp/f"{s['id']}.json").read_text());receipts.append(report)
    col=libraries[s['id']];lo=Vector(report['bounds_min']);hi=Vector(report['bounds_max']);size=hi-lo
    scale=s['width']/max(size.x,size.y)
    root=bpy.data.objects.new(f"{i+1:02d} {s['title']}",None);root.instance_type='COLLECTION';root.instance_collection=col
    models.objects.link(root);root.location=xyz(s['position']);root.scale=(scale,)*3;root.rotation_euler.z=s['yaw'];root['stop']=i+1;root['source_sha256']=report['sha256']
    x,y,z=s['position'];pad=cube(s['id']+' terrace',[x,(y-1)/2-.08,z],[size.x*scale+3,y+1,size.y*scale+3],stone);pad.rotation_euler.z=s['yaw']
    top=cube(s['id']+' paving',[x,y-.14,z],[size.x*scale+2.8,.22,size.y*scale+2.8],paving);top.rotation_euler.z=s['yaw']
    # Camera stop positions are shared with the browser after its bounds check.
    route.append([x,y+1,z+s['width']*.65])
if (temp/'paths.json').exists():
    for i,points in enumerate(json.loads((temp/'paths.json').read_text())):path(f'Connecting path {i+1:02d} — route study',points,.65,paving)
else:
    path('Connecting path — preliminary',route,.65,paving)
path('River — profile placeholder',[[p[0],-.7,p[2]] for p in layout['river']],2.6,water)
foliage=material('Simple pine foliage',(.28,.39,.16));trunk=material('Pine trunks',(.24,.15,.08))
for i in range(28):
    x=(-47 if i<14 else 45)+(i%2)*3;z=32-(i%14)*7;h=4+i%3
    cube('Tree trunk',[x,-.5+h*.2,z],[.35,h*.4,.35],trunk)
    for j in range(3):
        mesh=bpy.data.meshes.new('Pine tier');bm=bmesh.new();bmesh.ops.create_cone(bm,cap_ends=True,cap_tris=False,segments=5,radius1=h*(.32-j*.055),radius2=0,depth=h*.55);bm.to_mesh(mesh);bm.free()
        o=bpy.data.objects.new('Backdrop pine',mesh);o.location=xyz([x,-.5+h*(.48+j*.22),z]);o.data.materials.append(foliage);terrain.objects.link(o)
world=bpy.data.worlds.new('Soft daytime sky — provisional');scene.world=world;world.use_nodes=True
nodes=world.node_tree.nodes;sky=nodes.new('ShaderNodeTexSky');sky.sky_type='NISHITA';sky.sun_elevation=math.radians(40);sky.sun_rotation=math.radians(135);world.node_tree.links.new(sky.outputs['Color'],nodes.get('Background').inputs['Color']);nodes.get('Background').inputs['Strength'].default_value=.25
ld=bpy.data.lights.new('Shared sun — provisional','SUN');ld.energy=2;ld.angle=.15;sun=bpy.data.objects.new(ld.name,ld);scene.collection.objects.link(sun);sun.rotation_euler=(.55,-.4,-.65)
camdata=bpy.data.cameras.new('Village review camera');cam=bpy.data.objects.new(camdata.name,camdata);scene.collection.objects.link(cam);scene.camera=cam;camdata.lens=42;camdata.clip_end=700
poses_path=temp/'camera-poses.json'
if poses_path.exists():
    poses=json.loads(poses_path.read_text());scene.render.fps=30
    for i,p in enumerate(poses):
        frame=1+i*330;cam.location=xyz(p['eye']);direction=Vector(xyz(p['target']))-cam.location;cam.rotation_euler=direction.to_track_quat('-Z','Y').to_euler()
        for f in (frame,frame+90):cam.keyframe_insert('location',frame=f);cam.keyframe_insert('rotation_euler',frame=f)
        scene.timeline_markers.new(f"{i+1:02d} {layout['stops'][i]['title']}",frame=frame)
    scene.frame_end=(len(poses)-1)*330+90
    # Match the browser's raised travel arc; direct interpolation between stop
    # cameras can otherwise pass through neighboring plots.
    for i in range(len(poses)-1):
        a,b=poses[i],poses[i+1];eye=[(a['eye'][j]+b['eye'][j])/2 for j in range(3)];eye[1]=max(a['eye'][1],b['eye'][1],a['bounds']['max'][1],b['bounds']['max'][1])+13
        target=[(a['target'][j]+b['target'][j])/2 for j in range(3)];cam.location=xyz(eye);cam.rotation_euler=(Vector(xyz(target))-cam.location).to_track_quat('-Z','Y').to_euler()
        cam.keyframe_insert('location',frame=1+i*330+210);cam.keyframe_insert('rotation_euler',frame=1+i*330+210)
cam.location=xyz((100,108,135));cam.rotation_euler=(Vector(xyz((0,6,-22)))-cam.location).to_track_quat('-Z','Y').to_euler()
overview_data=camdata.copy();overview=bpy.data.objects.new('Village overview — static',overview_data);scene.collection.objects.link(overview);overview.location=cam.location.copy();overview.rotation_euler=cam.rotation_euler.copy();scene.camera=overview
scene.render.engine='CYCLES';scene.cycles.samples=16;scene.cycles.use_denoising=False
scene.render.resolution_x=1600;scene.render.resolution_y=1100;scene.render.resolution_percentage=100
scene.view_settings.view_transform='AgX';scene['stage']='Layout blockout; final shared lighting not baked';scene['originals_unchanged']=True
print('PACKING_WORLD',flush=True)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(output),compress=True)
output.with_suffix('.sources.json').write_text(json.dumps(receipts,indent=2)+'\n')
print('WORLD_MASTER_SAVED',str(output),flush=True)
