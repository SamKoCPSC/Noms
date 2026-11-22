"""Render the real, composed About GLBs, including a close-up of each plush.

blender -b --threads 4 --python .../render_review.py -- MODEL OUTPUT_DIR
MODEL is a name from placements.json, or 'library-gallery'.
"""
from pathlib import Path
import bpy
import json
import math
import sys
import numpy as np
from mathutils import Vector

HERE=Path(__file__).resolve().parents[1]
REPO=HERE.parents[2]
args=sys.argv[sys.argv.index('--')+1:]
name=args[0];out=Path(args[1]);out.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
gallery=name=='library-gallery'
if gallery:
    source=HERE/'models/penguini-plush.glb';row=None
else:
    row=next(r for r in json.loads((HERE/'placements.json').read_text())['models'] if r['model']==name)
    source=REPO/'assets/about'/row['file']
bpy.ops.import_scene.gltf(filepath=str(source))
meshes=[o for o in bpy.context.scene.objects if o.type=='MESH']

# The first-generation rooms use runtime image slots, separate from the GLB.
# Restore their color maps here; keep the newly appended plush texture-free.
textures={
 'kitchen':{1:'kitchen-laptop.webp',2:'kitchen-cookbook.webp',3:'kitchen-surface-color.webp'},
 'office':{1:'office-editor-left.webp',2:'office-editor-right.webp',3:'office-rtx4090.webp',4:'office-intel-core.webp'},
 'bedroom':{1:'bedroom-posters.webp'},'garden':{1:'garden-pastry.webp'},
}
if not gallery:
    for material in bpy.data.materials:
        if not material.use_nodes:continue
        # Blender adds numeric suffixes to duplicate material names; imported
        # extras retain the exact per-primitive print slot.
        meta=material.get('assembly',{})
        slot=meta.get('texture',0)
        filename=textures.get(name,{}).get(slot)
        if filename:
            image=bpy.data.images.load(str(REPO/'assets/about'/filename),check_existing=True)
            node=material.node_tree.nodes.new('ShaderNodeTexImage');node.image=image
            material.node_tree.links.new(node.outputs['Color'],material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'])

vertices=[]
for o in meshes:
    a=np.empty(len(o.data.vertices)*3,dtype=np.float32);o.data.vertices.foreach_get('co',a)
    matrix=np.array(o.matrix_world);vertices.append(a.reshape(-1,3)@matrix[:3,:3].T+matrix[:3,3])
points=np.concatenate(vertices);lo,hi=points.min(0),points.max(0)
scene=bpy.context.scene
scene.render.engine='CYCLES';scene.cycles.samples=48;scene.cycles.use_denoising=False
scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG'
scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.72,.75,.78,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.65
scene.view_settings.view_transform='AgX'
scene.view_settings.look='AgX - Medium High Contrast'
def area(n,pos,energy,size):
    data=bpy.data.lights.new(n,'AREA');data.energy=energy;data.shape='DISK';data.size=size
    o=bpy.data.objects.new(n,data);scene.collection.objects.link(o);o.location=pos
    o.rotation_euler=(Vector((0,0,1))-o.location).to_track_quat('-Z','Y').to_euler()
area('Large softbox',(-4,-6,12),1500,8)
area('Window fill',(5,2,9),900,7)
mat=bpy.data.materials.new('Warm studio ground');mat.diffuse_color=(.60,.575,.53,1)
bpy.ops.mesh.primitive_plane_add(size=200,location=(0,0,float(lo[2])-.025))
bpy.context.object.data.materials.append(mat)
camdata=bpy.data.cameras.new('Review camera');cam=bpy.data.objects.new('Review camera',camdata)
scene.collection.objects.link(cam);scene.camera=cam;camdata.type='ORTHO';camdata.lens=48

def camera_at(target,offset):
    cam.location=Vector(target)+Vector(offset)
    cam.rotation_euler=(Vector(target)-cam.location).to_track_quat('-Z','Y').to_euler()
    bpy.context.view_layer.update()

scene.render.resolution_x=1080;scene.render.resolution_y=945
center=(lo+hi)/2
right_wall=name in ('library','office','brick-kitchen')
camera_at(center,(-12 if right_wall else 12,-16,12))
rotation=np.array(cam.rotation_euler.to_matrix())
projected=(points-center)@rotation
aspect=1080/945
camdata.ortho_scale=max(np.ptp(projected[:,0]),np.ptp(projected[:,1])*aspect)*1.08
if '--closeups-only' not in args:
    scene.render.filepath=str(out/f'{name}.png');bpy.ops.render.render(write_still=True)

if row and '--overview-only' not in args:
    scene.render.resolution_x=480;scene.render.resolution_y=480
    for i,p in enumerate(row['plushes']):
        x,y,z=p['position'];h=p['height'];angle=math.radians(p['yaw_degrees'])
        target=(x,-z,y+h*.43)
        # Look toward its face, with enough surrounding furniture to check contact.
        offset_angle=angle+(-.20 if right_wall else .20)
        camera_at(target,(2.5*math.sin(offset_angle),-2.5*math.cos(offset_angle),1.4))
        camdata.ortho_scale=h*2.0
        scene.render.filepath=str(out/f'{name}-plush-{i+1}.png');bpy.ops.render.render(write_still=True)
