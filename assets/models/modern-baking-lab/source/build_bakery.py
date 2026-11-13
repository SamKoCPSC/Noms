"""Noms Modern Baking Lab — concept 10. Blender 4.3+, self-contained review master."""
from pathlib import Path
HERE=Path(__file__).resolve().parent
for d in ['models','previews']:(HERE.parent/d).mkdir(exist_ok=True)
exec(compile((HERE/'diorama_common.py').read_text(),str(HERE/'diorama_common.py'),'exec'))
code=(HERE/'scene_details.py').read_text().replace('scene.render.threads=2','scene.render.threads=6').replace('scene.cycles.samples=64','scene.cycles.samples=48').replace("scene['revision']=1","scene['revision']=3")
exec(compile(code,str(HERE/'scene_details.py'),'exec'))
exec(compile((HERE/'pastry.py').read_text(),str(HERE/'pastry.py'),'exec'))
for key,h,rough,metal in [('plaster','#E9E3D7',.93,0),('tile','#DDD9CF',.88,0),('grout','#B6B4AA',.95,0),('steelSoft','#AEB6B5',.48,.72),('steelLight','#CBD0CA',.45,.62),('steelInside','#5A6566',.54,.65),('screen','#253F42',.72,0),('digit','#B5D3C0',.65,0),('crustDark','#9C6034',.9,0)]:M[key]=material(key,h,rough,metal)
# Enough polish for clear glass and brushed steel, with matte cabinetry and food.
M['glass'].node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.07

def tray(n,x,y,z,w,d,parchment=True):
 rounded_slab(n+' baking sheet',(x,y,z),w,d,.016,.035,'steelInside')
 curve(n+' raised rolled rim',rounded_points(x,y,z+.023,w,d,.034),.010,'steelSoft',True,True)
 if parchment:rounded_slab(n+' parchment',(x,y,z+.012),w-.05,d-.05,.002,.018,'paper')

def pull(n,x,y,z,w=.32):
 for xx in [x-w/2,x+w/2]:rod(n+' fixed bracket',(xx,y+.035,z),(xx,y-.015,z),.014,'steelInside',12)
 rod(n+' bar',(x-w/2,y-.015,z),(x+w/2,y-.015,z),.018,'steelSoft',20)

def dial(n,x,y,z,r=.035):
 o=cyl(n,(x,y,z),r,.026,'black',32);o.rotation_euler.x=pi/2
 rod(n+' index',(x,y-.016,z+r*.45),(x,y-.016,z+r*.78),.002,'paper',8)

def boule(n,x,y,z,r=.20):
 uv(n+' round crust',(x,y,z+r*.60),(r,r*.93,r*.65),'crustLight',40,20)
 # Score lines follow the curved top instead of floating above a flat surface.
 for off in [-.07,.045]:
  pts=[]
  for j in range(13):
   xx=(-.73+j*.122)*r;yy=off+xx*.24;zz=z+r*.6+r*.65*math.sqrt(max(0,1-(xx/r)**2-(yy/(r*.93))**2))
   pts.append((x+xx,y+yy,zz+.002))
  curve(n+' opened flour score',pts,.010,'dough')

def jar_label(n,label,x,y,z,r=.115,h=.30,fill='paper'):
 jar(n,x,y,z,fill,r,h)
 # Replace the generic label with a readable ingredient name.
 o=bpy.data.objects.get(n+' label')
 if o:o.data.body=label;o.data.size=.032 if len(label)<6 else .027

def cupboard(n,x,y,w,d=.72):
 box(n+' recessed toe kick',(x,y,.14),(w-.1,d-.07,.18),'gap',.012)
 box(n+' oak carcass',(x,y,.56),(w,d,.78),'oak',.02)
 doors=max(1,round(w/.62))
 for i in range(doors):
  xx=x-w/2+(i+.5)*w/doors
  box(n+' inset door',(xx,y-d/2-.015,.57),(w/doors-.025,.035,.70),'oak3',.008)
  pull(n+' door handle',xx,y-d/2-.055,.82,min(.28,w/doors*.6))
 rounded_slab(n+' brushed worktop',(x,y,.985),w+.06,d+.08,.055,.035,'steelLight')

collection('01 Tiled floor and oak platform',offset=(0,0,0))
rounded_slab('Oak miniature base',(.75,-.575,-.10),9.5,7.40,.24,.15,'oak2')
rounded_slab('Grout bed',(.75,-.575,.035),9.36,7.25,.05,.10,'grout')
for j in range(10):
 for i in range(12):box('Warm grey porcelain floor tile',(-3.9+(i+.5)*.775,-4.17+(j+.5)*.72,.075),(.765,.71,.025),'tile',.005)
collection('02 Cream rear wall',offset=(0,-.5,.2))
box('Rear plaster wall',(.75,3.02,1.68),(9.5,.14,3.32),'plaster',.02)
box('Rear oak cap',(.75,3.02,3.36),(9.53,.18,.075),'oak3',.012)
box('Rear stainless skirting',(.75,2.932,.17),(9.35,.035,.16),'steelSoft',.007)
collection('03 Cream left wall',offset=(.5,-.2,.2))
box('Left plaster wall',(-3.93,-.58,1.68),(.14,7.26,3.32),'plaster',.02)
box('Left oak cap',(-3.93,-.64,3.36),(.18,7.14,.075),'oak3',.012)
box('Left stainless skirting',(-3.84,-.58,.17),(.03,7.16,.16),'steelSoft',.007)
# Subtle shallow wall grooves suggest large easy-clean panels.
for z in [1.4,2.1]:box('Left wall panel joint',(-3.85,-.58,z),(.003,7.0,.009),'cream',.001)

collection('04 Rear washing and preparation counter',offset=(.6,-.7,.2))
cupboard('Rear prep cabinet',-2.15,2.50,3.20,.78)
# Counter and carcass have a genuine opening for a hollow stainless basin.
box('Sanitary backsplash',(-2.15,2.91,1.23),(3.22,.025,.43),'steelSoft',.008)
cutter=box('Temporary basin aperture',(-2.88,2.46,1.00),(.60,.40,.46),'black',.055)
bpy.context.view_layer.objects.active=cutter
for md in list(cutter.modifiers):bpy.ops.object.modifier_apply(modifier=md.name)
for name in ['Rear prep cabinet brushed worktop','Rear prep cabinet oak carcass']:
 o=bpy.data.objects[name];md=o.modifiers.new('Real sink cutout','BOOLEAN');md.operation='DIFFERENCE';md.object=cutter
 bpy.context.view_layer.objects.active=o;bpy.ops.object.modifier_apply(modifier=md.name)
bpy.data.objects.remove(cutter,do_unlink=True)
levels=[(.46,.25,.83),(.50,.29,.85),(.55,.34,.93),(.58,.38,1.022),(.66,.46,1.026)]
vs=[]
for w,d,z in levels:vs.extend(rounded_points(-2.88,2.46,z,w,d,.065))
N=len(vs)//len(levels);fs=[tuple(reversed(range(N)))]+[(j*N+i,j*N+(i+1)%N,(j+1)*N+(i+1)%N,(j+1)*N+i) for j in range(len(levels)-1) for i in range(N)]
o=mesh('Hollow inset stainless sink',vs,fs,'steelSoft');md=o.modifiers.new('Stainless basin thickness','SOLIDIFY');md.thickness=.004
lathe('Sink drain',(-2.88,2.46,.835),[(0,0),(.04,0),(.04,.003),(0,.003)],'steelDark',32)
for a in range(6):rod('Drain grille',(-2.91+a*.012,2.44,.839),(-2.91+a*.012,2.48,.839),.0015,'steel',8)
curve('Attached gooseneck faucet',[(-2.88,2.76,1.02),(-2.88,2.76,1.35),(-2.88,2.65,1.45),(-2.88,2.48,1.40),(-2.88,2.48,1.33)],.021,'steelSoft')
cyl('Faucet base',(-2.88,2.76,1.04),.05,.025,'steelSoft',40)
rod('Mixer lever',(-2.77,2.76,1.03),(-2.77,2.70,1.18),.013,'steelSoft',16)
box('Soap dispenser',(-3.32,2.69,1.11),(.11,.10,.19),'ceramic',.02)
rod('Soap pump',(-3.32,2.69,1.20),(-3.32,2.64,1.25),.009,'steel',12)

collection('05 Oak ingredient shelves',offset=(.6,-.65,.3))
for z in [1.78,2.48]:
 box('Floating oak ingredient shelf',(-2.13,2.64,z),(3.10,.53,.07),'oak3',.012)
 for x in [-3.30,-.95]:
  box('Shelf bracket vertical',(x,2.88,z-.14),(.035,.025,.30),'steelInside',.004)
  rod('Shelf diagonal brace',(x,2.86,z-.27),(x,2.46,z-.035),.012,'steelInside',12)
for z,labels in [(1.82,['FLOUR','RYE','OATS','SUGAR','SEEDS','SALT']),(2.52,['BREAD','PASTRY','COCOA','SEMOLINA','SPELT','YEAST'])]:
 for i,label in enumerate(labels):jar_label('Ingredient '+label,label,-3.36+i*.49,2.61,z,.13,.31 if i%2 else .37,'paper' if i<4 else 'crustLight')

collection('06 Double electric deck oven',offset=(.4,-.9,.3))
OX,OY=.14,2.40
# Individual insulated chambers with glass doors and side control panels.
box('Oven black plinth',(OX,OY,.18),(1.18,1.05,.18),'black',.025)
box('Brushed oven back cabinet',(OX,OY+.52,1.50),(1.28,.08,2.55),'steelSoft',.025)
for x in [OX-.61,OX+.61]:box('Oven insulated side',(x,OY,1.50),(.08,1.13,2.55),'steelSoft',.022)
for z in [.27,1.35,2.34,2.75]:box('Oven insulated chamber divider',(OX,OY,z),(1.22,1.13,.10),'steelSoft',.012)
box('Oven top ventilation fascia',(OX,1.836,2.56),(1.20,.04,.31),'steelSoft',.012)
box('Oven lower fascia',(OX,1.836,.40),(1.20,.04,.22),'steelSoft',.012)
box('Oven side controls column',(OX+.51,1.84,1.47),(.19,.08,2.02),'steelSoft',.012)
for i,z in enumerate([.89,1.83]):
 for x in [OX-.58,OX+.38]:box('Oven door gasket',(x,1.809,z),(.045,.018,.65),'black',.008)
 box('Oven chamber back',(OX-.10,2.59,z),(.94,.04,.65),'steelInside',.015)
 # Thin clear front placed before dark interior with a warm horizontal light.
 box('Oven heat-resistant door glass',(OX-.10,1.794,z),(.86,.014,.52),'glass',.012)
 for zz in [z-.34,z+.34]:box('Oven brushed door rail',(OX-.10,1.777,zz),(1.06,.045,.075),'steelLight',.012)
 pull('Oven insulated handle',OX-.10,1.72,z+.25,.76)
 box('Oven digital control',(OX+.51,1.799,z+.15),(.16,.018,.18),'screen',.007)
 text('Oven temperature','220' if i else '180',(OX+.51,1.784,z+.16),.048,'digit')
 dial('Oven temperature selector',OX+.51,1.773,z-.10,.045)
 tray('Oven hearth sheet',OX-.1,2.17,z-.23,.84,.58,False)
 for x in [OX-.32,OX+.12]:loaf('Bread through oven door',x,1.99,z-.21,.33)
 box('Warm oven internal strip',(OX-.1,1.87,z+.23),(.79,.02,.018),'led',.004)
for j in range(7):box('Oven ventilation slot',(OX-.46+j*.145,1.824,2.59),(.085,.008,.015),'steelInside',.004)
text('Oven badge','NOMS / DECK 02',(OX,1.815,.42),.047,'ink')

collection('07 Glass proofing cabinet',offset=(.5,-.8,.2))
PX,PY=1.65,2.40
# Hollow cabinet; glass reveals the trays rather than a solid box behind the door.
box('Proofer back',(PX,PY+.48,1.43),(1.26,.08,2.48),'steelSoft',.02)
for x in [PX-.60,PX+.60]:box('Proofer insulated side',(x,PY,1.43),(.075,1.02,2.48),'steelSoft',.015)
for z in [.24,2.65]:box('Proofer cap',(PX,PY,z),(1.28,1.06,.10),'steelLight',.015)
for x in [PX-.48,PX+.48]:
 for y in [PY-.38,PY+.38]:cyl('Proofer adjustable foot',(x,y,.145),.045,.13,'black',24)
for zz in [.48,.85,1.22,1.59,1.96,2.33]:
 tray('Proofer dough pan',PX,PY,zz,1.08,.83)
 for x in [PX-.31,PX,PX+.31]:uv('Proofing dough portion',(x,PY-.14,zz+.085),(.115,.16,.073),'dough',24,12)
for x in [PX-.58,PX+.58]:box('Proofer door stile',(x,1.851,1.45),(.045,.045,2.31),'steelLight',.007)
box('Proofer full door glass',(PX,1.861,1.45),(1.12,.012,2.25),'glass',.002)
for z in [.29,2.60]:box('Proofer horizontal frame',(PX,1.843,z),(1.20,.045,.05),'steelLight',.008)
rod('Proofer vertical handle',(PX+.46,1.77,1.08),(PX+.46,1.77,1.73),.021,'steelInside')
for z in [1.08,1.73]:rod('Proofer handle mount',(PX+.46,1.77,z),(PX+.46,1.85,z),.018,'steelInside')
box('Proofing control panel',(PX,1.855,2.72),(.62,.03,.12),'screen',.009)
text('Proofing humidity display','28 C   /   75%',(PX,1.834,2.71),.048,'digit')

collection('08 Wheeled cooling rack',offset=(.6,-.65,.3))
RX,RY=4.65,.25
for x in [RX-.48,RX+.48]:
 for y in [RY-.39,RY+.39]:
  box('Cooling rack square upright',(x,y,1.45),(.045,.045,2.36),'steelSoft',.008)
  box('Caster fork',(x,y,.20),(.075,.055,.12),'steelDark',.012)
  o=cyl('Rack rubber caster',(x,y,.14),.071,.045,'black',24);o.rotation_euler.x=pi/2
for z in [.32,2.62]:
 for y in [RY-.39,RY+.39]:rod('Cooling rack cross rail',(RX-.48,y,z),(RX+.48,y,z),.018,'steelSoft')
for j,z in enumerate([.54,.91,1.28,1.65,2.02,2.39]):
 for x in [RX-.45,RX+.45]:rod('Attached rack tray rail',(x,RY-.4,z-.02),(x,RY+.4,z-.02),.012,'steelSoft',12)
 tray('Cooling sheet',RX,RY,z,.94,.76,False)
 if j%2:
  for x in [RX-.22,RX+.22]:loaf('Cooling oval sourdough',x,RY,z+.013,.37,pi/2)
 else:
  for x in [RX-.28,RX,RX+.28]:baguette('Cooling baguette',x,RY,z+.014,.62)

collection('09 Central oak baking island',offset=(.5,-1,.2))
IX,IY=1.87,-.46
# Open shelving, recessed doors and large work surface.
for x in [IX-1.30,IX+1.30]:
 for y in [IY-.58,IY+.58]:box('Island oak corner leg',(x,y,.58),(.10,.10,.98),'oak2',.012)
box('Island lower shelf',(IX,IY,.27),(2.67,1.23,.06),'oak2',.012)
box('Island rear panel',(IX,IY+.57,.67),(2.65,.06,.72),'oak3',.008)
box('Island center divider',(IX+.30,IY,.64),(.055,1.17,.70),'oak2',.008)
box('Island closed right cupboard',(IX+.82,IY-.59,.67),(1.0,.045,.70),'oak3',.012)
pull('Island cupboard pull',IX+.82,IY-.64,.90,.35)
for x in [IX-.80,IX-.15]:
 box('Shallow tool drawer',(x,IY-.59,.94),(.62,.06,.16),'oak3',.008)
 pull('Drawer handle',x,IY-.65,.94,.28)
rounded_slab('Thick pale oak island edge',(IX,IY,1.055),2.88,1.42,.10,.055,'oak3')
rounded_slab('Brushed steel island work surface',(IX,IY,1.113),2.87,1.41,.024,.045,'steelLight')
for k,x in enumerate([IX-.91,IX-.37]):
 bowl('Proofing banneton on lower shelf',x,IY-.13,.307,.22,'bamboo')
 for z in range(6):torus('Banneton coiled edge',(x,IY-.13,.34+z*.014),.14+z*.012,.006,'bambooDark',steps=40)


collection('10 Commercial stand mixer',offset=(.55,-.8,.25))
def mixer():
 rounded_slab('Mixer oval foot',(0,0,.035),.47,.62,.07,.11,'ivory')
 box('Mixer vertical pedestal',(0,.16,.30),(.23,.21,.52),'ivory',.065)
 pill('Mixer motor housing',(0,.015,.60),(.32,.57,.23),'ivory')
 box('Mixer stainless trim',(0,-.06,.61),(.328,.025,.065),'steelSoft',.012)
 cyl('Mixer bowl mounting disc',(0,-.10,.083),.16,.025,'steelInside',40)
 lathe('Deep hollow mixer bowl',(0,-.10,.096),[(0,0),(.10,0),(.14,.045),(.184,.15),(.195,.26),(.185,.268),(.173,.15),(.128,.05),(.09,.018),(0,.018)],'steelSoft',64)
 rod('Mixer beater shaft',(0,-.10,.51),(0,-.10,.29),.012,'steelInside')
 for a in [0,pi/2,pi,3*pi/2]:
  curve('Wire whisk in mixing bowl',[(0,-.1,.40),(.08*cos(a),-.10+.08*sin(a),.28),(.04*cos(a),-.1+.04*sin(a),.18),(0,-.1,.17)],.004,'steelSoft')
 curve('Mixer bowl handle',[(.17,-.10,.30),(.24,-.10,.32),(.25,-.10,.20),(.15,-.10,.18)],.013,'steelSoft')
 rod('Mixer speed lever',(.16,.12,.55),(.21,.05,.55),.008,'steelInside')
 uv('Mixer lever grip',(.21,.05,.55),(.018,.025,.018),'black',16,8)
local_group('Cream planetary mixer',(IX-.90,IY+.15,1.125),-.15,mixer)
curve('Mixer tucked power cable',[(IX-.9,IY+.39,1.18),(IX-1.1,IY+.51,1.14),(IX-1.31,IY+.46,1.0),(IX-1.31,IY+.35,.72)],.006,'black')

collection('11 Dough tools and recipe',offset=(.65,-.75,.25))
rounded_slab('Maple kneading board',(IX+.08,IY-.14,1.143),.92,.67,.034,.055,'oak3')
uv('Soft dough ready to shape',(IX+.03,IY-.16,1.25),(.235,.17,.095),'dough',40,20)
rod('Rolling pin barrel',(IX-.25,IY+.26,1.21),(IX+.30,IY+.26,1.21),.041,'oak3',32)
for a,b in [(-.37,-.25),(.30,.42)]:rod('Rolling pin turned handle',(IX+a,IY+.26,1.21),(IX+b,IY+.26,1.21),.018,'walnut2',16)
box('Bench scraper blade',(IX+.39,IY-.36,1.17),(.16,.10,.004),'steelSoft',.003)
rod('Bench scraper grip',(IX+.31,IY-.31,1.184),(IX+.47,IY-.31,1.184),.015,'oak2',12)
for i in range(26):
 a=i*2.4;r=.23*math.sqrt((i+.5)/26)
 uv('Small flour dusting',(IX+.03+r*cos(a),IY-.16+r*sin(a),1.163),(.007,.008,.001),'paper',8,4)
# Scale and an open handwritten recipe book rest on the back preparation counter.
rounded_slab('Digital kitchen scale',(-1.05,2.40,1.04),.43,.34,.055,.04,'steelSoft')
box('Scale black display',(-1.05,2.225,1.043),(.16,.009,.029),'screen',.004)
text('Scale grams','250 g',(-1.05,2.218,1.032),.025,'digit')
plate('Scale bowl',-1.05,2.40,1.077,.15)
rounded_slab('Recipe notebook cover',(-1.86,2.37,1.025),.48,.32,.019,.02,'sageDark')
for x in [-1.975,-1.745]:
 box('Open notebook page',(x,2.37,1.038),(.22,.29,.009),'paper',.004)
 for j in range(7):box('Handwritten recipe line',(x,2.45-j*.027,1.044),(.16 if j%2 else .12,.002,.001),'sageDark',0)

collection('12 Unbaked rolls on tray',offset=(.5,-.8,.3))
tray('Rolls prep tray',IX+.97,IY-.05,1.14,.57,.92)
for j in range(4):
 for dx in [-.13,.13]:uv('Shaped unbaked roll',(IX+.97+dx,IY-.35+j*.20,1.22),(.095,.085,.065),'dough',24,12)

collection('13 Oak retail display base',offset=(.65,-.8,.2))
DX,DY=-2.40,-2.74
box('Display recessed plinth',(DX,DY,.18),(2.32,.90,.19),'gap',.015)
box('Display oak cabinet',(DX,DY,.62),(2.50,1.08,.88),'oak',.025)
for x in [DX-1.20,DX+1.20]:box('Display inset border',(x,DY-.553,.65),(.055,.025,.77),'oak2',.007)
for z in [.28,1.00]:box('Display oak panel rail',(DX,DY-.553,z),(2.43,.025,.055),'oak2',.007)
box('Display recessed oak front',(DX,DY-.55,.64),(2.32,.015,.66),'oak3',.008)
rounded_slab('Bread case counter',(DX,DY,1.095),2.60,1.15,.07,.045,'steelLight')
text('Small bakery brand','N O M S  /  B A K E R Y',(DX,DY-.571,.70),.105,'ivory')
text('Small bakery tagline','FLOUR  /  WATER  /  TIME',(DX,DY-.572,.48),.045,'ivory')

collection('14 Sloped glass display enclosure',offset=(.65,-.8,.3))
# Clear 6mm panels with a sloping front, supported by charcoal frame and open staff side.
xl,xr=DX-1.27,DX+1.27;front,back=DY-.55,DY+.49;bottom,top=1.14,1.91
roof_front=DY-.17
for x in [xl,xr]:
 o=mesh('Display trapezoidal glass end',[(x,front,bottom),(x,back,bottom),(x,back,top),(x,roof_front,top)],[(0,1,2,3)],'glass')
 md=o.modifiers.new('6 mm safety glass','SOLIDIFY');md.thickness=.006
 curve('Display end frame',[(x,front,bottom),(x,roof_front,top),(x,back,top)],.015,'steelInside',poly=True)
 rod('Display rear upright',(x,back,bottom),(x,back,top),.014,'steelInside')
o=mesh('Angled clear front pane',[(xl,front,bottom),(xr,front,bottom),(xr,roof_front,top),(xl,roof_front,top)],[(0,1,2,3)],'glass')
md=o.modifiers.new('6 mm front safety glass','SOLIDIFY');md.thickness=.006
box('Display clear glass roof',(DX,(roof_front+back)/2,top),(2.55,back-roof_front,.008),'glass',.001)
for y,z in [(front,bottom),(roof_front,top),(back,top)]:rod('Display continuous frame',(xl,y,z),(xr,y,z),.013,'steelInside')
box('Raised rear pastry shelf',(DX,DY+.24,1.46),(2.40,.41,.025),'steelSoft',.01)
for x in [DX-1.13,DX+1.13]:rod('Display shelf standoff',(x,DY+.24,1.15),(x,DY+.24,1.45),.012,'steelSoft')

collection('15 Sourdough and baguette display',offset=(.6,-.8,.3))
tray('Bread display pan',DX-.58,DY-.13,1.15,1.13,.69)
for i,(x,y) in enumerate([(DX-.87,DY-.25),(DX-.34,DY-.25),(DX-.61,DY+.06)]):boule('Scored sourdough boule',x,y,1.174,.215)
tray('Baguette display pan',DX+.61,DY-.13,1.15,1.12,.69)
for j in range(3):baguette('Retail scored baguette',DX+.62,DY-.35+j*.22,1.174,.90,pi/2+.05)
# Small shelf-edge labels; kept clear of the bread.
for x,label in [(DX-.58,'SOURDOUGH'),(DX+.61,'BAGUETTE')]:
 box('Bread label holder',(x,DY-.54,1.17),(.43,.014,.10),'paper',.007)
 text('Bread label '+label,label,(x,DY-.550,1.158),.039,'ink')

collection('16 Croissants and cinnamon rolls',offset=(.5,-.8,.3))
for i in range(4):croissant('Laminated butter croissant',(DX-.98+i*.37,DY+.24,1.48),.31,.04*(i%2))
for i in range(2):
 x=DX+.65+i*.32;y=DY+.24
 cyl('Cinnamon bun base',(x,y,1.535),.13,.10,'crustLight',40)
 pts=[]
 for j in range(100):
  a=j/99*5*pi;r=.008+.113*j/99;pts.append((x+r*cos(a),y+r*sin(a),1.591+.004*sin(a)))
 curve('Cinnamon spiral',pts,.011,'crustDark')

collection('17 Mixing bowls and pantry storage',offset=(.6,-.7,.2))
bowl('Spare ceramic mixing bowl',-1.97,2.67,1.015,.20,'ceramic')
for i in range(3):
 bowl('Stacked steel prep bowl',-1.48,2.67,1.015+i*.034,.14,'steelSoft')
# A back-counter utensil crock with tools securely inside it.
lathe('Utensil crock',(-3.54,2.41,1.015),[(0,0),(.085,0),(.09,.19),(.079,.19),(.071,.02),(0,.02)],'ceramic',40)
for i in range(3):
 x=-3.58+i*.035
 rod('Wooden spoon handle',(x,2.41,1.06),(x,2.41,1.40+i*.025),.009,'oak2',12)
 uv('Wooden spoon bowl',(x,2.41,1.425+i*.025),(.027,.012,.04),'oak3',20,12)
# Ingredient tubs below the island are deliberately closed and clean.
for x in [IX-.94,IX-.34]:
 box('Stored flour tub',(x,IY+.32,.48),(.42,.36,.29),'ivory',.035)
 box('Flour tub lid',(x,IY+.32,.634),(.44,.38,.025),'steelSoft',.012)

exec(compile((HERE/'retail_and_equipment.py').read_text(),str(HERE/'retail_and_equipment.py'),'exec'))

scene=bpy.context.scene
scene['concept_reference']='previews/approved-concepts.png — concept 10 Modern Baking Lab'
scene['no_ceiling_pendant']=True
scene['staff_aisle_m']=.80
scene['display_to_island_clearance_m']=1.53
scene['front_service_aisle_m']=1.0
scene['revision_notes']='Central workbench and its contents shifted 0.85 m right for visibility beyond the storefront counter'
scene['original_model']='Modern baking lab; separate from the existing artisan bakery'
finish('modern-baking-lab','Noms — Modern Baking Lab',13.8,(.65,-.55,1.25))
