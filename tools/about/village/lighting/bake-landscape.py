"""Bake narrow landscape surfaces to vertex colors, avoiding tiny UV islands.
Blender --python bake-landscape.py -- STUDY_DIRECTORY [SAMPLES]
Ground shadows and the kitchen retain full-resolution texture atlases.
"""
import bpy, json, math, struct, sys
from pathlib import Path
import numpy as np

args=sys.argv[sys.argv.index('--')+1:];out=Path(args[0])
bpy.ops.wm.open_mainfile(filepath=str(out/'kitchen-district.blend'),load_ui=False,use_scripts=False)
scene=bpy.context.scene;scene.cycles.samples=int(args[1]) if len(args)>1 else 64;scene.cycles.use_denoising=False
members=[o for o in scene.objects if o.get('bake_group')=='landscape']
bpy.ops.object.select_all(action='DESELECT')
for obj in members:obj.select_set(True)
bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();obj=bpy.context.object;obj.name='Baked district landscape vertex lighting'
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
attribute=obj.data.color_attributes.new(name='WorldLighting',type='FLOAT_COLOR',domain='CORNER');obj.data.color_attributes.active_color=attribute
scene.render.bake.target='VERTEX_COLORS'
for key,value in dict(use_pass_diffuse=True,use_pass_glossy=False,use_pass_transmission=False,use_pass_emit=True,use_pass_direct=True,use_pass_indirect=True,use_pass_color=True).items():setattr(scene.render.bake,key,value)
bpy.ops.object.bake(type='COMBINED')
# Use the exact same Blender view transform as the image atlases. The resulting
# display RGB is decoded to linear vertex color for the browser's sRGB output.
count=len(attribute.data);width=1024;height=math.ceil(count/width)
pixels=np.ones((width*height,4),dtype=np.float32);attribute.data.foreach_get('color',pixels[:count].reshape(-1))
palette=bpy.data.images.new('World vertex display palette',width=width,height=height,alpha=False,float_buffer=True);palette.colorspace_settings.name='Linear Rec.709';palette.pixels.foreach_set(pixels.reshape(-1))
scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_depth='16';palette.save_render(str(out/'landscape-vertex-palette.png'),scene=scene)
display=bpy.data.images.load(str(out/'landscape-vertex-palette.png'));display.colorspace_settings.name='Non-Color'
display.pixels.foreach_get(pixels.reshape(-1));colors=pixels[:count].copy();rgb=colors[:,:3]
colors[:,:3]=np.where(rgb<=.04045,rgb/12.92,((rgb+.055)/1.055)**2.4);colors[:,3]=1
attribute.data.foreach_set('color',colors.reshape(-1))
mat=bpy.data.materials.new('Baked world vertex lighting');mat.use_nodes=True
node=mat.node_tree.nodes.new('ShaderNodeVertexColor');node.layer_name='WorldLighting'
shader=mat.node_tree.nodes.get('Principled BSDF');shader.inputs['Roughness'].default_value=1;mat.node_tree.links.new(node.outputs['Color'],shader.inputs['Base Color'])
obj.data.materials.clear();obj.data.materials.append(mat)
for polygon in obj.data.polygons:polygon.material_index=0
for uv in list(obj.data.uv_layers):obj.data.uv_layers.remove(uv)
path=out/'landscape-vertex.glb'
bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True)
data=path.read_bytes();size=struct.unpack_from('<I',data,12)[0];doc=json.loads(data[20:20+size])
assert len(doc['materials'])==1 and all('COLOR_0' in p['attributes'] for mesh in doc['meshes'] for p in mesh['primitives'])
doc['materials'][0].setdefault('extensions',{})['KHR_materials_unlit']={};doc.setdefault('extensionsUsed',[]).append('KHR_materials_unlit')
encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4)
chunks=struct.pack('<I4s',len(encoded),b'JSON')+encoded+data[20+size:];path.write_bytes(struct.pack('<4sII',b'glTF',2,12+len(chunks))+chunks)
(out/'vertex-verification.json').write_text(json.dumps(dict(baked_corners=count,bake_samples=scene.cycles.samples,display_transform=scene.view_settings.view_transform,unlit_vertex_colors=True,texture_uvs_required=False),indent=2)+'\n')
print('LANDSCAPE_VERTEX_BAKE_COMPLETE',count,flush=True)
