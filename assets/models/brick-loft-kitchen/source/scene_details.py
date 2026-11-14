"""Reusable original fittings, food and studio setup for the Noms collection."""
for k,h in {'clay':'#BD765E','sageLight':'#A4B3A1','sageDark':'#526B59','honey':'#D4A45D','navy':'#303F49','teal':'#527E7C','pink':'#CC9A94','red':'#BC5142','leaf':'#647D48','sky':'#C6DAD8','brass':'#AD8A51','linen':'#D9CDB5','white':'#F7F0E2','crust':'#AC6B32','crustLight':'#D1A05D','ink':'#343C35','mint':'#ABC2AB'}.items():
 M[k]=material(k,h,.80,.38 if k=='brass' else 0)

def text(n,words,loc,size=.12,mat='ivory',rot=(pi/2,0,0),align='CENTER'):
 d=bpy.data.curves.new(n,'FONT');d.body=words;d.align_x=align;d.size=size;d.extrude=.0006;d.bevel_depth=.0002;d.resolution_u=4
 o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.rotation_euler=rot;assign(o,mat);return o

def board(n,words,x,y,z,w,h,mat='sageDark',size=.14):
 box(n+' walnut frame',(x,y,z),(w,.065,h),'walnut2',.025)
 box(n+' painted face',(x,y-.037,z),(w-.065,.009,h-.065),mat,.008)
 text(n+' lettering',words,(x,y-.044,z+.03),size,'ink' if mat in ['ivory','paper','white','linen'] else 'paper')

def shell(w,d,h=3.4,tiles=False,wall='wall',window=True):
 collection('01 Rounded foundation and floor',offset=(0,0,0))
 rounded_slab('Rounded walnut diorama plinth',(0,0,-.13),w+.16,d+.16,.26,.23,'walnut2')
 rounded_slab('Continuous floor',(0,0,.015),w,d,.05,.18,'stone' if tiles else 'floor')
 if tiles:
  nx,ny=int(w/.45),int(d/.45)
  for j in range(ny):
   for i in range(nx):
    x=-w/2+(i+.5)*w/nx;y=-d/2+(j+.5)*d/ny
    box('Handmade floor tile',(x,y,.052),(w/nx-.012,d/ny-.012,.035),'ivory' if (i+j)%2 else 'clay',.009)
 else:
  ny=int(d/.29)
  for j in range(ny):
   y=-d/2+(j+.5)*d/ny;cuts=[-w/2];a=-w/2+(.6 if j%2 else 1.2)
   while a<w/2:cuts.append(a);a+=1.5
   cuts.append(w/2)
   for i,(a,b) in enumerate(zip(cuts,cuts[1:])):box('Individual timber floorboard',((a+b)/2,y,.047),(b-a-.006,d/ny-.007,.024),['oak','oak2','oak3'][(j+i)%3],.003)
 collection('02 Back wall and skirting',offset=(0,-.3,.25))
 box('Continuous rear wall',(0,d/2,h/2),(w+.12,.14,h),wall,.025)
 box('Rear oak skirting',(0,d/2-.086,.14),(w,.045,.20),'walnut2',.009)
 collection('03 Left wall and window',offset=(.45,-.2,.2))
 if window:
  ya,yb=-d*.25,d*.20;za,zb=1.3,2.85
  for n,cy,dy,cz,dz in [('lower',0,d,za/2,za),('upper',0,d,(h+zb)/2,h-zb),('near',(-d/2+ya)/2,ya+d/2,(za+zb)/2,zb-za),('far',(yb+d/2)/2,d/2-yb,(za+zb)/2,zb-za)]:
   box('Window wall '+n,(-w/2,cy,cz),(.14,dy,dz),wall,.012)
  box('Window softly blue glazing',(-w/2-.016,(ya+yb)/2,(za+zb)/2),(.016,yb-ya,zb-za),'sky',.002)
  for y in [ya,(ya+yb)/2,yb]:box('Window mullion',(-w/2+.07,y,(za+zb)/2),(.11,.055,zb-za+.13),'white',.008)
  for z in [za,(za+zb)/2,zb]:box('Window horizontal frame',(-w/2+.07,(ya+yb)/2,z),(.11,yb-ya+.14,.06),'white',.008)
  box('Deep window sill',(-w/2+.14,(ya+yb)/2,za),(.34,yb-ya+.22,.075),'oak',.014)
 else:box('Continuous left wall',(-w/2,0,h/2),(.14,d,h),wall,.025)
 box('Left oak skirting',(-w/2+.086,0,.14),(.045,d,.20),'walnut2',.009)

def table(n,x,y,z,w,d,mat='oak'):
 rounded_slab(n+' rounded top',(x,y,z),w,d,.095,.10,mat)
 for dx in [-w/2+.14,w/2-.14]:
  for dy in [-d/2+.14,d/2-.14]:rod(n+' tapered leg',(x+dx*1.05,y+dy*1.05,.065),(x+dx,y+dy,z-.05),.045,'walnut2')
 box(n+' front apron',(x,y-d/2+.10,z-.17),(w-.18,.055,.18),'walnut2',.008)
 return z+.05

def chair(n,loc,angle=0,mat='sage'):
 def fn():
  rounded_slab(n+' seat',(0,0,.51),.54,.53,.09,.13,'walnut2')
  pill(n+' upholstered cushion',(0,-.015,.57),(.48,.46,.085),mat)
  for x in [-.19,.19]:
   for y in [-.18,.18]:rod(n+' splayed leg',(x*1.3,y*1.3,.07),(x,y,.49),.025,'walnut2')
  for x in [-.23,.23]:rod(n+' back support',(x,.20,.48),(x,.24,1.04),.022,'walnut2')
  curve(n+' curved bentwood back',[(-.25,.18,.91),(-.20,.29,1.01),(0,.32,1.03),(.20,.29,1.01),(.25,.18,.91)],.058,mat)
 local_group(n,loc,angle,fn)

def latte(n,x,y,z,r=.08):
 h=r*1.48
 lathe(n+' hollow ceramic cup',(x,y,z),[(0,0),(.62*r,0),(.73*r,.012),(r,h*.9),(r,h),(r-.008,h),(r-.012,h*.9),(.60*r,.018),(0,.018)],'ceramic',48)
 curve(n+' attached handle',[(x+r*.9,y,z+h*.8),(x+r*1.6,y,z+h*.84),(x+r*1.65,y,z+h*.34),(x+r*.72,y,z+h*.2)],.011,'ceramic')
 cyl(n+' crema',(x,y,z+h*.87),r-.013,.003,'crust',48)
 for i in range(5):
  yy=y-.035+i*.012;wid=.042*(1-i*.14)
  curve(n+' rosetta foam',[(x-wid,yy,z+h*.894),(x,yy+.017,z+h*.896),(x+wid,yy,z+h*.894)],.0035,'paper')
 curve(n+' milk stem',[(x,y-.044,z+h*.895),(x,y+.037,z+h*.895)],.0023,'paper')
 plate(n+' saucer',x,y,z-.018,r*1.55)

def jug(n,x,y,z):
 lathe(n+' stainless body',(x,y,z),[(0,0),(.065,0),(.071,.015),(.063,.15),(.060,.157),(.055,.148),(.060,.020),(0,.020)],'steel',48)
 curve(n+' handle',[(x+.060,y,z+.12),(x+.115,y,z+.13),(x+.12,y,z+.055),(x+.067,y,z+.035)],.009,'steel')
 mesh(n+' pouring lip',[(x-.055,y-.025,z+.14),(x-.09,y,z+.166),(x-.055,y+.025,z+.14),(x-.050,y,z+.133)],[(0,1,2,3)],'steel')

def loaf(n,x,y,z,length=.48,angle=0):
 def fn():
  uv(n+' browned loaf',(0,0,.12),(length/2,.16,.14),'crustLight',40,20)
  for i in range(4):
   xx=-length*.31+i*length*.20
   pts=[(xx-.026,-.095),(xx,0),(xx+.026,.095)]
   curve(n+' opened score',[(a,b,.12+.14*math.sqrt(max(0,1-(a/(length/2))**2-(b/.16)**2))+.001)for a,b in pts],.009,'dough')
 local_group(n,(x,y,z),angle,fn)

def baguette(n,x,y,z,length=.80,angle=0):
 def fn():
  uv(n+' baked body',(0,0,.075),(.085,length/2,.085),'crustLight',40,16)
  for i in range(5):
   yy=-length*.34+i*length*.17
   curve(n+' diagonal scoring',[(a,b,.075+.085*math.sqrt(max(0,1-(a/.085)**2-(b/(length/2))**2))+.001)for a,b in [(-.05,yy-.022),(0,yy),(.05,yy+.022)]],.006,'dough')
 local_group(n,(x,y,z),angle,fn)

def jar(n,x,y,z,contents='honey',r=.065,h=.19):
 lathe(n+' rounded glass',(x,y,z),[(0,0),(.8*r,0),(r,.012),(r,h*.85),(.86*r,h*.94),(.86*r,h),(0,h)],'glass',40)
 cyl(n+' contents',(x,y,z+h*.46),r*.93,h*.81,contents,40)
 cyl(n+' metal lid',(x,y,z+h+.013),r*.92,.028,'brass',40)
 box(n+' paper label',(x,y-r-.001,z+h*.48),(r*1.35,.003,h*.40),'paper',.006)
 text(n+' label','NOMS',(x,y-r-.004,z+h*.48),.025,'ink')

def crate(n,x,y,z,w=.8,d=.56,h=.35):
 box(n+' base',(x,y,z+.025),(w,d,.045),'oak2',.008)
 for dx in [-w/2+.035,w/2-.035]:
  for dy in [-d/2+.035,d/2-.035]:box(n+' corner post',(x+dx,y+dy,z+h/2),(.055,.055,h),'walnut2',.006)
 for i in range(3):
  zz=z+.07+i*(h-.1)/3
  for dy in [-d/2,d/2]:box(n+' long open slat',(x,y+dy,zz),(w,.025,.065),'oak',.006)
  for dx in [-w/2,w/2]:box(n+' end open slat',(x+dx,y,zz),(.025,d,.065),'oak3',.006)

def fruit(n,x,y,z,kind='apple',r=.065):
 ma={'apple':'red','lemon':'lemon','orange':'orange','pear':'leaf','tomato':'tomato'}[kind]
 uv(n+' fruit',(x,y,z+r*.85),(r,r*.93,r*.85),ma,20,12)
 if kind=='pear':uv(n+' pear neck',(x,y,z+r*1.48),(r*.53,r*.53,r*.55),ma,20,10)
 rod(n+' short stem',(x,y,z+r*1.60),(x+.007,y,z+r*1.82),.006,'walnut',10)
 o=uv(n+' leaf',(x+.026,y,z+r*1.67),(.035,.014,.004),'leaf',12,6);o.rotation_euler.z=.5

def bao(n,x,y,z,r=.09):
 N,L=48,20;v=[]
 for j in range(L+1):
  t=pi*j/L;rr=r*sin(t);zz=z+r*(1-cos(t))*.80
  for i in range(N):
   a=2*pi*i/N;crease=1-.045*(sin(t)**.5)*cos(a*12)*(j/L)**2
   v.append((x+rr*cos(a)*crease,y+rr*sin(a)*crease,zz))
 f=[(j*N+i,j*N+(i+1)%N,(j+1)*N+(i+1)%N,(j+1)*N+i)for j in range(L)for i in range(N)]
 smooth(mesh(n+' pleated bun',v,f,'dough'))
 for i in range(10):
  a=i*2*pi/10;curve(n+' pinched pleat',[(x+r*.7*cos(a),y+r*.7*sin(a),z+r*1.36),(x+r*.35*cos(a+.2),y+r*.35*sin(a+.2),z+r*1.56),(x+r*.07*cos(a+.4),y+r*.07*sin(a+.4),z+r*1.60)],.0025,'dough')

def steamer(n,x,y,z,r=.28,filled=True):
 lathe(n+' hollow bamboo wall',(x,y,z),[(r*.92,0),(r,0),(r,.13),(r-.016,.13),(r-.016,.018),(r*.92,0)],'bamboo',64)
 cyl(n+' bamboo floor',(x,y,z+.015),r*.95,.015,'bamboo',48)
 for zz in [.016,.112]:torus(n+' binding band',(x,y,z+zz),r,.008,'bambooDark',steps=48)
 for i in range(32):
  a=2*pi*i/32;rod(n+' stave',(x+r*cos(a),y+r*sin(a),z+.025),(x+r*cos(a),y+r*sin(a),z+.1),.002,'bambooDark',8)
 if filled:
  for i in range(3):
   a=i*2*pi/3;xx=x+r*.45*cos(a);yy=y+r*.45*sin(a)
   cyl(n+' parchment',(xx,yy,z+.026),r*.37,.002,'paper',24);bao(n,xx,yy,z+.028,r*.34)

def noodle_bowl(n,x,y,z,r=.18):
 lathe(n+' ceramic bowl',(x,y,z),[(0,0),(.45*r,0),(.7*r,.04),(r,.15),(r-.008,.16),(.92*r,.145),(.55*r,.04),(0,.03)],'ceramic',48)
 cyl(n+' broth',(x,y,z+.115),r*.85,.008,'gravy',40)
 for i in range(8):
  yy=y+(i-3.5)*.023;pts=[(x-.11+j*.036,yy+.008*sin(j*1.9+i),z+.126+.003*sin(j+i))for j in range(7)]
  curve(n+' separate noodles',pts,.005,'pasta')
 uv(n+' egg white',(x+.065,y+.03,z+.14),(.05,.034,.014),'paper',24,12);uv(n+' egg yolk',(x+.065,y+.026,z+.154),(.023,.018,.006),'honey',20,10)
 for i in range(3):uv(n+' greens',(x-.055+i*.025,y+.075,z+.14),(.026,.045,.008),'leaf',16,8)
 for dx in [-.022,.022]:rod(n+' chopstick',(x+r+.018+dx,y-.15,z+.05),(x+r-.03+dx,y+.19,z+.18),.004,'walnut2',10)

def pot(n,x,y,z,r=.12):
 lathe(n+' terracotta pot',(x,y,z),[(0,0),(.7*r,0),(r,.19),(r+.005,.20),(.84*r,.20),(.80*r,.035),(0,.035)],'clay',40)
 cyl(n+' soil',(x,y,z+.175),r*.8,.012,'walnut',24)
 for i in range(8):
  a=i*2.4;rr=.035+random.random()*.04;end=(x+rr*cos(a),y+rr*sin(a),z+.28+random.random()*.11)
  curve(n+' stem',[(x,y,z+.18),end],.003,'leaf');o=uv(n+' herb leaf',end,(.04,.018,.008),'leaf',16,8);o.rotation_euler=(.3,.3,a)

def pendant(n,x,y,z,night=False):
 rod(n+' cord',(x,y,z+.46),(x,y,z),.009,'black',12)
 lathe(n+' spun shade',(x,y,z),[(.27,0),(.26,.045),(.15,.16),(.07,.21),(0,.21)],'clay' if not night else 'navy',48)
 uv(n+' warm bulb',(x,y,z+.015),(.055,.055,.045),'led',24,12)

def area(n,loc,energy,size,col,target=(0,0,0)):
 d=bpy.data.lights.new(n,'AREA');d.energy=energy;d.shape='DISK';d.size=size;d.color=col;o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.rotation_euler=(Vector(target)-o.location).to_track_quat('-Z','Y').to_euler();return o

def finish(key,title,scale=10.2,target=(0,0,1.1),night=False):
 out=R.parent;scene=bpy.context.scene;scene.unit_settings.system='METRIC';scene['model_name']=title;scene['revision']=1;scene['style']='Noms matte miniature diorama';scene['animation_status']='Static review model; separate assembly roots for future animation'
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
 scene.render.engine='CYCLES';scene.cycles.device='CPU';scene.cycles.samples=64;scene.cycles.use_denoising=False;scene.cycles.max_bounces=7;scene.cycles.transmission_bounces=6;scene.cycles.transparent_max_bounces=8
 scene.render.threads_mode='FIXED';scene.render.threads=2;scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.76,.82,.88,1);scene.world.node_tree.nodes['Background'].inputs[1].default_value=.30 if not night else .13
 area('Broad warm key',(-5,-7,10),1400 if not night else 650,6,(1,.91,.82))
 area('Soft cool fill',(7,-2,7),1000 if not night else 700,5,(.88,.94,1))
 area('Roofless overhead softbox',(0,5,10),1200 if not night else 450,5,(1,.94,.84))
 d=bpy.data.cameras.new('Camera Hero');o=bpy.data.objects.new('Camera Hero',d);COL.objects.link(o);o.location=(10,-14,10);o.rotation_euler=(Vector(target)-o.location).to_track_quat('-Z','Y').to_euler();d.type='ORTHO';d.ortho_scale=scale;scene.camera=o
 scene.render.resolution_x=1600;scene.render.resolution_y=1400;scene.render.resolution_percentage=100;scene.render.image_settings.file_format='PNG';scene.render.film_transparent=False;scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast';scene.view_settings.exposure=.15
 for o in list(bpy.data.objects):
  if not o.users_collection:bpy.data.objects.remove(o,do_unlink=True)
 bpy.context.preferences.filepaths.save_version=0;bpy.ops.file.pack_all()
 notes=bpy.data.texts.new('README');notes.write(title+'\nOriginal Noms review model. Native editable curves, text, bevels and materials.\nAssembly roots are static and carry suggested incoming offsets. No baked placement shadows on walls or ground.\nNative Cycles lights produce dynamic shadows. GLB includes PBR materials and geometry; lighting is supplied by the host viewer.\nOriginal models using shared Noms modeling primitives.\n')
 for screen in bpy.data.screens:
  for a in screen.areas:
   if a.type=='VIEW_3D':a.spaces.active.region_3d.view_perspective='CAMERA'
 scene.render.filepath=f'//../previews/{key}.png'
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
 if '--proof' in sys.argv:scene.render.resolution_percentage=65;scene.cycles.samples=24
 scene.render.filepath=str(out/'previews'/f'{key}.png');bpy.ops.render.render(write_still=True);print('RENDER_COMPLETE',key,flush=True)
