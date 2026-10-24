"""Build a single-draw web GLB from Sam's approved Blender master.
Usage: blender -b MASTER --threads 5 --python tools/about/export_kitchen.py -- OUTPUT_DIR
The master is read only. Keep the native source separately from this generated asset.
"""
import bpy,sys,math,json
from pathlib import Path
out=Path(sys.argv[sys.argv.index('--')+1]);out.mkdir(parents=True,exist_ok=True)
s=bpy.context.scene;s.cycles.use_denoising=False
bpy.ops.object.select_all(action='DESELECT')
for o in bpy.data.objects:
 if o.type in ['MESH','CURVE'] and o.name!='Studio ground':
  o.select_set(True)
  if o.type=='CURVE':o.data.resolution_u=8;o.data.bevel_resolution=min(o.data.bevel_resolution,3)
  for m in o.modifiers:
   if m.type=='BEVEL':m.segments=min(4,m.segments)
   if o.name.startswith('Checker tile') and m.type in ['BEVEL','WEIGHTED_NORMAL']:o.modifiers.remove(m)
bpy.context.view_layer.objects.active=next(o for o in bpy.context.selected_objects if o.type=='MESH');bpy.ops.object.convert(target='MESH')
objects=list(bpy.context.selected_objects)
# Keep radial/rim geometry intact: collapse decimation makes these silhouettes angular.
# Simplification is concentrated on less visible surfaces and foliage instead.
protected_prefixes=('Wok ·','Burger meal ·','Burger ·','Cookies · porcelain plate','Stool · rounded terracotta seat','Soda ·','Onion ·','Croissant tray ·','Espresso ·','Fridge ·')
protected_words=('bowl','porcelain','carafe','pot','dripper','rounded sill','ivory frame')
geometry_audit=[]
for o in objects:
 o.data.calc_loop_triangles();tris=len(o.data.loop_triangles)
 protected=o.name.startswith(protected_prefixes) or any(word in o.name.lower() for word in protected_words)
 if protected or o.name.startswith('Walls ·'):ratio=1
 elif 'rolled triangular dough' in o.name:ratio=min(1,3600/max(1,tris))
 elif o.name.startswith(('Cookie','Bread knife','Fries')):ratio=.65
 elif any(c.name=='02 · Terracotta cabinetry' for c in o.users_collection):ratio=.5
 else:ratio=.45 if tris>150 else 1
 if ratio<1:
  o.data=o.data.copy()
  bpy.context.view_layer.objects.active=o;m=o.modifiers.new('Web silhouette budget','DECIMATE');m.ratio=ratio;bpy.ops.object.modifier_apply(modifier=m.name)
 o.data.calc_loop_triangles()
 geometry_audit.append({'name':o.name,'source_triangles':tris,'web_triangles':len(o.data.loop_triangles),'preserved':protected})
 if protected:assert len(o.data.loop_triangles)==tris,o.name
(out/'kitchen-geometry-audit.json').write_text(json.dumps(geometry_audit,indent=2))
# Bake low-frequency contact shading to vertex colors instead of fragmenting curved surfaces into UV islands.
# This also avoids a separate texture decode/upload and keeps the model to a single draw call.
bpy.context.view_layer.objects.active=objects[0];bpy.ops.object.join();o=bpy.context.object;o.name='Sams kitchen · web diorama'
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
base=[]
for m in o.data.materials:
 p=m.node_tree.nodes.get('Principled BSDF') if m.use_nodes else None
 rgb=list(p.inputs['Base Color'].default_value[:3] if p else m.diffuse_color[:3])
 if 'glass' in m.name.lower():rgb=[.44,.62,.56]
 if 'baked outer dough' in m.name.lower():rgb=[.56,.24,.055]
 base.append(rgb)
attr=o.data.color_attributes.new(name='KitchenColor',type='FLOAT_COLOR',domain='CORNER');o.data.color_attributes.active_color=attr
s.render.engine='CYCLES';s.cycles.samples=16;s.render.bake.target='VERTEX_COLORS'
bpy.ops.object.bake(type='AO',use_clear=True)
for poly in o.data.polygons:
 rgb=base[poly.material_index]
 for li in poly.loop_indices:
  ao=max(.68,min(1.,attr.data[li].color[0]));attr.data[li].color=(*[c*(.38+.62*ao) for c in rgb],1)
mat=bpy.data.materials.new('Kitchen · vertex color and soft contact shading');mat.use_nodes=True;p=mat.node_tree.nodes.get('Principled BSDF');p.inputs['Roughness'].default_value=.65
v=mat.node_tree.nodes.new('ShaderNodeVertexColor');v.layer_name='KitchenColor';mat.node_tree.links.new(v.outputs['Color'],p.inputs['Base Color'])
o.data.materials.clear();o.data.materials.append(mat)
for face in o.data.polygons:face.material_index=0
for name in [layer.name for layer in o.data.uv_layers]:o.data.uv_layers.remove(o.data.uv_layers[name])
bpy.ops.export_scene.gltf(filepath=str(out/'sams-kitchen.glb'),export_format='GLB',use_selection=True,export_apply=True,export_cameras=False,export_lights=False,export_yup=True,export_materials='EXPORT')
o.data.calc_loop_triangles();report={'source_revision':11,'geometry_profile':'curved props preserved','triangles':len(o.data.loop_triangles),'draw_calls':1,'textures':0,'contact_shading':'baked vertex ambient occlusion','glb_bytes':(out/'sams-kitchen.glb').stat().st_size,'native_master_unchanged':True};(out/'kitchen-budget.json').write_text(json.dumps(report,indent=2));print('WEB_ASSET',json.dumps(report),flush=True)
