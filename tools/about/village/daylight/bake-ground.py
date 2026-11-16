"""Bake four ground tiles against the complete native village, including room shadows."""
import bpy,json,sys,hashlib
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from common import *
from terrain import split_ground
sys.path.insert(0,str(REPO/'assets/models/terracotta-recipe-kitchen/source'))
from finalize_baked_glb import finalize
out=Path(sys.argv[sys.argv.index('--')+1]);out.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(DAY/'models/terraced-village-daylight.blend'),load_ui=False,use_scripts=False)
scene=bpy.context.scene;scene.cycles.samples=CONFIG['ground_samples'];tiles=split_ground();baked=[]
assert sum(o.instance_type=='COLLECTION' and 'stop' in o for o in scene.objects)==12
for i,obj in enumerate(tiles[:4]):
    bpy.ops.object.select_all(action='DESELECT');obj.select_set(True);bpy.context.view_layer.objects.active=obj
    uv=obj.data.uv_layers.new(name='BakedLighting');obj.data.uv_layers.active=uv
    x0,x1,z0,z1=obj['tile_bounds']
    for polygon in obj.data.polygons:
        for loop in polygon.loop_indices:
            v=obj.data.vertices[obj.data.loops[loop].vertex_index].co;uv.data[loop].uv=((v.x-x0)/(x1-x0),(-v.y-z0)/(z1-z0))
    image=bpy.data.images.new('Ground tile '+str(i),width=2048,height=2048,alpha=False,float_buffer=True);image.colorspace_settings.name='Linear Rec.709'
    node=obj.active_material.node_tree.nodes.new('ShaderNodeTexImage');node.image=image;obj.active_material.node_tree.nodes.active=node
    scene.render.bake.target='IMAGE_TEXTURES';scene.render.bake.margin=8;scene.render.bake.use_clear=True
    for key,value in dict(use_pass_diffuse=True,use_pass_glossy=False,use_pass_transmission=False,use_pass_emit=True,use_pass_direct=True,use_pass_indirect=True,use_pass_color=True).items():setattr(scene.render.bake,key,value)
    print('GROUND_BAKE_START',i,flush=True);bpy.ops.object.bake(type='COMBINED',uv_layer='BakedLighting')
    scene.render.image_settings.file_format='OPEN_EXR';image.save_render(str(out/f'ground-{i}.exr'),scene=scene)
    scene.render.image_settings.file_format='PNG';image.save_render(str(out/f'ground-{i}.png'),scene=scene);baked.append((obj,i))
for obj,i in baked:
    mat=bpy.data.materials.new('Baked master lighting ground '+str(i));mat.use_nodes=True;mat.node_tree.nodes.clear()
    tex=mat.node_tree.nodes.new('ShaderNodeTexImage');tex.image=bpy.data.images.load(str(out/f'ground-{i}.png'));tex.image.colorspace_settings.name='sRGB'
    uv=mat.node_tree.nodes.new('ShaderNodeUVMap');uv.uv_map='BakedLighting';mat.node_tree.links.new(uv.outputs[0],tex.inputs['Vector'])
    emit=mat.node_tree.nodes.new('ShaderNodeEmission');mat.node_tree.links.new(tex.outputs['Color'],emit.inputs['Color'])
    output=mat.node_tree.nodes.new('ShaderNodeOutputMaterial');mat.node_tree.links.new(emit.outputs[0],output.inputs['Surface']);obj.data.materials.clear();obj.data.materials.append(mat)
bpy.ops.object.select_all(action='DESELECT')
for obj,_ in baked:obj.select_set(True)
path=out/'ground.glb';bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True);finalize(path,expected_count=4)
(out/'ground.json').write_text(json.dumps(dict(lighting=CONFIG,tiles=4,atlas_size=2048,full_world_occluders=True,stone_path_geometry_sha256=path_revision(),vegetation_geometry_sha256=vegetation_revision(),triangles=sum(len(o.data.polygons) for o,_ in baked)),indent=2)+'\n')
print('GROUND_BAKE_COMPLETE',flush=True)
