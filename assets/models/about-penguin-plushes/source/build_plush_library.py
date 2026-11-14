"""Small, editable felt penguin companions. Run with Blender 4.3+.

blender -b --python assets/models/about-penguin-plushes/source/build_plush_library.py
The GLB contains four unit-height variants, each in its own mesh. The About
overlay exporter instances their geometry without changing existing scene data.
"""
from pathlib import Path
import bpy
import json
import math
import struct
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)

def linear(hex_color):
    values = [int(hex_color[i:i+2], 16)/255 for i in (1, 3, 5)]
    return tuple(v/12.92 if v <= .04045 else ((v+.055)/1.055)**2.4 for v in values)

def material(name, color):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*linear(color), 1)
    m.use_nodes = True
    p = m.node_tree.nodes.get('Principled BSDF')
    p.inputs['Base Color'].default_value = m.diffuse_color
    p.inputs['Roughness'].default_value = .95
    p.inputs['Sheen Weight'].default_value = .3
    return m

M = {k: material(k, c) for k,c in {
    'charcoal':'#38434B', 'navy':'#364D60', 'cream':'#F4EADA',
    'feet':'#C88C56', 'eyes':'#22282C', 'eye glint':'#FFF6E9',
    'blue':'#7CA8BA', 'sage':'#97A589', 'berry':'#B585A0', 'honey':'#C8A768',
}.items()}

def oval(name, pos, size, mat, segments=12, rings=8, rotation=0):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=segments, ring_count=rings, location=pos)
    o = bpy.context.object
    o.name = name
    o.scale = size
    o.rotation_euler.y = rotation
    o.data.materials.append(M[mat])
    for poly in o.data.polygons:
        poly.use_smooth = True
    return o

def scarf_tail(color):
    o = oval('Soft hanging scarf end', (.15,-.205,.43), (.065,.026,.19), color, 10, 6, -.18)
    # Three short raised stitches at the end read as fabric, without a texture.
    for x in [.122,.15,.178]:
        oval('Scarf hem stitch', (x,-.232,.30), (.004,.003,.022), 'cream', 6, 4)

variants = ['blue', 'sage', 'berry', 'honey']
all_geometry = []
for i, color in enumerate(variants):
    old = set(bpy.context.scene.objects)
    coat = 'navy' if color in ('blue','honey') else 'charcoal'
    oval('Squishy pear-shaped body', (0,0,.36), (.28,.235,.32), coat, 20, 12)
    oval('Round plush head', (0,-.012,.735), (.252,.225,.245), coat, 20, 12)
    oval('Cream belly applique', (0,-.207,.365), (.207,.048,.252), 'cream')
    for side in [-1,1]:
        oval('Cream cheek applique', (side*.09,-.20,.741), (.136,.048,.15), 'cream')
        oval('Embroidered button eye', (side*.101,-.247,.782), (.025,.012,.032), 'eyes', 10, 6)
        oval('Tiny eye highlight', (side*.101-.006,-.259,.792), (.006,.003,.008), 'eye glint', 6, 4)
        oval('Soft dropped flipper', (side*.273,.018,.36), (.083,.105,.202), coat, 10, 6, side*.29)
        oval('Flattened plush foot', (side*.139,-.129,.048), (.108,.153,.048), 'feet', 10, 6)
    oval('Rounded fabric beak', (0,-.255,.689), (.067,.065,.037), 'feet', 10, 6)
    # Flat stuffed tail and a soft neck band. The band is deliberately very matte.
    oval('Stubby stuffed tail', (0,.225,.19), (.10,.095,.075), coat, 10, 6)
    bpy.ops.mesh.primitive_torus_add(major_segments=16, minor_segments=6,
        location=(0,-.005,.56), major_radius=.191, minor_radius=.027)
    band = bpy.context.object
    band.name = 'Fabric scarf neck band'
    band.scale.y = .94
    band.data.materials.append(M[color])
    for f in band.data.polygons: f.use_smooth = True
    scarf_tail(color)
    members = [o for o in bpy.context.scene.objects if o not in old]
    root = bpy.data.objects.new(f'{color.title()} scarf penguin', None)
    bpy.context.collection.objects.link(root)
    root['variant'] = color
    root['material_finish'] = 'Matte felt, embroidered eyes, soft stuffed feet'
    positions, normals, colors, indices = [], [], [], []
    graph = bpy.context.evaluated_depsgraph_get()
    for o in members:
        data = bpy.data.meshes.new_from_object(o.evaluated_get(graph), depsgraph=graph)
        data.calc_loop_triangles()
        normal_matrix = o.matrix_world.to_3x3().inverted().transposed()
        rgb = o.data.materials[0].diffuse_color[:3]
        for tri in data.loop_triangles:
            for loop in tri.loops:
                p = o.matrix_world @ data.vertices[data.loops[loop].vertex_index].co
                n = (normal_matrix @ data.corner_normals[loop].vector).normalized()
                positions.extend((p.x,p.z,-p.y))
                normals.extend((n.x,n.z,-n.y))
                colors.extend(rgb)
                indices.append(len(indices))
        bpy.data.meshes.remove(data)
        o.parent = root
    all_geometry.append((color, positions, normals, colors, indices))
    root.location.x = i*.85

buf = bytearray(); views=[]; accessors=[]; meshes=[]; nodes=[]; materials=[]
def attr(values, width, fmt, kind):
    while len(buf)%4: buf.append(0)
    raw = struct.pack('<'+fmt*len(values), *values)
    views.append(dict(buffer=0, byteOffset=len(buf), byteLength=len(raw)))
    buf.extend(raw)
    a=dict(bufferView=len(views)-1,componentType=kind,count=len(values)//width,
           type={1:'SCALAR',3:'VEC3'}[width])
    if fmt=='f':
        a['min']=[min(values[k::width]) for k in range(width)]
        a['max']=[max(values[k::width]) for k in range(width)]
    accessors.append(a)
    return len(accessors)-1
for color,p,n,c,idx in all_geometry:
    attributes={k:attr(v,3,'f',5126) for k,v in [('POSITION',p),('NORMAL',n),('COLOR_0',c)]}
    materials.append(dict(name=f'{color} felt',pbrMetallicRoughness=dict(metallicFactor=0,roughnessFactor=.95)))
    meshes.append(dict(name=color,primitives=[dict(attributes=attributes,indices=attr(idx,1,'I',5125),material=len(materials)-1)]))
    nodes.append(dict(mesh=len(meshes)-1,name=f'{color} scarf penguin',translation=[(len(meshes)-1)*.85,0,0]))
doc=dict(asset=dict(version='2.0',generator='Noms editable penguin plush library'),scene=0,
    scenes=[dict(nodes=list(range(4)))],nodes=nodes,meshes=meshes,materials=materials,
    accessors=accessors,bufferViews=views,buffers=[dict(byteLength=len(buf))])
js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4)
buf+=b'\0'*((-len(buf))%4)
raw=struct.pack('<III',0x46546c67,2,28+len(js)+len(buf))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(buf),0x004e4942)+buf
(ROOT/'models/penguin-plush-variants.glb').write_bytes(raw)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'models/penguin-plush-variants.blend'))
print('Triangles per plush:', len(all_geometry[0][4])//3)
