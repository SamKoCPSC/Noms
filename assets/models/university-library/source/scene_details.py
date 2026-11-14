"""Library fittings and export setup, adapted from the Noms community collection."""
for k,h in {'clay':'#BD765E','sageLight':'#A4B3A1','sageDark':'#526B59','honey':'#D4A45D','navy':'#303F49','teal':'#527E7C','pink':'#CC9A94','red':'#BC5142','leaf':'#647D48','sky':'#C6DAD8','brass':'#AD8A51','linen':'#D9CDB5','white':'#F7F0E2','crust':'#AC6B32','crustLight':'#D1A05D','ink':'#343C35','mint':'#ABC2AB'}.items():
 M[k]=material(k,h,.80,.38 if k=='brass' else 0)


def text(n,words,loc,size=.12,mat='ivory',rot=(pi/2,0,0),align='CENTER'):
 d=bpy.data.curves.new(n,'FONT');d.body=words;d.align_x=align;d.size=size;d.extrude=.0006;d.bevel_depth=.0002;d.resolution_u=4
 o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.rotation_euler=rot;assign(o,mat);return o


def table(n,x,y,z,w,d,mat='oak'):
 rounded_slab(n+' rounded top',(x,y,z),w,d,.095,.10,mat)
 for dx in [-w/2+.14,w/2-.14]:
  for dy in [-d/2+.14,d/2-.14]:rod(n+' tapered leg',(x+dx*1.05,y+dy*1.05,.065),(x+dx,y+dy,z-.05),.045,'walnut2')
 box(n+' front apron',(x,y-d/2+.10,z-.17),(w-.18,.055,.18),'walnut2',.008)
 return z+.05


def area(n,loc,energy,size,col,target=(0,0,0)):
 d=bpy.data.lights.new(n,'AREA');d.energy=energy;d.shape='DISK';d.size=size;d.color=col;o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.rotation_euler=(Vector(target)-o.location).to_track_quat('-Z','Y').to_euler();return o


def finish(key,title,scale=10.2,target=(0,0,1.1),night=False,revision=1,camera_position=(10,-14,10),render_threads=2,samples=48):
 out=R.parent;scene=bpy.context.scene;scene.unit_settings.system='METRIC';scene['model_name']=title;scene['revision']=revision;scene['style']='Noms matte miniature diorama';scene['animation_status']='Static review model; separate assembly roots for future animation'
 bpy.context.view_layer.update();manifest=[]
 for root in GROUPS:
  obs=[o for o in root.children_recursive if o.type in ['MESH','CURVE','FONT']];pts=[o.matrix_world@Vector(v)for o in obs for v in o.bound_box]
  if not pts:continue
  lo=[min(p[i]for p in pts)for i in range(3)];hi=[max(p[i]for p in pts)for i in range(3)];root['pivot']=[(a+b)/2 for a,b in zip(lo,hi)]
  manifest.append(dict(name=root.name,members=len(obs),bounds=[lo,hi],pivot=list(root['pivot']),incoming_offset=list(root['incoming_offset'])))
 (out/'models'/f'{key}-assemblies.json').write_text(json.dumps({'model':title,'units':'meters','animation':'metadata only','assemblies':manifest},indent=2))
 collection('99 Studio lighting and cameras',assembly=False)
 M['studio']=material('Studio backdrop','#E6E1D6' if not night else '#45545C',.9)
 box('Studio ground',(0,0,-.32),(200,200,.1),'studio',0)
 scene.render.engine='CYCLES';scene.cycles.device='CPU';scene.cycles.samples=samples;scene.cycles.use_denoising=False;scene.cycles.max_bounces=7;scene.cycles.transmission_bounces=6;scene.cycles.transparent_max_bounces=8
 scene.render.threads_mode='FIXED';scene.render.threads=render_threads;scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.76,.82,.88,1);scene.world.node_tree.nodes['Background'].inputs[1].default_value=.30 if not night else .13
 area('Broad warm key',(-5,-7,10),1400 if not night else 650,6,(1,.91,.82))
 area('Soft cool fill',(7,-2,7),1000 if not night else 700,5,(.88,.94,1))
 area('Roofless overhead softbox',(0,5,10),1200 if not night else 450,5,(1,.94,.84))
 d=bpy.data.cameras.new('Camera Hero');o=bpy.data.objects.new('Camera Hero',d);COL.objects.link(o);o.location=camera_position;o.rotation_euler=(Vector(target)-o.location).to_track_quat('-Z','Y').to_euler();d.type='ORTHO';d.ortho_scale=scale;scene.camera=o
 scene.render.resolution_x=1600;scene.render.resolution_y=1400;scene.render.resolution_percentage=100;scene.render.image_settings.file_format='PNG';scene.render.film_transparent=False;scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast';scene.view_settings.exposure=.15
 for o in list(bpy.data.objects):
  if not o.users_collection:bpy.data.objects.remove(o,do_unlink=True)
 bpy.context.preferences.filepaths.save_version=0;bpy.ops.file.pack_all()
 notes=bpy.data.texts.new('README');notes.write(title+'\nOriginal Noms review model. Native editable curves, text, bevels and materials.\nAssembly roots are static and carry suggested incoming offsets. No baked occlusion shadows.\nNative Cycles lights produce dynamic shadows. GLB includes PBR materials and geometry; lighting is supplied by the host viewer.\nReconstructed from the retained UBC-inspired university library brief. No external assets required.\n')
 for screen in bpy.data.screens:
  for a in screen.areas:
   if a.type=='VIEW_3D':a.spaces.active.region_3d.view_perspective='CAMERA'
 bpy.ops.object.select_all(action='DESELECT');bpy.ops.wm.save_as_mainfile(filepath=str(out/'models'/f'{key}.blend'),compress=True)
 print('MASTER_SAVED',key,len(bpy.data.objects),flush=True)
 # Evaluated derivatives include editable curve and text geometry in glTF.
 dg=bpy.context.evaluated_depsgraph_get();copies=[];exportroots=[]
 for root in GROUPS:
  e=bpy.data.objects.new(root.name+' export',None);scene.collection.objects.link(e)
  for k,v in root.items():e[k]=v
  exportroots.append(e)
  for src in root.children_recursive:
   if src.type not in ['MESH','CURVE','FONT']:continue
   data=bpy.data.meshes.new_from_object(src.evaluated_get(dg),preserve_all_data_layers=True,depsgraph=dg)
   if not len(data.vertices):continue
   obj=bpy.data.objects.new(src.name+' export',data);scene.collection.objects.link(obj);obj.parent=e;obj.matrix_world=src.matrix_world.copy();copies.append(obj)
 bpy.ops.object.select_all(action='DESELECT')
 for o in copies+exportroots:o.select_set(True)
 bpy.ops.export_scene.gltf(filepath=str(out/'models'/f'{key}.glb'),export_format='GLB',use_selection=True,export_apply=False,export_cameras=False,export_lights=False,export_animations=False,export_extras=True,export_yup=True)
 for o in copies+exportroots:bpy.data.objects.remove(o,do_unlink=True)
 print('GLB_SAVED',key,flush=True)
 if '--proof' in sys.argv:scene.render.resolution_percentage=55;scene.cycles.samples=16
 scene.render.filepath=str(out/'previews'/f'{key}.png');bpy.ops.render.render(write_still=True);print('RENDER_COMPLETE',key,flush=True)
