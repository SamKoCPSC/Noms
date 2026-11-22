"""Build a grouped web GLB from Sam's approved Blender master.
Usage: blender -b MASTER --threads 5 --python tools/about/export_kitchen.py -- OUTPUT_DIR [--surface-materials]
The master is read only. Keep the native source separately from this generated asset.
"""
import bpy,sys,json
from pathlib import Path
out=Path(sys.argv[sys.argv.index('--')+1]);out.mkdir(parents=True,exist_ok=True)
s=bpy.context.scene;s.frame_set(160);s.cycles.use_denoising=False
surface_materials='--surface-materials' in sys.argv
manifest=json.loads((Path(bpy.data.filepath).parent/'assembly-manifest.json').read_text())
groups=manifest['assembly_groups']
membership={name:i for i,g in enumerate(groups) for name in g['members']}
# Appliance-prefixed base cabinets belong to the architectural shell, not the
# mixer/blender fly-in. Keep their countertops supported throughout the opening.
for name in membership:
 if name.startswith(('Mixer cabinet ·','Blender cabinet ·')):membership[name]=2
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
for o in objects:
 if surface_materials and o.data.uv_layers.active:
  # Each source mesh can use a different UV layer (notably the croissants).
  # Make implicit texture coordinates explicit before joining the objects.
  uv_name=o.data.uv_layers.active.name
  for mat in o.data.materials:
   if not mat or not mat.use_nodes:continue
   for tex in [n for n in mat.node_tree.nodes if n.type=='TEX_IMAGE' and not n.inputs['Vector'].is_linked]:
    uv_node=mat.node_tree.nodes.new('ShaderNodeUVMap');uv_node.uv_map=uv_name
    mat.node_tree.links.new(uv_node.outputs['UV'],tex.inputs['Vector'])
 a=o.data.attributes.new(name='Assembly',type='INT',domain='FACE')
 t=o.data.attributes.new(name='RecipeTexture',type='INT',domain='FACE')
 for face in o.data.polygons:
  o.data.attributes['Assembly'].data[face.index].value=membership[o.name]
  mat=o.data.materials[face.material_index] if len(o.data.materials)>face.material_index else None
  o.data.attributes['RecipeTexture'].data[face.index].value=1 if mat and mat.name=='Laptop · actual Noms editor capture' else 2 if mat and mat.name=='Cookbook · readable sourdough recipe' else 0
# Keep radial/rim geometry intact: collapse decimation makes these silhouettes angular.
# Simplification is concentrated on less visible surfaces and foliage instead.
protected_prefixes=('Wok ·','Burger meal ·','Burger ·','Cookies · porcelain plate','Stool · rounded terracotta seat','Soda ·','Onion ·','Croissant tray ·','Espresso ·','Fridge ·','Laptop ·','Recipe book ·')
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
# Preserve procedural coordinates before batching each assembly. This retains
# the source material mapping while avoiding hundreds of separate bake setups.
if surface_materials:
 for obj in objects:
  loc=obj.data.texspace_location.copy();size=obj.data.texspace_size.copy()
  coords=obj.data.attributes.new(name='MasterGenerated',type='FLOAT_VECTOR',domain='POINT')
  for vertex,value in zip(obj.data.vertices,coords.data):
   value.vector=tuple((vertex.co[i]-loc[i])/(2*max(size[i],1e-8))+.5 for i in range(3))
  for mat in obj.data.materials:
   if not mat or not mat.use_nodes:continue
   tree=mat.node_tree
   for node in list(tree.nodes):
    if node.type.startswith('TEX_') and node.type not in ('TEX_IMAGE','TEX_COORD') and node.inputs.get('Vector') and not node.inputs['Vector'].is_linked:
     attr_node=tree.nodes.new('ShaderNodeAttribute');attr_node.attribute_name='MasterGenerated'
     tree.links.new(attr_node.outputs['Vector'],node.inputs['Vector'])
    if node.type=='TEX_COORD':
     for link in list(node.outputs['Generated'].links):
      attr_node=tree.nodes.new('ShaderNodeAttribute');attr_node.attribute_name='MasterGenerated'
      tree.links.new(attr_node.outputs['Vector'],link.to_socket)
 assemblies=[]
 by_group={gi:[obj for obj in objects if membership[obj.name]==gi] for gi in range(len(groups))}
 for members in by_group.values():
  bpy.ops.object.select_all(action='DESELECT')
  for obj in members:obj.select_set(True)
  bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();obj=bpy.context.object
  mx=obj.matrix_world.copy();obj.parent=None;obj.matrix_world=mx
  bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
  assemblies.append(obj)
 objects=assemblies
 for obj in objects:obj.select_set(True)

if surface_materials:
 sys.path.insert(0,str(Path(__file__).resolve().parent))
 from bake_kitchen_surfaces import bake_surfaces
 bake_surfaces(objects,s,out)

bpy.context.view_layer.objects.active=objects[0];bpy.ops.object.join();o=bpy.context.object;o.name='Sams kitchen · web diorama'
mx=o.matrix_world.copy();o.parent=None;o.matrix_world=mx
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
o.data.color_attributes.new(name='KitchenColor',type='FLOAT_COLOR',domain='CORNER')
if surface_materials:o.data.attributes.new(name='_METALLIC',type='FLOAT',domain='CORNER')
# Creating another CustomData layer can invalidate Blender's previous RNA handles.
attr=o.data.color_attributes['KitchenColor'];o.data.color_attributes.active_color=attr
if surface_materials:metal_attr=o.data.attributes['_METALLIC']
if surface_materials:
 # Retain source albedo and metallic factors alongside the unlit surface atlases.
 base=[]
 for mat in o.data.materials:
  shader=mat.node_tree.nodes.get('Principled BSDF') if mat.use_nodes else None
  rgb=list(shader.inputs['Base Color'].default_value[:3] if shader else mat.diffuse_color[:3])
  base.append(rgb)
 for poly in o.data.polygons:
  rgb=base[poly.material_index] if o.data.attributes['RecipeTexture'].data[poly.index].value==3 else [1.,1.,1.]
  mat=o.data.materials[poly.material_index];shader=mat.node_tree.nodes.get('Principled BSDF') if mat.use_nodes else None
  metallic=shader.inputs['Metallic'].default_value if shader else 1.
  for li in poly.loop_indices:
   attr.data[li].color=(*rgb,1.)
   if surface_materials:metal_attr.data[li].value=metallic
 atlas=o.data.uv_layers['KitchenSurfaceAtlas']
 for gi in range(len(groups)):
  uv={tuple(atlas.data[li].uv) for p in o.data.polygons
      if o.data.attributes['Assembly'].data[p.index].value==gi
      for li in p.loop_indices}
  assert len(uv)>3, f'Missing baked UVs for {groups[gi]["name"]}'
else:
 s.render.engine='CYCLES';s.cycles.samples=16;s.render.bake.target='VERTEX_COLORS'
 base=[]
 for m in o.data.materials:
  p=m.node_tree.nodes.get('Principled BSDF') if m.use_nodes else None
  rgb=list(p.inputs['Base Color'].default_value[:3] if p else m.diffuse_color[:3])
  if 'glass' in m.name.lower():rgb=[.44,.62,.56]
  if 'baked outer dough' in m.name.lower():rgb=[.56,.24,.055]
  base.append(rgb)
 bpy.ops.object.bake(type='AO',use_clear=True)
 for poly in o.data.polygons:
  rgb=[1.,1.,1.] if o.data.attributes['RecipeTexture'].data[poly.index].value else base[poly.material_index]
  for li in poly.loop_indices:
   ao=max(.68,min(1.,attr.data[li].color[0]))
   attr.data[li].color=(*[c*(.38+.62*ao) for c in rgb],1)
# One material/primitive per assembly, with two extra textured print primitives.
o.data.materials.clear();batches={}
for face in o.data.polygons:
 gi=o.data.attributes['Assembly'].data[face.index].value
 ti=o.data.attributes['RecipeTexture'].data[face.index].value
 key=(gi,ti)
 if key not in batches:
  g=groups[gi];mat=bpy.data.materials.new(g['name']+f' · print {ti}');mat.use_nodes=True
  mat['assembly']={'name':g['name'],'pivot':[g['pivot'][0],g['pivot'][2],-g['pivot'][1]],'offset':[g['incoming_offset'][0],g['incoming_offset'][2],-g['incoming_offset'][1]],'start':g['start_frame'],'end':g['end_frame'],'texture':ti,'surfaceMaterials':surface_materials}
  p=mat.node_tree.nodes.get('Principled BSDF');v=mat.node_tree.nodes.new('ShaderNodeVertexColor');v.layer_name='KitchenColor';mat.node_tree.links.new(v.outputs['Color'],p.inputs['Base Color'])
  batches[key]=len(o.data.materials);o.data.materials.append(mat)
 face.material_index=batches[key]
bpy.ops.export_scene.gltf(filepath=str(out/'sams-kitchen.glb'),export_format='GLB',use_selection=True,export_apply=True,export_cameras=False,export_lights=False,export_yup=True,export_materials='EXPORT',export_extras=True,export_attributes=surface_materials)
o.data.calc_loop_triangles();report={'source_revision':13,'geometry_profile':'curved props preserved','triangles':len(o.data.loop_triangles),'draw_calls':len(batches),'textures':4 if surface_materials else 2,'assembly_groups':18,'contact_shading':'2048px unlit albedo/local cavity and object-normal/roughness atlases; live lighting and cast shadows' if surface_materials else 'baked vertex ambient occlusion','glb_bytes':(out/'sams-kitchen.glb').stat().st_size,'native_master_unchanged':True};(out/'kitchen-budget.json').write_text(json.dumps(report,indent=2));print('WEB_ASSET',json.dumps(report),flush=True)
