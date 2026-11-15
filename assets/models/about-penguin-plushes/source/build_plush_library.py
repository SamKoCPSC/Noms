"""Create a small static derivative of the approved Chef Penguini mesh.

blender -b --threads 2 --python assets/models/about-penguin-plushes/source/build_plush_library.py
The original character and animation are never modified. Every accessory part
comes from the approved base.blend, with evaluated modifiers and local mesh
simplification for the About tour's bounded loader.
"""
from pathlib import Path
import bpy
import hashlib
import json
import struct
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[2]
SOURCE = REPO/'assets/brand/penguini/3d/source/base.blend'
bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
scene = bpy.context.scene
scene.frame_set(0)
original = bpy.data.objects['Penguini']
source_objects = sorted((o for o in original.children_recursive if o.type in ('MESH','CURVE') and not o.hide_render), key=lambda o:o.name)
# The source's sub-pixel piping is omitted in this small static derivative.
# Thin patches are evaluated as single surfaces so decimation cannot fold their
# front and back faces through the underlying body.
source_objects = [o for o in source_objects if not o.name.startswith('Eye glimmer')
                  and (o.type != 'CURVE' or o.name.startswith('Apron bow') or 'smile seam' in o.name)]
for o in source_objects:
    for mod in list(o.modifiers):
        if mod.type in ('SOLIDIFY', 'BEVEL'):o.modifiers.remove(mod)
bpy.context.view_layer.update()
graph = bpy.context.evaluated_depsgraph_get()
created = []
audit = []
feet = []

def target_for(name):
    limits = [('Body •',560), ('Head •',460), ('sculpted cloud crown',360),
              ('heart-shaped',260), ('Foot.',140), ('Belly •',120),
              ('curved skirt',120), ('front pocket',64), ('waist tie',64),
              ('soft band',120), ('Eye.',40), ('upper bill',60), ('lower bill',40),
              ('Eye highlight',12), ('Eye glimmer',8), ('Apron knot',20),
              ('Apron strap',40), ('Apron hanging',30)]
    return next((count for key,count in limits if key in name),18)

for source in source_objects:
    mesh = bpy.data.meshes.new_from_object(source.evaluated_get(graph), depsgraph=graph)
    mesh.transform(source.matrix_world)
    mesh.calc_loop_triangles()
    before = len(mesh.loop_triangles)
    obj = bpy.data.objects.new(source.name + ' • tour', mesh)
    scene.collection.objects.link(obj)
    # Preserve the real source surface colors, including the cream heart mask.
    for material in mesh.materials:
        if material and material.use_nodes:
            bsdf=next((n for n in material.node_tree.nodes if n.type=='BSDF_PRINCIPLED'),None)
            if bsdf:material.diffuse_color=bsdf.inputs['Base Color'].default_value
    bpy.ops.object.select_all(action='DESELECT')
    obj.select_set(True);bpy.context.view_layer.objects.active=obj
    target=target_for(source.name)
    if before>target:
        mod=obj.modifiers.new('Tour silhouette budget','DECIMATE')
        mod.ratio=target/before;mod.use_collapse_triangulate=True
        bpy.ops.object.modifier_apply(modifier=mod.name)
    # A tiny outward offset keeps simplified applique surfaces on the coat.
    lift = .025 if source.name.startswith(('Face •','Belly •')) else .02 if 'front pocket' in source.name else 0
    if lift:
        for v in obj.data.vertices:v.co.y -= lift
    obj.data.calc_loop_triangles()
    for face in obj.data.polygons:face.use_smooth=True
    created.append(obj)
    audit.append(dict(name=source.name,source_triangles=before,triangles=len(obj.data.loop_triangles)))
    if source.name.startswith('Foot.'):
        coords=[v.co.copy() for v in obj.data.vertices]
        floor=min(v.z for v in coords)
        sole=[v for v in coords if v.z<floor+.025]
        feet.append(sum(sole,Vector())/len(sole))

points=[v.co for o in created for v in o.data.vertices]
lo=Vector(tuple(min(v[i]for v in points)for i in range(3)))
hi=Vector(tuple(max(v[i]for v in points)for i in range(3)))
height=hi.z-lo.z
center=Vector(((lo.x+hi.x)/2,(lo.y+hi.y)/2,lo.z))
# Keep only editable optimized source parts; remove the studio and dense master.
for o in list(bpy.data.objects):
    if o not in created:bpy.data.objects.remove(o,do_unlink=True)
root=bpy.data.objects.new('Chef Penguini • approved model derivative',None)
scene.collection.objects.link(root);root['variant']='penguini'
root['source_model']='assets/brand/penguini/3d/source/base.blend'
root['unit_height']=1.0
positions=[];normals=[];colors=[];indices=[]
for o in created:
    for v in o.data.vertices:v.co=(v.co-center)/height
    o.data.update();o.data.calc_loop_triangles();o.parent=root
    for tri in o.data.loop_triangles:
        color=o.data.materials[tri.material_index].diffuse_color[:3]
        for loop in tri.loops:
            p=o.data.vertices[o.data.loops[loop].vertex_index].co
            n=o.data.corner_normals[loop].vector
            positions.extend((p.x,p.z,-p.y));normals.extend((n.x,n.z,-n.y))
            colors.extend(color);indices.append(len(indices))
triangles=len(indices)//3
assert triangles<=2907, f'Garden budget: {triangles}'
metadata=dict(source=str(SOURCE.relative_to(REPO)),source_sha256=hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
    variant='penguini',unit_height=1,triangles=triangles,
    foot_contacts=[[float((v.x-center.x)/height),float((v.z-center.z)/height),float(-(v.y-center.y)/height)]for v in feet],
    parts=audit)
(ROOT/'library-metadata.json').write_text(json.dumps(metadata,indent=2)+'\n')
buf=bytearray();views=[];accessors=[]
def attr(values,width,fmt,kind):
    while len(buf)%4:buf.append(0)
    raw=struct.pack('<'+fmt*len(values),*values)
    views.append(dict(buffer=0,byteOffset=len(buf),byteLength=len(raw)));buf.extend(raw)
    a=dict(bufferView=len(views)-1,componentType=kind,count=len(values)//width,type={1:'SCALAR',3:'VEC3'}[width])
    if fmt=='f':
        a['min']=[min(values[k::width])for k in range(width)];a['max']=[max(values[k::width])for k in range(width)]
    accessors.append(a);return len(accessors)-1
attrs={k:attr(v,3,'f',5126)for k,v in [('POSITION',positions),('NORMAL',normals),('COLOR_0',colors)]}
primitive=dict(attributes=attrs,indices=attr(indices,1,'I',5125),material=0)
doc=dict(asset=dict(version='2.0',generator='Noms approved Penguini derivative'),scene=0,scenes=[dict(nodes=[0])],
    nodes=[dict(mesh=0,name='Chef Penguini')],meshes=[dict(name='penguini',primitives=[primitive])],
    materials=[dict(name='Penguini original colors • matte',pbrMetallicRoughness=dict(metallicFactor=0,roughnessFactor=.95))],
    buffers=[dict(byteLength=len(buf))],bufferViews=views,accessors=accessors,extras=dict(source=metadata['source'],source_sha256=metadata['source_sha256']))
js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4)
raw=struct.pack('<III',0x46546c67,2,28+len(js)+len(buf))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(buf),0x004e4942)+buf
(ROOT/'models/penguini-plush.glb').write_bytes(raw)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'models/penguini-plush.blend'))
print('PENGUINI',json.dumps(metadata))
