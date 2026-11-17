"""Bake stationary kitchen surfaces; preserve the 45.2-second animated sequence.
Adapted from Noms tools/about/baked/bake_scene.py, read 2026-11-16.
Run: blender -b -t 6 --python bake_static.py
"""
import bpy,bmesh,math,json,hashlib,sys,os
from pathlib import Path
from mathutils import Vector
R=Path(__file__).parent;sys.path.insert(0,str(R))
from finalize_baked_glb import finalize
MASTER=R/'penguini-cookie-kitchen-baking.blend';sha=hashlib.sha256(MASTER.read_bytes()).hexdigest()
bpy.ops.wm.open_mainfile(filepath=str(MASTER));s=bpy.context.scene;s.frame_set(0)
s.render.engine='CYCLES';s.cycles.samples=int(os.environ.get('BAKE_SAMPLES','128'));s.cycles.use_denoising=False
for layer in s.view_layers:layer.cycles.use_denoising=False
studio=bpy.data.collections['STUDIO | Cameras and soft lighting'];character=set(bpy.data.collections['PENGUINI • editable model'].all_objects)
def moving(o):
 if o in character or any(m.type=='ARMATURE' for m in o.modifiers):return True
 p=o
 while p:
  if p.animation_data and (p.animation_data.action or p.animation_data.drivers):return True
  p=p.parent
 return False
static=sorted([o for o in s.objects if o.type in {'MESH','CURVE','FONT'} and studio not in o.users_collection and not moving(o)],key=lambda o:o.name)
animated={o for o in s.objects if studio not in o.users_collection and moving(o)}
static_names=[o.name for o in static];animated_names=sorted(o.name for o in animated)
# No permanent shadows from the mascot, book, utensils, cups or ingredients.
hidden={o:o.hide_render for o in animated}
for o in animated:o.hide_render=True
bpy.context.view_layer.update()
meta={'source_sha256':sha,'method':'About-tour diffuse direct + indirect + emission, display-baked unlit atlases','bake_samples':s.cycles.samples,'atlas_size':2048,'view_transform':s.view_settings.view_transform,'look':s.view_settings.look,'exposure':s.view_settings.exposure,'static_source_objects':static_names,'excluded_animated_objects':animated_names,'animated_shadow_casters_during_bake':0,'duration_seconds':45.2,'lights':[{'name':o.name,'energy':o.data.energy,'color':list(o.data.color),'position':list(o.matrix_world.translation)} for o in s.objects if o.type=='LIGHT']}
# Reflection lighting for the shared About/Home Three.js renderer (glTF Y-up).
meta['palette']=s.get('palette','')
meta['palette_materials']=json.loads(s.get('palette_materials','{}'))
world=s.world.node_tree.nodes.get('Background')
meta['world_color']=list(world.inputs['Color'].default_value[:3])
meta['world_strength']=world.inputs['Strength'].default_value
meta['lights']=[dict(name=o.name,position=[o.location.x,o.location.z,-o.location.y],rotation=list(o.rotation_euler),size=o.data.size,energy=o.data.energy,color=list(o.data.color)) for o in s.objects if o.type=='LIGHT' and o.data.type=='AREA']
# Evaluate only the stationary derivative. The editable master stays untouched.
bpy.ops.object.select_all(action='DESELECT')
for o in static:o.select_set(True)
bpy.context.view_layer.objects.active=static[0];bpy.ops.object.convert(target='MESH');static=list(bpy.context.selected_objects)
material_copies={}
for o in static:
 mw=o.matrix_world.copy();o.parent=None;o.matrix_world=mw;o.animation_data_clear();o.data=o.data.copy();mesh=o.data
 if not mesh.polygons:continue
 bm=bmesh.new();bm.from_mesh(mesh);bm.normal_update()
 probe=bm.copy();bmesh.ops.remove_doubles(probe,verts=list(probe.verts),dist=1e-6);probe.normal_update()
 closed=not any(e.is_boundary for e in probe.edges)
 center=sum((v.co for v in probe.verts),Vector())/max(1,len(probe.verts));bmesh.ops.translate(probe,vec=-center,verts=list(probe.verts))
 reverse=closed and probe.calc_volume(signed=True)<-1e-8;probe.free()
 if reverse:
  bmesh.ops.reverse_faces(bm,faces=list(bm.faces));bm.to_mesh(mesh)
  if mesh.has_custom_normals:mesh.normals_split_custom_set([(0.,0.,0.)]*len(mesh.loops))
  meta.setdefault('outward_winding_repairs',[]).append(o.name)
 bm.free()
 for name,values in [('MasterGenerated',[tuple((v.co[i]-mesh.texspace_location[i])/(2*max(mesh.texspace_size[i],1e-8))+.5 for i in range(3)) for v in mesh.vertices]),('MasterObject',[tuple(v.co) for v in mesh.vertices])]:
  attr=mesh.attributes.new(name,'FLOAT_VECTOR','POINT')
  for item,value in zip(attr.data,values):item.vector=value
 uv=mesh.uv_layers.active.name if mesh.uv_layers.active else None
 for i,mat in enumerate(list(mesh.materials)):
  if mat not in material_copies:
   clone=mat.copy();material_copies[mat]=clone
   if clone.use_nodes:
    tree=clone.node_tree
    for node in list(tree.nodes):
     if node.type=='TEX_COORD':
      for socket,key in [('Generated','MasterGenerated'),('Object','MasterObject')]:
       for link in list(node.outputs[socket].links):
        attr=tree.nodes.new('ShaderNodeAttribute');attr.attribute_name=key;tree.links.new(attr.outputs['Vector'],link.to_socket)
     if node.type.startswith('TEX_') and node.type not in ('TEX_COORD','TEX_IMAGE') and node.inputs.get('Vector') and not node.inputs['Vector'].is_linked:
      attr=tree.nodes.new('ShaderNodeAttribute');attr.attribute_name='MasterGenerated';tree.links.new(attr.outputs['Vector'],node.inputs['Vector'])
     if node.type=='TEX_IMAGE' and uv and not node.inputs['Vector'].is_linked:
      u=tree.nodes.new('ShaderNodeUVMap');u.uv_map=uv;tree.links.new(u.outputs['UV'],node.inputs['Vector'])
  mesh.materials[i]=material_copies[mat]
opaque=[];live=[]
for o in static:
 if not o.data.polygons:continue
 kinds=[]
 for mat in o.data.materials:
  p=next((n for n in mat.node_tree.nodes if n.type=='BSDF_PRINCIPLED'),None) if mat and mat.use_nodes else None
  kinds.append(bool(p and (p.inputs['Metallic'].default_value>.3 or p.inputs['Transmission Weight'].default_value>.1 or p.inputs['Alpha'].default_value<.999)))
 flags={kinds[p.material_index] for p in o.data.polygons}
 if len(flags)==1:(live if next(iter(flags)) else opaque).append(o);continue
 for flag in [False,True]:
  cp=o.copy();cp.data=o.data.copy();s.collection.objects.link(cp)
  bm=bmesh.new();bm.from_mesh(cp.data);bmesh.ops.delete(bm,geom=[f for f in bm.faces if kinds[f.material_index]!=flag],context='FACES');bm.to_mesh(cp.data);bm.free()
  (live if flag else opaque).append(cp)
 bpy.data.objects.remove(o,do_unlink=True)
# Separate small details from large surfaces to keep printed jar labels readable.
groups=[('architecture',[o for o in opaque if max(o.dimensions)>.85]),('details',[o for o in opaque if max(o.dimensions)<=.85])]
baked=[]
for label,members in groups:
 bpy.ops.object.select_all(action='DESELECT')
 for o in members:o.select_set(True)
 bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();o=bpy.context.object;o.name='Static kitchen | Baked '+label
 bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
 uv=o.data.uv_layers.new(name='BakedLighting');o.data.uv_layers.active=uv
 bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT');bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.002,margin_method='FRACTION');bpy.ops.object.mode_set(mode='OBJECT')
 for i,m in enumerate(list(o.data.materials)):o.data.materials[i]=m.copy()
 image=bpy.data.images.new('Kitchen '+label+' lighting',width=2048,height=2048,alpha=False,float_buffer=True);image.colorspace_settings.name='Linear Rec.709'
 for m in o.data.materials:
  node=m.node_tree.nodes.new('ShaderNodeTexImage');node.image=image;m.node_tree.nodes.active=node
 s.render.bake.target='IMAGE_TEXTURES';s.render.bake.margin=8;s.render.bake.use_clear=True
 for key in ['direct','indirect','color','diffuse','emit']:setattr(s.render.bake,'use_pass_'+key,True)
 s.render.bake.use_pass_glossy=False;s.render.bake.use_pass_transmission=False
 reuse=label=='architecture' and os.environ.get('REUSE_ARCHITECTURE')=='1'
 if reuse:
  assert (R/'architecture-linear.exr').exists() and (R/'architecture-baked.png').exists()
  assert (R/'architecture-source.sha256').read_text().strip()==sha
  meta['architecture_reused_after_transparency_split']=True
  print('REUSE_ARCHITECTURE',flush=True)
 else:
  print('BAKE_START',label,flush=True);bpy.ops.object.bake(type='COMBINED',uv_layer='BakedLighting')
  s.render.image_settings.file_format='OPEN_EXR';image.save_render(str(R/(label+'-linear.exr')),scene=s)
  s.render.image_settings.file_format='PNG';s.render.image_settings.color_mode='RGB';s.render.image_settings.color_depth='8';image.save_render(str(R/(label+'-baked.png')),scene=s)
  if label=='architecture':(R/'architecture-source.sha256').write_text(sha)
 baked.append((o,label));print('BAKE_COMPLETE',label,flush=True)
# Change shaders only after both atlases finish, so original surfaces bounce light.
for o,label in baked:
 tex=bpy.data.images.load(str(R/(label+'-baked.png')));tex.colorspace_settings.name='sRGB';tex.pack()
 mat=bpy.data.materials.new('Baked master lighting '+label);mat.use_nodes=True;nodes=mat.node_tree.nodes;nodes.clear()
 image=nodes.new('ShaderNodeTexImage');image.image=tex;uv=nodes.new('ShaderNodeUVMap');uv.uv_map='BakedLighting';mat.node_tree.links.new(uv.outputs[0],image.inputs['Vector'])
 emission=nodes.new('ShaderNodeEmission');mat.node_tree.links.new(image.outputs['Color'],emission.inputs['Color']);out=nodes.new('ShaderNodeOutputMaterial');mat.node_tree.links.new(emission.outputs[0],out.inputs['Surface'])
 o.data.materials.clear();o.data.materials.append(mat)
 for p in o.data.polygons:p.material_index=0
 for layer in list(o.data.uv_layers):
  if layer.name!='BakedLighting':o.data.uv_layers.remove(layer)
 for attr in list(o.data.attributes):
  if attr.name in ('MasterGenerated','MasterObject'):o.data.attributes.remove(attr)
# Keep actual metallic surfaces as PBR rather than freezing camera reflections.
if live:
 bpy.ops.object.select_all(action='DESELECT')
 for o in live:o.select_set(True)
 bpy.context.view_layer.objects.active=live[0];bpy.ops.object.join();live=[bpy.context.object];live[0].name='Static kitchen | Live metal and steam';bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
for o,hide in hidden.items():o.hide_render=hide
bpy.ops.object.select_all(action='DESELECT')
export_objects=list(animated)+[o for o,_ in baked]+live
for o in export_objects:o.select_set(True)
s.frame_set(0)
# Cache the baked export scene for material/export iterations without rebaking.
bpy.ops.wm.save_as_mainfile(filepath=str(R/'baked-export-cache.blend'))
bpy.ops.export_scene.gltf(filepath=str(R/'penguini-cookie-kitchen-baking.glb'),export_format='GLB',use_selection=True,export_apply=True,export_animations=True,export_force_sampling=True,export_frame_range=True,export_frame_step=1,export_cameras=False,export_lights=False,export_animation_mode='SCENE',export_anim_scene_split_object=False)
finalize(R/'penguini-cookie-kitchen-baking.glb')
meta['static_meshes_after']=len(baked)+len(live);meta['atlases']=2;meta['moving_objects_baked']=False
(R/'lighting-bake.json').write_text(json.dumps(meta,indent=2));assert hashlib.sha256(MASTER.read_bytes()).hexdigest()==sha
print('BAKED_ANIMATED_EXPORT_COMPLETE',flush=True)
