"""Bake narrow landscape surfaces to vertex colors, avoiding tiny UV islands.
Blender --python bake-landscape.py -- STUDY_DIRECTORY [SAMPLES]
Ground shadows and all twelve rooms retain full-resolution texture atlases.
"""
import bpy, json, math, struct, sys
from pathlib import Path
import numpy as np

sys.path.insert(0,str(Path(__file__).resolve().parent))
from common import *
from glass import glass_revision
from terrain import split_ground
from tree_casters import install_tree_casters
from rocks import install_rocks
from cottage_walls import prepare_cottage_walls, smooth_panel_lighting
from wayside_cottages import prepare_joinery_lighting, restore_joinery_geometry
args=sys.argv[sys.argv.index('--')+1:];out=Path(args[0]);out.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(DAY/'models/terraced-village-daylight.blend'),load_ui=False,use_scripts=False)
rock_revision=install_rocks()
tree_revision=install_tree_casters()
scene=bpy.context.scene;scene.cycles.samples=CONFIG['landscape_samples'];scene.cycles.use_denoising=False
assert sum(o.instance_type=='COLLECTION' and 'stop' in o for o in scene.objects)==12
split_ground()
prepare_joinery_lighting(bpy.data.collections['World terrain — blockout'])
cottage_centers=prepare_cottage_walls(bpy.data.collections['World terrain — blockout'])
members=[o for o in bpy.data.collections['World terrain — blockout'].objects if o.type=='MESH' and not o.name.startswith('Village tree ') and o.get('ground_tile',4)==4]
glass=[o for o in members if o.active_material.get('village_clear_glass')]
members=[o for o in members if o not in glass]
bpy.ops.object.select_all(action='DESELECT')
for obj in members:obj.select_set(True)
bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();obj=bpy.context.object;obj.name='Baked village landscape vertex lighting'
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
attribute=obj.data.color_attributes.new(name='WorldLighting',type='FLOAT_COLOR',domain='CORNER');obj.data.color_attributes.active_color=attribute
scene.render.bake.target='VERTEX_COLORS'
for key,value in dict(use_pass_diffuse=True,use_pass_glossy=False,use_pass_transmission=False,use_pass_emit=True,use_pass_direct=True,use_pass_indirect=True,use_pass_color=True).items():setattr(scene.render.bake,key,value)
bpy.ops.object.bake(type='COMBINED')
restore_joinery_geometry(obj.data)
# Use the exact same Blender view transform as the image atlases. The resulting
# display RGB is decoded to linear vertex color for the browser's sRGB output.
count=len(attribute.data);width=1024;height=math.ceil(count/width)
pixels=np.ones((width*height,4),dtype=np.float32);attribute.data.foreach_get('color',pixels[:count].reshape(-1))
palette=bpy.data.images.new('World vertex display palette',width=width,height=height,alpha=False,float_buffer=True);palette.colorspace_settings.name='Linear Rec.709';palette.pixels.foreach_set(pixels.reshape(-1))
scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_depth='16';palette.save_render(str(out/'landscape-vertex-palette.png'),scene=scene)
display=bpy.data.images.load(str(out/'landscape-vertex-palette.png'));display.colorspace_settings.name='Non-Color'
display.pixels.foreach_get(pixels.reshape(-1));colors=pixels[:count].copy();rgb=colors[:,:3]
colors[:,:3]=np.where(rgb<=.04045,rgb/12.92,((rgb+.055)/1.055)**2.4);colors[:,3]=1
smooth_panel_lighting(obj.data, colors)
attribute.data.foreach_set('color',colors.reshape(-1))
# Only export the final lighting, not the source albedo attribute.
for source_color in list(obj.data.color_attributes):
    if source_color.name!='WorldLighting':obj.data.color_attributes.remove(source_color)
obj.data.color_attributes.active_color=obj.data.color_attributes['WorldLighting']
obj.data.color_attributes.render_color_index=0
mat=bpy.data.materials.new('Baked world vertex lighting');mat.use_nodes=True
node=mat.node_tree.nodes.new('ShaderNodeVertexColor');node.layer_name='WorldLighting'
shader=mat.node_tree.nodes.get('Principled BSDF');shader.inputs['Roughness'].default_value=1;mat.node_tree.links.new(node.outputs['Color'],shader.inputs['Base Color'])
obj.data.materials.clear();obj.data.materials.append(mat)
for polygon in obj.data.polygons:polygon.material_index=0
for uv in list(obj.data.uv_layers):obj.data.uv_layers.remove(uv)
# Keep each compressed position range local. A single 460-unit-wide mesh
# would quantize small street lettering too coarsely at 16 bits.
regions={};assignments=[]
# Pack an entire cottage shell in one coordinate range. Independent rounding
# on opposite sides of a region boundary can open a hairline through the facade.
shell_attr=obj.data.attributes['CottageShell']
shell_ids=np.empty(len(obj.data.polygons),dtype=np.int32)
shell_attr.data.foreach_get('value',shell_ids)
obj.data.attributes.remove(shell_attr)
for polygon,shell in zip(obj.data.polygons,shell_ids):
    center=cottage_centers[shell] if shell else polygon.center
    key=(math.floor(center.x/64),math.floor(center.y/64))
    if key not in regions:regions[key]=len(regions)
    assignments.append(regions[key])
obj.data.materials.clear()
for key in regions:obj.data.materials.append(mat.copy())
obj.data.polygons.foreach_set('material_index',assignments)
bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT');bpy.ops.mesh.separate(type='MATERIAL');bpy.ops.object.mode_set(mode='OBJECT')
pieces=list(bpy.context.selected_objects)
assert sum(len(piece.data.loops) for piece in pieces)==count
for piece in pieces:
    piece.data.materials.clear();piece.data.materials.append(mat)
    for polygon in piece.data.polygons:polygon.material_index=0
for pane in glass:pane.select_set(True)
path=out/'landscape-vertex.glb'
bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True,export_extras=True)
data=path.read_bytes();size=struct.unpack_from('<I',data,12)[0];doc=json.loads(data[20:20+size])
baked_indices=[i for i,m in enumerate(doc['materials']) if m['name']=='Baked world vertex lighting']
assert len(baked_indices)==1 and glass
for mesh in doc['meshes']:
 for p in mesh['primitives']:
  if p['material'] in baked_indices:assert 'COLOR_0' in p['attributes']
for i in baked_indices:doc['materials'][i].setdefault('extensions',{})['KHR_materials_unlit']={}
doc.setdefault('extensionsUsed',[]).append('KHR_materials_unlit')
encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4)
chunks=struct.pack('<I4s',len(encoded),b'JSON')+encoded+data[20+size:];path.write_bytes(struct.pack('<4sII',b'glTF',2,12+len(chunks))+chunks)
(out/'vertex-verification.json').write_text(json.dumps(dict(cottage_walls_refined=len(cottage_centers),rock_geometry_sha256=rock_revision,tree_casters_sha256=tree_revision,glass_revision=glass_revision(),clear_glass_meshes=len(glass),lighting=CONFIG,full_world_occluders=True,stone_path_geometry_sha256=path_revision(),vegetation_geometry_sha256=vegetation_revision(),signage_geometry_sha256=signage_revision(),baked_corners=count,export_chunks=len(pieces),bake_samples=scene.cycles.samples,display_transform=scene.view_settings.view_transform,unlit_vertex_colors=True,texture_uvs_required=False),indent=2)+'\n')
print('LANDSCAPE_VERTEX_BAKE_COMPLETE',count,flush=True)
