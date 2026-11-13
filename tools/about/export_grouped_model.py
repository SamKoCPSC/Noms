"""Export the grouped Blender masters to the About renderer's grouped GLB contract.

blender -b MASTER --python tools/about/export_grouped_model.py -- OUTPUT_DIR MODEL
python tools/about/pack_kitchen.py OUTPUT_DIR MODEL

Preserves corporate office and brick kitchen geometry; simplifies library/supermarket/bakery/farmers-market derivatives.
Applies world transforms,
retains evaluated normals and linear material colors, and batches by assembly/opacity.
The master is never saved. Studio geometry and lights are excluded.
"""
import bpy
import hashlib
import json
import struct
import sys
from array import array
from pathlib import Path

model=sys.argv[sys.argv.index("--")+2] if len(sys.argv)>sys.argv.index("--")+2 else "corporate-office"
assert model in ("corporate-office", "library", "supermarket", "brick-kitchen", "bakery", "farmers-market")
source=Path(bpy.data.filepath)
out=Path(sys.argv[sys.argv.index('--')+1]);out.mkdir(parents=True,exist_ok=True)
bpy.context.view_layer.update()
graph=bpy.context.evaluated_depsgraph_get()
groups=sorted((o for o in bpy.context.scene.objects if o.type=='EMPTY' and 'assembly' in o),key=lambda o:o.name)
assert len(groups)=={"corporate-office":15,"library":13,"supermarket":17,"brick-kitchen":17,"bakery":24,"farmers-market":8}[model]

def yup(v):return (v[0],v[2],-v[1])

def material_info(material):
    shader=material.node_tree.nodes.get('Principled BSDF') if material and material.use_nodes else None
    rgb=tuple(shader.inputs['Base Color'].default_value[:3] if shader else material.diffuse_color[:3])
    transmission=float(shader.inputs['Transmission Weight'].default_value) if shader else 0.
    # Approximate the reading hall's partially transmitting art glass with alpha;
    # its solid blue window panels have zero transmission and remain opaque.
    opacity=max(.20,1.-transmission) if model=='library' else (.20 if transmission>.5 else 1.)
    return rgb,opacity

reduction_scene=bpy.data.scenes.new('Web geometry workspace')
batches={};chunks={};audit=[]
for group in groups:
    label=group['assembly']
    if model=='brick-kitchen':
        index=int(label[:2])
        group['start_frame']=1. if index<=3 else (30.+(index-4)*6. if index<=9 else 80.+(index-10)*5.)
        group['end_frame']=group['start_frame']+35.
    if model=='bakery':
        index=int(label[:2]);details=[10,11,12,15,16,17]
        group['start_frame']=1. if index<=3 else (80.+details.index(index)*6. if index in details else 30.+(index%8)*5.)
        group['end_frame']=group['start_frame']+35.
    if model=='farmers-market':
        index=int(label[:2])
        group['start_frame']=1. if index==1 else (30.+(index-2)*8. if index<=6 else 78.+(index-7)*12.)
        group['end_frame']=group['start_frame']+35.
    print('Export', label, flush=True)
    if label.startswith('01 '):label+=' / floor'
    elif model!='farmers-market' and label.startswith('02 '):label+=' / back wall'
    elif model!='farmers-market' and label.startswith('03 '):label+=' / side wall'
    for obj in sorted(group.children_recursive,key=lambda o:o.name):
        if obj.type not in ('MESH','CURVE','FONT'):continue
        data=bpy.data.meshes.new_from_object(obj.evaluated_get(graph),depsgraph=graph)
        data.calc_loop_triangles()
        source_triangles=len(data.loop_triangles)
        # The review collections contain dense bevels and lettering. Remove only
        # coplanar edges first, then use a bounded derivative for the larger scenes.
        if model in ('library', 'supermarket', 'bakery', 'farmers-market'):
            import bmesh
            bm=bmesh.new();bm.from_mesh(data)
            bmesh.ops.dissolve_limit(bm,angle_limit=.001,verts=list(bm.verts),edges=list(bm.edges),delimit={'MATERIAL'})
            bm.to_mesh(data);bm.free()
            data.calc_loop_triangles()
            if len(data.loop_triangles)>100:
                temp=bpy.data.objects.new('Web geometry reduction',data)
                reduction_scene.collection.objects.link(temp)
                modifier=temp.modifiers.new('Web silhouette budget','DECIMATE')
                modifier.ratio={'supermarket':.53,'library':.30,'bakery':.45,'farmers-market':.45}[model]
                with bpy.context.temp_override(scene=reduction_scene,view_layer=reduction_scene.view_layers[0]):
                    reduction_graph=bpy.context.evaluated_depsgraph_get()
                    reduced=bpy.data.meshes.new_from_object(temp.evaluated_get(reduction_graph),depsgraph=reduction_graph)
                bpy.data.objects.remove(temp,do_unlink=True);bpy.data.meshes.remove(data)
                data=reduced;data.calc_loop_triangles()
        positions=[yup(obj.matrix_world@v.co) for v in data.vertices]
        normals_matrix=obj.matrix_world.to_3x3().inverted().transposed()
        normals=[yup((normals_matrix@n.vector).normalized()) for n in data.corner_normals]
        materials=[material_info(m) for m in data.materials]
        audit.append(dict(name=obj.name,source_triangles=source_triangles,web_triangles=len(data.loop_triangles),preserved=source_triangles==len(data.loop_triangles)))
        for tri in data.loop_triangles:
            rgb,opacity=materials[tri.material_index]
            group_key=(label,opacity)
            chunk=chunks.get(group_key,0)
            key=(*group_key,chunk)
            if key in batches and len(batches[key]['indices'])>=240000:
                chunk+=1;chunks[group_key]=chunk;key=(*group_key,chunk)
            if key not in batches:
                batches[key]=dict(vertices=[],indices=[],lookup={},meta=dict(name=label,pivot=list(yup(group['pivot'])),offset=list(yup(group['incoming_offset'])),start=float(group['start_frame']) if model!='library' or label[:2] in ('01','02','03') else 32.+(int(label[:2])-4)*9.,end=float(group['end_frame']) if model!='library' or label[:2] in ('01','02','03') else 68.+(int(label[:2])-4)*9.,texture=0,opacity=opacity))
            batch=batches[key]
            for loop in tri.loops:
                vertex=(*positions[data.loops[loop].vertex_index],*normals[loop],*rgb)
                index=batch['lookup'].get(vertex)
                if index is None:
                    index=len(batch['vertices']);batch['lookup'][vertex]=index;batch['vertices'].append(vertex)
                batch['indices'].append(index)
        bpy.data.meshes.remove(data)

print('Web triangles',sum(len(b['indices'])//3 for b in batches.values()),flush=True)
(out/f'{model}-geometry-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
binary=bytearray();views=[];accessors=[];materials=[];primitives=[]
def attribute(values,width,fmt,target):
    while len(binary)%4:binary.append(0)
    packed=array(fmt,values)
    if sys.byteorder!='little':packed.byteswap()
    raw=packed.tobytes()
    view=len(views);views.append(dict(buffer=0,byteOffset=len(binary),byteLength=len(raw),target=target));binary.extend(raw)
    index=len(accessors)
    accessors.append(dict(bufferView=view,componentType=5126 if fmt=='f' else 5125,count=len(values)//width,type={1:'SCALAR',3:'VEC3'}[width]))
    return index
for batch in batches.values():
    vertices=batch['vertices'];assert len(vertices)<=300000
    attrs={name:attribute([x for v in vertices for x in v[start:start+3]],3,'f',34962) for name,start in [('POSITION',0),('NORMAL',3),('COLOR_0',6)]}
    meta=batch['meta'];material=dict(name=meta['name'],extras=dict(assembly=meta),pbrMetallicRoughness=dict(metallicFactor=0,roughnessFactor=.8),doubleSided=True)
    if meta['opacity']<1:
        material['alphaMode']='BLEND';material['pbrMetallicRoughness']['baseColorFactor']=[1,1,1,meta['opacity']]
    primitives.append(dict(attributes=attrs,indices=attribute(batch['indices'],1,'I',34963),mode=4,material=len(materials)))
    materials.append(material)
assert len(primitives)<=64
triangles=sum(len(b['indices'])//3 for b in batches.values());assert triangles<400000

doc=dict(asset=dict(version='2.0',generator=f'Noms {model} web exporter'),scene=0,scenes=[dict(nodes=[0])],nodes=[dict(mesh=0,name=model)],meshes=[dict(primitives=primitives)],materials=materials,accessors=accessors,bufferViews=views,buffers=[dict(byteLength=len(binary))])
js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4);binary+=b'\0'*((-len(binary))%4)
raw=struct.pack('<III',0x46546c67,2,28+len(js)+len(binary))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(binary),0x004e4942)+binary
(out/f'sams-{model}.glb').write_bytes(raw)
budget=dict(source=str(source.relative_to(source.parents[3])),source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),triangles=triangles,assemblies=len(groups),materials=len(materials),glb_bytes=len(raw),textures=0,geometry='All evaluated master triangles retained; no decimation' if model in ('corporate-office','brick-kitchen') else 'Coplanar dissolution and bounded mesh simplification; masters unchanged',shading='Linear material colors and evaluated normals; runtime matte lighting and dynamic shadows')
(out/f'{model}-budget.json').write_text(json.dumps(budget,indent=2)+'\n')
(out/f'{model}-geometry-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
print(json.dumps(budget,indent=2))
