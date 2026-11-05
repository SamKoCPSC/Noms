"""Bake the approved 6.6 s SVG jump-and-wave into an editable Blender rig.

Run: blender -b -t 4 -P animate_penguini.py
The original v2 model is retained as base.blend. Output uses ordinary node
animation and skeletal skinning, with no runtime Python handlers.
"""
import bpy
import math
import re
import json
from pathlib import Path
from mathutils import Vector, Matrix

SOURCE=Path(__file__).resolve().parent
OUT=SOURCE.parent
bpy.ops.wm.open_mainfile(filepath=str(SOURCE/'base.blend'))
scene=bpy.context.scene
scene.name='Excited - Jump and wave'
model=bpy.data.collections['PENGUINI • editable model']
root=bpy.data.objects['Penguini']
head=bpy.data.objects['Head • rotate here']
body=bpy.data.objects['Body • continuous shoulders and flippers']
reference=json.loads((SOURCE/'excited-reference.json').read_text())
tracks={t['selector']:t for t in reference['tracks']}
FPS=30;DURATION=reference['durationMs']/1000;END=round(FPS*DURATION)
scene.render.fps=FPS;scene.frame_start=0;scene.frame_end=END
scene.frame_set(0)

def smooth(t):
    t=max(0,min(1,t));return t*t*(3-2*t)

def bezier(t,x1,y1,x2,y2):
    lo,hi=0.,1.
    for _ in range(30):
        u=(lo+hi)/2
        x=3*(1-u)**2*u*x1+3*(1-u)*u*u*x2+u**3
        if x<t:lo=u
        else:hi=u
    u=(lo+hi)/2
    return 3*(1-u)**2*u*y1+3*(1-u)*u*u*y2+u**3

def easing(t,kind):
    if kind.startswith('steps'):return 1. if t>=1 else 0.
    if kind=='linear':return t
    presets={'ease':(.25,.1,.25,1),'ease-in-out':(.42,0,.58,1),'ease-in':(.42,0,1,1),'ease-out':(0,0,.58,1)}
    controls=presets.get(kind)
    if controls is None:controls=tuple(float(x) for x in re.findall(r'[-+]?(?:\d*\.)?\d+',kind))
    return bezier(t,*controls)

def parse(transform):
    data={'translateY':0.,'rotate':0.,'scaleX':1.,'scaleY':1.}
    for kind,args in re.findall(r'(translateY|rotate|scaleX|scaleY|scale)\(([^)]+)\)',transform):
        values=[float(x) for x in re.findall(r'[-+]?(?:\d*\.)?\d+',args)]
        if kind=='scale':data['scaleX']=values[0];data['scaleY']=values[-1]
        else:data[kind]=values[0]
    return data

def sample(selector,time):
    track=tracks[selector];keys=track['keyframes'];t=time/DURATION
    left=keys[0];right=keys[-1]
    for a,b in zip(keys,keys[1:]):
        if t<=b['offset']+1e-10:left,right=a,b;break
    span=right['offset']-left['offset']
    alpha=easing(max(0,min(1,(t-left['offset'])/span)),left.get('easing',track['easing'])) if span else 0
    if 'opacity' in left:return float(left['opacity'])*(1-alpha)+float(right['opacity'])*alpha
    a,b=parse(left['transform']),parse(right['transform'])
    return {key:a[key]*(1-alpha)+b[key]*alpha for key in a}

def link(obj):
    for collection in list(obj.users_collection):collection.objects.unlink(obj)
    model.objects.link(obj)

def empty(name,parent,world):
    obj=bpy.data.objects.new(name,None);model.objects.link(obj)
    obj.empty_display_type='PLAIN_AXES';obj.empty_display_size=.10
    obj.parent=parent;bpy.context.view_layer.update()
    obj.matrix_world=Matrix.Translation(Vector(world))
    bpy.context.view_layer.update()
    return obj

def reparent(obj,parent):
    bpy.context.view_layer.update();matrix=obj.matrix_world.copy()
    obj.parent=parent;obj.matrix_world=matrix;bpy.context.view_layer.update()

def sphere(name,location,scale,material,parent):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=40,ring_count=24,location=location)
    obj=bpy.context.object;obj.name=name;link(obj)
    for vertex in obj.data.vertices:
        vertex.co.x*=scale[0];vertex.co.y*=scale[1];vertex.co.z*=scale[2]
    obj.data.materials.append(material)
    for polygon in obj.data.polygons:polygon.use_smooth=True
    reparent(obj,parent)
    return obj

def curve_mesh(name,points,radius,material,parent):
    data=bpy.data.curves.new(name,'CURVE');data.dimensions='3D'
    data.bevel_depth=radius;data.bevel_resolution=3;data.resolution_u=8;data.use_fill_caps=True
    spline=data.splines.new('BEZIER');spline.bezier_points.add(len(points)-1)
    for point,co in zip(spline.bezier_points,points):
        point.co=co;point.handle_left_type=point.handle_right_type='AUTO'
    obj=bpy.data.objects.new(name,data);model.objects.link(obj);obj.data.materials.append(material)
    bpy.ops.object.select_all(action='DESELECT');obj.select_set(True);bpy.context.view_layer.objects.active=obj
    bpy.ops.object.convert(target='MESH')
    reparent(obj,parent)
    return obj

# Bake trims into meshes before animation so the GLB and .blend share the
# same hierarchy and every animated surface can use ordinary glTF channels.
for obj in list(model.objects):
    if obj.type=='CURVE':
        bpy.ops.object.select_all(action='DESELECT');obj.select_set(True);bpy.context.view_layer.objects.active=obj
        bpy.ops.object.convert(target='MESH')

# Preserve the welded torso/wing surface. Skin it across broad shoulder zones;
# do not separate the wings into rigid floating pieces.
bpy.ops.object.select_all(action='DESELECT')
data=bpy.data.armatures.new('Penguini • flipper skeleton')
rig=bpy.data.objects.new('Penguini rig',data);model.objects.link(rig);rig.parent=root
rig.show_in_front=True;rig.select_set(True);bpy.context.view_layer.objects.active=rig
bpy.ops.object.mode_set(mode='EDIT')
torso=data.edit_bones.new('Torso');torso.head=(0,0,.45);torso.tail=(0,0,1.65)
for name,origin,tip in [
    ('Flipper.L',(-.66,.025,1.62),(-1.49,-.04,1.90)),
    ('Flipper.R',(.66,.025,1.62),(1.47,-.045,1.18)),
]:
    bone=data.edit_bones.new(name);bone.head=origin;bone.tail=tip;bone.parent=torso;bone.use_connect=False
    shoulder=data.edit_bones.new(name.replace('Flipper','Shoulder'))
    shoulder.head=origin;shoulder.tail=Vector(origin)+(Vector(tip)-Vector(origin))*.35
    shoulder.parent=torso;shoulder.use_connect=False
bpy.ops.object.mode_set(mode='OBJECT')
body.vertex_groups.clear()
groups={name:body.vertex_groups.new(name=name) for name in ('Torso','Flipper.L','Flipper.R','Shoulder.L','Shoulder.R')}
for vertex in body.data.vertices:
    x,y,z=vertex.co;side='R' if x>=0 else 'L';a=abs(x)
    c=(z-1.12)/.91
    if abs(c)>=.9999:outside=1 if a>.4 else 0
    else:
        s=math.sqrt(1-c*c);rx=.86*s*(1-.18*c);ry=.55*s*(1-.05*c)
        outside=max(0,math.sqrt((x/rx)**2+(y/ry)**2)-1.008)
    # The original torso surface stays stationary. Only geometry extending
    # beyond it rolls through a half-angle shoulder into the waving flipper.
    shoulder=smooth(outside/.12)
    wing=smooth((outside-.10)/.20)
    groups['Torso'].add([vertex.index],1-shoulder,'REPLACE')
    if shoulder*(1-wing)>0:groups['Shoulder.'+side].add([vertex.index],shoulder*(1-wing),'REPLACE')
    if shoulder*wing>0:groups['Flipper.'+side].add([vertex.index],shoulder*wing,'REPLACE')
modifier=body.modifiers.new('Connected flipper deformation','ARMATURE');modifier.object=rig
modifier.use_deform_preserve_volume=False

for bone in rig.pose.bones:bone.rotation_mode='QUATERNION'

# Facial controls are attached to the head, including their highlights.
eyes=[]
eye_material=bpy.data.materials['Eyes • espresso gloss']
def face_y(x,z):return -.015-.655*math.sqrt(max(.01,1-(x/.91)**2-((z-2.35)/.865)**2))
for side in ('L','R'):
    eye=bpy.data.objects['Eye.'+side]
    center=eye.matrix_world.translation.copy()
    opened=empty('Expression • open eye '+side,head,center)
    for name in ('Eye.'+side,'Eye highlight.'+side,'Eye glimmer.'+side):reparent(bpy.data.objects[name],opened)
    happy_center=(center.x,face_y(center.x,2.55)-.025,2.55)
    happy=empty('Expression • happy eye '+side,head,happy_center)
    points=[]
    for i in range(9):
        u=i/8;x=center.x+(u-.5)*.20;z=2.525+.070*math.sin(math.pi*u)
        points.append((x,face_y(x,z)-.023,z))
    curve_mesh('Happy eye arc.'+side,points,.019,eye_material,happy)
    eyes.append((opened,happy,happy.location.copy()))

jaw=empty('Expression • lower beak hinge',head,(0,-.57,2.27))
reparent(bpy.data.objects['Beak • lower bill'],jaw)
upper_bill=bpy.data.objects['Beak • upper bill']
inside_mat=bpy.data.materials['Beak smile seam'].copy();inside_mat.name='Mouth • warm dark interior'
node=inside_mat.node_tree.nodes.get('Principled BSDF')
node.inputs['Base Color'].default_value=(.075,.022,.008,1);inside_mat.diffuse_color=(.075,.022,.008,1)
mouth=empty('Expression • open smile',head,(0,-.74,2.255))
inside=sphere('Mouth • rounded smile cavity',(0,-.705,2.17),(.135,.07,.095),inside_mat,mouth)
for v in inside.data.vertices:
    zn=v.co.z/.095
    v.co.x*=.78+.22*zn
mouth_rest=mouth.location.copy()

# A small supporting motion in the other flipper keeps the 3D pose balanced.
left_keys=[(0,0),(.3,0),(.594,-4),(1.026,17),(1.404,3),(1.566,-3),(1.944,14),(2.268,3),(2.646,0),(3.186,5),(3.618,2),(4.05,5),(4.644,3),(6,0),(6.6,0)]
def supporting(time):
    for (ta,a),(tb,b) in zip(left_keys,left_keys[1:]):
        if time<=tb:
            t=smooth((time-ta)/(tb-ta));return a*(1-t)+b*t
    return 0

head_rest=head.location.copy()
records=[]
def rotate_bone(name,world_rotation):
    bone=rig.pose.bones[name]
    rest=bone.bone.matrix_local.to_3x3()
    bone.rotation_quaternion=(rest.inverted()@world_rotation@rest).to_quaternion()

for frame in range(END+1):
    time=frame/FPS;scene.frame_set(frame)
    motion=sample('.penguin',time)
    # Crouching happens through scale about the soles. Clamp the tiny 2D
    # downward offsets so the feet never sink through the floor.
    root.location.z=max(0,-motion['translateY']*.00375)
    sx,sz=motion['scaleX'],motion['scaleY']
    root.scale=(sx,1/math.sqrt(sx*sz),sz)
    root.keyframe_insert('location',frame=frame);root.keyframe_insert('scale',frame=frame)
    h=sample('.head',time)
    head.location=head_rest+Vector((0,0,-h['translateY']*.00375))
    head.rotation_euler=(0,math.radians(h['rotate']),0)
    head.keyframe_insert('location',frame=frame);head.keyframe_insert('rotation_euler',frame=frame)
    angle=(sample('.flipper',time)['rotate']-88)*.93
    lift=min(1,abs(angle)/93)
    waving=Matrix.Rotation(math.radians(17*lift),3,'X')@Matrix.Rotation(math.radians(angle),3,'Y')
    rotate_bone('Flipper.R',waving)
    rotate_bone('Flipper.L',Matrix.Rotation(math.radians(supporting(time)),3,'Y'))
    rotate_bone('Shoulder.R',Matrix.Rotation(math.radians(17*lift*.5),3,'X')@Matrix.Rotation(math.radians(angle*.5),3,'Y'))
    rotate_bone('Shoulder.L',Matrix.Rotation(math.radians(supporting(time)*.5),3,'Y'))
    for bone in rig.pose.bones:bone.keyframe_insert('rotation_quaternion',frame=frame)
    happiness=sample('.eyes-happy',time)
    blink=sample('.blink',time)['scaleY']
    for opened,happy,position in eyes:
        factor=max(.0001,1-happiness)
        opened.scale=(factor,factor,max(.0001,blink*factor))
        opened.keyframe_insert('scale',frame=frame)
        happy.scale=(max(.0001,happiness),1,max(.0001,happiness))
        happy.location=position+Vector((0,.06*(1-happiness),0))
        happy.keyframe_insert('scale',frame=frame);happy.keyframe_insert('location',frame=frame)
    smile=max(0,min(1,(sample('.lower-beak',time)['scaleY']-.45)/(.73)))
    jaw.rotation_euler=(math.radians(50*smile),0,0)
    jaw.keyframe_insert('rotation_euler',frame=frame)
    upper_bill.scale=(1,1,1-.15*smile)
    upper_bill.keyframe_insert('scale',frame=frame)
    mouth.scale=(max(.0001,smile**.5),1,max(.0001,smile))
    mouth.location=mouth_rest+Vector((0,.20*(1-smooth(smile*4)),0))
    mouth.keyframe_insert('scale',frame=frame);mouth.keyframe_insert('location',frame=frame)
    records.append({'frame':frame,'time':time,'jump_height':root.location.z,'wave_degrees':angle,'happy_eyes':happiness,'smile':smile})

# The CSS easing is already sampled above. Linear baked keys avoid Blender's
# automatic Bézier overshoot on feet, eyes and the final neutral pose.
for obj in model.objects:
    if obj.animation_data and obj.animation_data.action:
        action=obj.animation_data.action;action.name='Excited | '+obj.name
        for fcurve in action.fcurves:
            for key in fcurve.keyframe_points:key.interpolation='LINEAR'

for name,seconds in [('Ready',0),('Crouch',.594),('First hop',1.026),('Land',1.404),('Second hop',1.944),('Wave',3.15),('Recover',4.8),('Rest',6)]:
    scene.timeline_markers.new(name,frame=round(seconds*FPS))
scene.frame_set(0)
root['animation']='Excited • Jump and wave | 6.6 s | 30 fps | based on Noms canonical SVG excited clip'
root['status']='Experimental animated 3D mascot: connected flipper skinning, two hops, wave, head tilts, happy eyes and opening beak.'
notes=bpy.data.texts.get('READ ME • Penguini')
if notes:
    notes.clear();notes.write('PENGUINI — EXCITED / JUMP AND WAVE\n\nPlay frames 0–198 at 30 fps.\nThe 6.6-second motion follows the canonical 2D excited clip.\nThe torso and both flippers remain one skinned mesh.\nUse Penguini rig for the flippers; named Expression controls for the face.\nHead and whole-character transforms are keyed separately.\nAll motion is baked; no runtime Python or external textures are required.\n\nbase.blend is the static revision 02 starting point.\nanimate_penguini.py rebuilds this animation from excited-reference.json.\nThe .glb carries the same skinning and node animation in one clip.\n')

# Framing includes the apex of the jump, not just the standing pose.
for obj in bpy.data.objects:
    if obj.type=='CAMERA':
        target=Vector((0,0,2.40))
        obj.rotation_euler=(target-obj.location).to_track_quat('-Z','Y').to_euler()
        obj.data.ortho_scale=5.8
scene.camera=bpy.data.objects['Camera • three-quarter']
scene.render.engine='BLENDER_EEVEE_NEXT'
scene.eevee.taa_render_samples=32
bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig
bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE/'penguini-excited.blend'),compress=True)

bpy.ops.object.select_all(action='DESELECT')
for obj in model.objects:obj.select_set(True)
options=dict(filepath=str(OUT/'penguini-excited.glb'),export_format='GLB',use_selection=True,export_apply=True,export_animations=True,export_force_sampling=True,export_frame_range=True,export_frame_step=1,export_cameras=False,export_lights=False,export_animation_mode='SCENE',export_anim_scene_split_object=False,export_nla_strips_merged_animation_name='Excited - Jump and wave')
bpy.ops.export_scene.gltf(**options)
print('EXCITED_BUILD_COMPLETE',str(OUT),flush=True)
