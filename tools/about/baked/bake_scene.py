"""Bake one catalog scene without overwriting its master or fallback export.

blender -b --threads 2 --python tools/about/baked/bake_scene.py -- MODEL OUTPUT [SAMPLES] [--reference-only]
"""
import bpy, bmesh, json, math, hashlib, sys
from pathlib import Path
from mathutils import Vector, Matrix
sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[3]/"assets/models/terracotta-recipe-kitchen/source"))
from finalize_baked_glb import finalize

REPO=Path(__file__).resolve().parents[3]
args=sys.argv[sys.argv.index('--')+1:]
MODEL=args[0];OUT=Path(args[1]);OUT.mkdir(parents=True,exist_ok=True)
config=json.loads((Path(__file__).parent/'models.json').read_text())[MODEL]
assert not config.get('existing'), 'Use the archived first-kitchen bake script instead'
MASTER=REPO/config.get('master',config.get('export'))
sha=hashlib.sha256(MASTER.read_bytes()).hexdigest()
if 'master' in config:
 bpy.ops.wm.open_mainfile(filepath=str(MASTER))
else:
 from reconstruct_export import reconstruct
 reconstruct(MASTER,config['prints'])
scene=bpy.context.scene;scene.frame_set(160)
scene.render.engine='CYCLES';scene.cycles.samples=int(args[2]) if len(args)>2 else 128;scene.cycles.use_denoising=False
roots=[o for o in scene.objects if o.type=='EMPTY' and 'assembly' in o]
objects=sorted({o for r in roots for o in r.children_recursive if o.type in ('MESH','CURVE','FONT') and not o.hide_render},key=lambda o:o.name)
assert objects, 'No assembly geometry'
# Recovered exports already contain the approved companions.
if 'master' in config:
 plush=REPO/'assets/models/about-penguin-plushes'
 row=next(r for r in json.loads((plush/'placements.json').read_text())['models'] if r['model']==MODEL)
 for placement in row['plushes']:
  with bpy.data.libraries.load(str(plush/'models/penguini-plush.blend'),link=False)as(src,dst):dst.objects=src.objects
  root=next(o for o in dst.objects if o and o.get('variant')=='penguini');keep={root,*root.children_recursive}
  for o in dst.objects:
   if o in keep:scene.collection.objects.link(o)
   elif o:bpy.data.objects.remove(o,do_unlink=True)
  x,y,z=placement['position'];root.location=(x,-z,y);root.scale=(placement['height'],)*3;root.rotation_euler.z=math.radians(placement['yaw_degrees'])
  objects.extend(o for o in root.children_recursive if o.type=='MESH')
bpy.context.view_layer.update()
lights=[dict(name=o.name,position=[o.location.x,o.location.z,-o.location.y],rotation=list(o.rotation_euler),energy=o.data.energy,size=getattr(o.data,'size',5.),color=list(o.data.color))for o in scene.objects if o.type=='LIGHT']
bg=next(n for n in scene.world.node_tree.nodes if n.type=='BACKGROUND')
meta=dict(model=MODEL,source=str(MASTER.relative_to(REPO)),source_kind='master' if 'master' in config else 'reconstructed-export',source_sha256=sha,frame=160,view_transform=scene.view_settings.view_transform,look=scene.view_settings.look,exposure=scene.view_settings.exposure,world_color=list(bg.inputs['Color'].default_value[:3]),world_strength=bg.inputs['Strength'].default_value,lights=lights)
# Reference uses the untouched source shaders, original lights, and approved camera.
scene.render.resolution_x=1080;scene.render.resolution_y=945;scene.render.resolution_percentage=100
camera=scene.camera
projection=camera.calc_matrix_camera(bpy.context.evaluated_depsgraph_get(),x=1080,y=945)
view=camera.matrix_world.inverted()@Matrix.Rotation(math.pi/2,4,'X')
vp=projection@view
meta['reference_camera']=dict(vp=[vp[row][col] for col in range(4) for row in range(4)],view=[view[row][col] for col in range(4) for row in range(4)],eye=[camera.location.x,camera.location.z,-camera.location.y],width=1080,height=945)
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(OUT/'master-reference.png')
previous_path=OUT/(MODEL+'-lighting.json')
previous=json.loads(previous_path.read_text()) if previous_path.exists() else {}
reuse_lighting='--reuse-lighting' in args
if reuse_lighting:
 assert 'export' in config and previous.get('source_sha256')==sha, 'Lighting reuse requires the same recovered source'
 assert previous.get('frame')==160 and previous.get('atlas_size')==2048
 for label in ('architecture','details'):assert (OUT/(label+'-baked.png')).exists()
reference_stale=any(previous.get(k)!=meta[k] for k in ('source_sha256','reference_camera','view_transform','look','exposure'))
if '--reference-only' in args or reference_stale or not (OUT/'master-reference.png').exists():
 samples=scene.cycles.samples;scene.cycles.samples=min(32,samples)
 bpy.ops.render.render(write_still=True);scene.cycles.samples=samples
if '--reference-only' in args:
 assert hashlib.sha256(MASTER.read_bytes()).hexdigest()==sha
 print('REFERENCE_COMPLETE',MODEL,flush=True)
 sys.exit(0)
# Evaluate modifiers and retain procedural coordinate systems before merging.
bpy.ops.object.select_all(action='DESELECT')
for o in objects:o.select_set(True)
bpy.context.view_layer.objects.active=objects[0];bpy.ops.object.convert(target='MESH')
objects=list(bpy.context.selected_objects)
for o in objects:
 mx=o.matrix_world.copy();o.parent=None;o.matrix_world=mx;o.animation_data_clear()
 o.data=o.data.copy();mesh=o.data
 if not mesh.polygons:continue
 # Some authored parametric spheres/lathes have inward winding. Cycles
 # renders both sides, but bake rays otherwise start inside their dark interior.
 # Repair only the derivative; retain source geometry and the archived master.
 if 'master' in config:
  bm=bmesh.new();bm.from_mesh(mesh);bm.normal_update()
  lo=Vector(tuple(min(v.co[i] for v in bm.verts) for i in range(3)))
  hi=Vector(tuple(max(v.co[i] for v in bm.verts) for i in range(3)))
  spans=hi-lo;planar=min(spans)<max(spans)*1e-6
  if planar:
   # Shallow painted relief is two-sided in Cycles' camera render. Bake its
   # visible face, rather than sending lighting rays into the backing canvas.
   normal=sum((f.normal*f.calc_area() for f in bm.faces),Vector())
   normal=o.matrix_world.to_3x3().inverted().transposed()@normal
   center=o.matrix_world@((lo+hi)/2)
   reverse=normal.dot(camera.location-center)<0
   repair='front_facing_relief_repairs'
  else:
   # Signed volume only identifies inward CLOSED shells. Weld coincident
   # UV seams in a temporary probe, leaving the export mesh untouched.
   # Open folded foliage and uncapped arches have no meaningful volume.
   probe=bm.copy()
   bmesh.ops.remove_doubles(probe,verts=list(probe.verts),dist=1e-6)
   bmesh.ops.dissolve_degenerate(probe,edges=list(probe.edges),dist=1e-8)
   probe.normal_update()
   closed=not any(e.is_boundary for e in probe.edges)
   center=sum((v.co for v in probe.verts),Vector())/len(probe.verts)
   bmesh.ops.translate(probe,vec=-center,verts=list(probe.verts))
   reverse=closed and probe.calc_volume(signed=True)<-1e-8;probe.free()
   repair='outward_winding_repairs'
  if reverse:
   bmesh.ops.reverse_faces(bm,faces=list(bm.faces));bm.to_mesh(mesh)
   if mesh.has_custom_normals:mesh.normals_split_custom_set([(0.,0.,0.)]*len(mesh.loops))
   meta.setdefault(repair,[]).append(o.name)
  bm.free()
 for name,values in [('MasterGenerated',[tuple((v.co[i]-mesh.texspace_location[i])/(2*max(mesh.texspace_size[i],1e-8))+.5 for i in range(3))for v in mesh.vertices]),('MasterObject',[tuple(v.co)for v in mesh.vertices])]:
  attr=mesh.attributes.new(name,'FLOAT_VECTOR','POINT')
  for item,value in zip(attr.data,values):item.vector=value
 uv=mesh.uv_layers.active.name if mesh.uv_layers.active else None
 for mat in mesh.materials:
  if not mat or not mat.use_nodes:continue
  tree=mat.node_tree
  for node in list(tree.nodes):
   if node.type=='TEX_COORD':
    for socket,key in [('Generated','MasterGenerated'),('Object','MasterObject')]:
     for link in list(node.outputs[socket].links):
      attr=tree.nodes.new('ShaderNodeAttribute');attr.attribute_name=key;tree.links.new(attr.outputs['Vector'],link.to_socket)
   if node.type.startswith('TEX_') and node.type not in ('TEX_COORD','TEX_IMAGE') and node.inputs.get('Vector') and not node.inputs['Vector'].is_linked:
    attr=tree.nodes.new('ShaderNodeAttribute');attr.attribute_name='MasterGenerated';tree.links.new(attr.outputs['Vector'],node.inputs['Vector'])
   if node.type=='TEX_IMAGE' and uv and not node.inputs['Vector'].is_linked:
    u=tree.nodes.new('ShaderNodeUVMap');u.uv_map=uv;tree.links.new(u.outputs['UV'],node.inputs['Vector'])
   if node.type=='NORMAL_MAP' and uv and not node.uv_map:node.uv_map=uv
# Split reflective/transmitting polygons out; all original geometry is retained.
opaque=[];dynamic=[]
for o in objects:
 if not o.data.polygons:continue
 kinds=[]
 for mat in o.data.materials:
  p=next((n for n in mat.node_tree.nodes if n.type=='BSDF_PRINCIPLED'),None)if mat and mat.use_nodes else None
  kinds.append(bool(p and (p.inputs['Metallic'].default_value>.3 or p.inputs['Transmission Weight'].default_value>.1)))
 flags={kinds[p.material_index] for p in o.data.polygons}
 if len(flags)==1:(dynamic if next(iter(flags)) else opaque).append(o);continue
 for flag in (False,True):
  copy=o.copy();copy.data=o.data.copy();scene.collection.objects.link(copy)
  bm=bmesh.new();bm.from_mesh(copy.data);bmesh.ops.delete(bm,geom=[f for f in bm.faces if kinds[f.material_index]!=flag],context='FACES');bm.to_mesh(copy.data);bm.free()
  (dynamic if flag else opaque).append(copy)
 bpy.data.objects.remove(o,do_unlink=True)
# Larger architecture and small food/props get separate 2048px atlases.
large=[];small=[]
for o in opaque:
 (large if max(o.dimensions)>.85 else small).append(o)
baked=[]
for label,members in [('architecture',large),('details',small)]:
 bpy.ops.object.select_all(action='DESELECT')
 for o in members:o.select_set(True)
 bpy.context.view_layer.objects.active=members[0];bpy.ops.object.join();o=bpy.context.object;o.name='Baked '+MODEL+' '+label
 bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
 # Keep source UV layers intact; use a new UV layer only for the lighting bake.
 uv=o.data.uv_layers.new(name='BakedLighting');o.data.uv_layers.active=uv
 bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
 bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.001,margin_method='FRACTION');bpy.ops.object.mode_set(mode='OBJECT')
 # Dedicated copies prevent another atlas from changing these bake targets.
 for i,m in enumerate(list(o.data.materials)):o.data.materials[i]=m.copy()
 image=bpy.data.images.new(MODEL+' '+label+' lighting',width=2048,height=2048,alpha=False,float_buffer=True);image.colorspace_settings.name='Linear Rec.709'
 for m in o.data.materials:
  node=m.node_tree.nodes.new('ShaderNodeTexImage');node.image=image;m.node_tree.nodes.active=node
 scene.render.bake.target='IMAGE_TEXTURES';scene.render.bake.margin=8;scene.render.bake.use_clear=True
 scene.render.bake.use_pass_direct=True;scene.render.bake.use_pass_indirect=True;scene.render.bake.use_pass_color=True
 # Diffuse + emission keeps the original detail and stationary direct/indirect
 # shadows; view-dependent reflective surfaces are exported separately below.
 scene.render.bake.use_pass_diffuse=True;scene.render.bake.use_pass_glossy=False;scene.render.bake.use_pass_transmission=False;scene.render.bake.use_pass_emit=True
 print('REUSE_LIGHTING' if reuse_lighting else 'BAKE_START',label,flush=True)
 if not reuse_lighting:bpy.ops.object.bake(type='COMBINED',uv_layer='BakedLighting')
 # Preserve linear HDR pixels and apply the master's AgX look only to the PNG.
 scene.render.image_settings.file_format='OPEN_EXR'
 if not reuse_lighting:image.save_render(str(OUT/(label+'-linear.exr')),scene=scene)
 scene.render.image_settings.file_format='PNG'
 if not reuse_lighting:image.save_render(str(OUT/(label+'-baked.png')),scene=scene)
 baked.append((o,label))
# Replace only export materials after BOTH bakes; original shaders light each other.
for o,label in baked:
 tex=bpy.data.images.load(str(OUT/(label+'-baked.png')));tex.colorspace_settings.name='sRGB';tex.pack()
 mat=bpy.data.materials.new('Baked master lighting '+label);mat.use_nodes=True;mat.node_tree.nodes.clear()
 image=mat.node_tree.nodes.new('ShaderNodeTexImage');image.image=tex
 uv=mat.node_tree.nodes.new('ShaderNodeUVMap');uv.uv_map='BakedLighting';mat.node_tree.links.new(uv.outputs[0],image.inputs['Vector'])
 emission=mat.node_tree.nodes.new('ShaderNodeEmission');mat.node_tree.links.new(image.outputs['Color'],emission.inputs['Color'])
 output=mat.node_tree.nodes.new('ShaderNodeOutputMaterial');mat.node_tree.links.new(emission.outputs[0],output.inputs['Surface'])
 o.data.materials.clear();o.data.materials.append(mat)
 for p in o.data.polygons:p.material_index=0
 # Source UV layers are no longer needed for display-baked opaque geometry.
 for layer in list(o.data.uv_layers):
  if layer.name!='BakedLighting':o.data.uv_layers.remove(layer)
 for attr in list(o.data.attributes):
  if attr.name in ('MasterGenerated','MasterObject') or attr.data_type in ('FLOAT_COLOR','BYTE_COLOR'):o.data.attributes.remove(attr)
# Batch dynamic surfaces by authored material while preserving transforms.
bpy.ops.object.select_all(action='DESELECT')
for o in dynamic:o.select_set(True)
assert dynamic, 'Expected glass or metal in source'
bpy.context.view_layer.objects.active=dynamic[0];bpy.ops.object.join();live=bpy.context.object;live.name='Live glass and reflective metal'
bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
# Remove procedural links unsupported by glTF only from live material copies.
for i,m in enumerate(list(live.data.materials)):
 copy=m.copy();live.data.materials[i]=copy
 for n in copy.node_tree.nodes:
  if n.type=='BSDF_PRINCIPLED':
   for key in ('Base Color','Roughness','Normal'):
    for link in list(n.inputs[key].links):
     # glTF supports image textures and vertex colors. Recovered glass/metal
     # rely on vertex tint, so retain those links instead of exporting gray.
     if key=='Base Color' and link.from_node.type in ('VERTEX_COLOR','TEX_IMAGE'):continue
     copy.node_tree.links.remove(link)
bpy.ops.object.select_all(action='DESELECT')
for o,_ in baked:o.select_set(True)
live.select_set(True)
triangles=0
for o in bpy.context.selected_objects:o.data.calc_loop_triangles();triangles+=len(o.data.loop_triangles)
meta.update(triangles=triangles,atlases=2,atlas_size=2048,bake_samples=scene.cycles.samples,animations=False,penguini=True)
bpy.ops.export_scene.gltf(filepath=str(OUT/(MODEL+'-baked.glb')),export_format='GLB',use_selection=True,export_apply=True,export_animations=False,export_cameras=False,export_lights=False,export_yup=True,export_extras=True)
finalize(OUT/(MODEL+'-baked.glb'))
meta['glb_bytes']=(OUT/(MODEL+'-baked.glb')).stat().st_size
(OUT/(MODEL+'-lighting.json')).write_text(json.dumps(meta,indent=2)+'\n')
assert hashlib.sha256(MASTER.read_bytes()).hexdigest()==sha
print('BAKE_COMPLETE',json.dumps(meta),flush=True)
