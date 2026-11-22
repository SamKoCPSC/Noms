"""Concept 04: U-shaped industrial loft kitchen. Run Blender -b --python this_file.py."""
from pathlib import Path
HERE=Path(__file__).resolve().parent
exec(compile((HERE/'diorama_common.py').read_text(),str(HERE/'diorama_common.py'),'exec'))
code=(HERE/'scene_details.py').read_text().replace('o.location=(10,-14,10)','o.location=(-10,-14,10)').replace('scene.render.threads=2','scene.render.threads=6').replace('scene.cycles.samples=64','scene.cycles.samples=40').replace("scene['revision']=1","scene['revision']=5")
exec(compile(code,str(HERE/'scene_details.py'),'exec'))
for key,h in [('grout','#B4A38E'),('concrete','#B9B5A9'),('cabinet','#383E3E'),('door','#454A48'),('panel','#3E4442'),('steelSoft','#A5AAA6'),('copper','#AE7147'),('window','#D9E8E3'),('screen','#E8E2D2'),('leafDark','#34563C')]:M[key]=material(key,h,.8,.5 if key in ['steelSoft','copper'] else 0)
for i,h in enumerate(['#995B43','#A1664A','#AC7053','#8E5644','#AE7960','#9D654F']):M['brick'+str(i)]=material('Brick variation '+str(i),h,.93)
# Individual softened masonry with subtle procedural surface relief.
for i in range(6):
 m=M['brick'+str(i)];n=m.node_tree.nodes;l=m.node_tree.links;t=n.new('ShaderNodeTexNoise');t.inputs['Scale'].default_value=65;b=n.new('ShaderNodeBump');b.inputs['Strength'].default_value=.18;b.inputs['Distance'].default_value=.008;l.new(t.outputs['Fac'],b.inputs['Height']);l.new(b.outputs[0],n.get('Principled BSDF').inputs['Normal'])
collection('01 Foundation and concrete floor',offset=(0,0,0))
rounded_slab('Oak-edged miniature foundation',(0,0,-.12),6.2,5.9,.25,.10,'oak2')
rounded_slab('Continuous concrete floor',(0,0,.016),6.04,5.74,.045,.06,'concrete')
for x in [-1.5,0,1.5]:box('Concrete expansion joint',(x,0,.041),(.008,5.6,.002),'rugline',0)
for y in [-1.4,0,1.4]:box('Concrete expansion joint',(0,y,.041),(5.95,.008,.002),'rugline',0)
collection('02 Rear brick wall',offset=(0,-.6,.2))
box('Back mortar wall',(0,2.8,1.78),(6.12,.16,3.48),'grout',.01)
for row in range(19):
 z=.15+row*.18
 for j in range(14):
  a=-3.0+j*.46-(row%2)*.23;b=min(a+.44,3.0);a=max(a,-3.0)
  if b>a:box('Back staggered brick',((a+b)/2,2.699,z),(b-a,.065,.156),'brick'+str(random.randrange(6)),.009)
box('Back wall top coping',(0,2.8,3.54),(6.15,.20,.07),'cream',.012)
collection('03 Right window wall',offset=(-.6,0,.2))
# Real window opening: y=-.55..2.08, z=1.46..3.16.
ya,yb,za,zb=-.55,2.08,1.46,3.16
for cy,dy,cz,dz in [(0,5.6,za/2,za),(0,5.6,(3.5+zb)/2,3.5-zb),((-2.8+ya)/2,ya+2.8,(za+zb)/2,zb-za),((yb+2.8)/2,2.8-yb,(za+zb)/2,zb-za)]:box('Window wall mortar',(3.0,cy,cz),(.16,dy,dz),'grout',.005)
for row in range(19):
 z=.15+row*.18
 for j in range(13):
  a=-2.8+j*.46-(row%2)*.23;b=min(a+.44,2.8);a=max(a,-2.8)
  if za-.08<z<zb+.08:
   if a<yb and b>ya:
    if a<ya:b=ya
    elif b>yb:a=yb
    else:continue
  if b>a:box('Side staggered brick',(2.899,(a+b)/2,z),(.065,b-a,.156),'brick'+str(random.randrange(6)),.009)
box('Window daylight panes',(3.015,(ya+yb)/2,(za+zb)/2),(.012,yb-ya,zb-za),'window',.001)
for y in [ya,ya+(yb-ya)/3,ya+2*(yb-ya)/3,yb]:box('Steel window mullion',(2.87,y,(za+zb)/2),(.10,.035,zb-za+.08),'black',.006)
for z in [za,(za+zb)/2,zb]:box('Steel window crossbar',(2.87,(ya+yb)/2,z),(.10,yb-ya+.08,.04),'black',.006)
box('Oak window sill',(2.81,(ya+yb)/2,za),(.32,yb-ya+.20,.07),'oak3',.012)
box('Side wall top coping',(3,-.05,3.54),(.20,5.50,.07),'cream',.012)
# Cabinet fronts use a recessed inner panel and independent frame rails.
def front(n,x,y,z,w,h,draw=False):
 box(n+' recessed field',(x,y+.014,z),(w-.07,.033,h-.07),'panel',.006)
 for xx in [x-w/2+.025,x+w/2-.025]:box(n+' stile',(xx,y,z),(.05,.055,h),'door',.006)
 for zz in [z-h/2+.025,z+h/2-.025]:box(n+' rail',(x,y,zz),(w-.08,.055,.05),'door',.006)
 if draw:rod(n+' brass pull',(x-w*.20,y-.05,z),(x+w*.20,y-.05,z),.012,'brass')
 else:rod(n+' brass pull',(x+.10,y-.05,z+h*.30),(x+w*.30,y-.05,z+h*.30),.012,'brass')
def cabinet(n,x,y,w):
 box(n+' carcass',(x,y,.56),(w,.69,.86),'cabinet',.01)
 box(n+' recessed plinth',(x,y+.035,.12),(w-.04,.57,.15),'black',.005)
 front(n+' drawer',x,y-.36,.87,w-.035,.19,True)
 front(n+' door',x,y-.36,.47,w-.035,.57)
collection('04 Back cabinetry and worktop',offset=(0,-.8,.15))
for x,w in [(-1.18,.75),(.60,.80),(1.46,.85),(2.33,.88)]:cabinet('Back kitchen unit',x,2.31,w)
# Continuous stone countertop except range opening x=-.76..+.17.
box('Left stone countertop',(-1.18,2.30,1.02),(.83,.80,.065),'stone',.012)
box('Right stone countertop',(1.51,2.30,1.02),(2.66,.80,.065),'stone',.012)
box('Counter backsplash',(1.49,2.66,1.13),(2.70,.035,.17),'stone',.006)
collection('05 Window sink return',offset=(-.7,-.35,.15))
for y in [-.47,.30,1.07,1.84]:local_group('Return cabinet',(2.48,y,0),-pi/2,lambda:cabinet('Window return',0,0,.76))
# Counter pieces around a genuine inset sink, avoiding an opaque slab across bowl.
for y,dy in [(-.505,.89),(1.45,.90)]:box('Return stone worktop',(2.48,y,1.02),(.80,dy,.065),'stone',.01)
for x in [2.125,2.835]:box('Sink surround stone',(x,.47,1.02),(.09,1.06,.065),'stone',.008)
box('Sink basin floor',(2.48,.47,.825),(.55,.99,.035),'steelSoft',.025)
for x in [2.20,2.76]:box('Sink basin side',(x,.47,.922),(.035,1.05,.20),'steelSoft',.015)
for y in [-.045,.985]:box('Sink basin end',(2.48,y,.922),(.58,.035,.20),'steelSoft',.015)
cyl('Sink drain',(2.48,.47,.847),.058,.006,'steelDark',32)
for x in [2.17,2.79]:box('Sink rim',(x,.47,1.052),(.045,1.12,.012),'steel',.006)
for y in [-.07,1.01]:box('Sink rim',(2.48,y,1.052),(.65,.045,.012),'steel',.006)
# Deck-mount on the short end of the sink, projecting into the basin. The
# complete faucet stays in front of the sill's inner edge at x=2.65.
faucet_objects=[]
faucet_objects.append(cyl('Faucet deck mounting collar',(2.48,1.09,1.064),.043,.023,'black',40))
faucet_objects.append(curve('Black gooseneck faucet',[(2.48,1.09,1.07),(2.48,1.09,1.45),(2.48,1.01,1.54),(2.48,.79,1.52),(2.48,.74,1.42)],.023,'black'))
faucet_objects.append(cyl('Faucet mixer base',(2.59,1.10,1.067),.025,.026,'black',32))
faucet_objects.append(rod('Faucet mixer lever',(2.59,1.10,1.08),(2.59,1.10,1.22),.014,'black'))
bpy.context.view_layer.update()
dg=bpy.context.evaluated_depsgraph_get();faucet_x=[]
for faucet in faucet_objects:
 ev=faucet.evaluated_get(dg);me=ev.to_mesh()
 faucet_x.extend((ev.matrix_world@v.co).x for v in me.vertices)
 ev.to_mesh_clear()
sill=bpy.data.objects['Oak window sill'];sill_min_x=min((sill.matrix_world@Vector(v)).x for v in sill.bound_box)
clearance=sill_min_x-max(faucet_x)
assert clearance>.03, 'Faucet enters window sill clearance'
bpy.context.scene['faucet_sill_clearance_m']=clearance
bpy.context.scene['faucet_clearance_checked']=True
collection('06 Stainless refrigerator',offset=(0,-.8,.1))
box('Fridge cabinet',(-2.30,2.28,1.17),(.94,.88,2.20),'cabinet',.045)
for z,h in [(1.48,1.42),(.40,.65)]:box('Separate brushed steel fridge door',(-2.30,1.806,z),(.91,.10,h),'steelSoft',.025)
for z,h in [(1.43,.64),(.42,.35)]:rod('Fridge vertical handle',(-1.94,1.705,z-h/2),(-1.94,1.705,z+h/2),.018,'steel')
box('Fridge top cupboard',(-2.30,2.31,2.57),(1.0,.74,.53),'cabinet',.012)
front('Fridge top cupboard',-2.30,1.92,2.57,.94,.48)
collection('07 Range and extractor',offset=(0,-.85,.2))
box('Range body',(-.30,2.28,.53),(.92,.77,.96),'steelSoft',.025)
box('Oven dark glass',(-.30,1.88,.48),(.77,.035,.53),'black',.025)
box('Oven inner window',(-.30,1.857,.48),(.63,.009,.36),'charcoal',.025)
rod('Oven door handle',(-.66,1.80,.79),(.06,1.80,.79),.023,'steel')
for x in [-.62,-.40,-.18,.04]:
 o=cyl('Range control knob',(x,1.864,.94),.041,.032,'black',32);o.rotation_euler.x=pi/2
 box('Knob indicator',(x,1.843,.958),(.005,.004,.018),'ivory',.001)
box('Black enamel hob',(-.30,2.28,1.035),(.90,.73,.035),'black',.012)
for x in [-.52,-.08]:
 for y in [2.10,2.46]:
  cyl('Gas burner',(x,y,1.068),.09,.027,'steelDark',32)
  for a in [0,pi/2]:rod('Cast iron grate',(x-.15*cos(a),y-.15*sin(a),1.10),(x+.15*cos(a),y+.15*sin(a),1.10),.012,'black')
# Pyramid-frustum range hood.
v=[(-.87,1.92,2.14),(.27,1.92,2.14),(.27,2.70,2.14),(-.87,2.70,2.14),(-.50,2.30,2.63),(-.10,2.30,2.63),(-.10,2.70,2.63),(-.50,2.70,2.63)]
bevel(mesh('Tapered stainless extractor',v,[(0,1,2,3),(0,4,5,1),(1,5,6,2),(2,6,7,3),(3,7,4,0),(4,7,6,5)],'steelSoft'),.009,3)
box('Extractor bottom lip',(-.30,2.31,2.13),(1.16,.82,.055),'steel',.012)
box('Extractor chimney',(-.30,2.49,3.03),(.40,.38,.82),'steelSoft',.012)
for i in range(10):box('Extractor underside vent',(-.73+i*.094,2.28,2.098),(.045,.46,.006),'black',.001)
collection('08 Front dining peninsula',offset=(0,-.8,.1))
# U return joined to right counter; opening left is 1.4m, interior aisle 2.4m.
box('Peninsula graphite carcass',(.68,-1.05,.54),(3.85,.63,.90),'cabinet',.015)
box('Peninsula recessed plinth',(.68,-1.03,.12),(3.75,.51,.14),'black',.005)
for x in [-.84,-.08,.68,1.44,2.20]:
 box('Oak dining side panel',(x,-1.38,.59),(.744,.045,.77),'oak',.008)
 for dx in [-.32,.32]:box('Panel slim inset edging',(x+dx,-1.410,.59),(.018,.012,.70),'oak2',.003)
rounded_slab('Generous oak dining countertop',(.65,-1.26,1.055),4.05,1.12,.095,.055,'oak3')
for x in [-1.25,2.55]:box('Peninsula black corner frame',(x,-1.70,.52),(.04,.045,.99),'black',.004)
collection('09 Three oak and steel stools',offset=(0,-1.0,.1))
for x in [-.78,.55,1.88]:
 y=-2.18
 cyl('Rounded oak stool seat',(x,y,.76),.245,.075,'oak3',64)
 for a in [pi/4,3*pi/4,5*pi/4,7*pi/4]:rod('Splayed black stool leg',(x+.21*cos(a),y+.21*sin(a),.08),(x+.14*cos(a),y+.14*sin(a),.73),.019,'black')
 torus('Stool circular footrest',(x,y,.32),.197,.014,'black')
collection('10 Floating oak shelves and kitchenware',offset=(0,-.65,.15))
for z in [2.05,2.68]:
 box('Oak floating shelf',(1.35,2.51,z),(1.88,.37,.065),'oak3',.013)
 for x in [.60,2.10]:
  rod('Shelf steel support',(x,2.68,z-.30),(x,2.68,z),.012,'black');rod('Shelf steel brace',(x,2.68,z-.28),(x,2.35,z-.04),.012,'black')
for i in range(4):jar('Pantry jar',.60+i*.27,2.50,2.716,contents='honey',r=.087,h=.23+(i%2)*.06)
for x in [.70,1.05]:
 for i in range(4):plate('Stacked dinner plates',x,2.49,2.09+i*.025,.13)
for x in [1.45,1.78,2.02]:bowl('Serving bowl',x,2.49,2.09,.105)
# Copper pans hung under shelves.
rod('Utensil rail',(.62,2.61,1.79),(2.15,2.61,1.79),.013,'black')
for x in [.80,1.24]:
 torus('Copper pan hanging eye',(x,2.57,1.80),.022,.006,'black',(pi/2,0,0),32)
 rod('Copper pan handle',(x,2.55,1.77),(x,2.55,1.59),.018,'walnut')
 o=lathe('Hanging copper pan',(0,0,0),[(0,0),(.13,0),(.15,.04),(.145,.047),(.125,.012),(0,.012)],'copper',48);o.location=(x,2.55,1.46);o.rotation_euler.x=pi/2
collection('11 Stove cooking pot and counter details',offset=(0,-.7,.30))
x,y=-.52,2.10
lathe('Orange enamel cooking pot',(x,y,1.11),[(0,0),(.15,0),(.17,.04),(.17,.20),(.155,.20),(.15,.03),(0,.03)],'rust',56)
cyl('Pot lid',(x,y,1.32),.174,.025,'rust',48);uv('Lid knob',(x,y,1.355),(.036,.036,.022),'black')
for dx in [-.215,.215]:rod('Pot handle',(x+dx-.035,y,1.24),(x+dx+.035,y,1.24),.018,'black')
for x in [-1.42,-1.10]:
 lathe('Olive oil bottle',(x,2.50,1.055),[(0,0),(.055,0),(.055,.22),(.025,.26),(.025,.34),(0,.34)],'sageDark',32)
 cyl('Bottle cap',(x,2.50,1.40),.026,.035,'brass',24)
box('Oak cutting board on counter',(2.48,1.35,1.067),(.53,.33,.025),'oak',.012)
for i in range(4):uv('Tomatoes on counter',(2.36+i*.08,1.35,1.13),(.045,.044,.043),'tomato',20,12)
lathe('Utensil crock',(.45,2.48,1.056),[(0,0),(.09,0),(.095,.20),(.08,.20),(.075,.025),(0,.025)],'ceramic',48)
for i in range(4):
 xx=.40+i*.035;rod('Wood utensil handle',(xx,2.48,1.10),(xx-.045+i*.03,2.48,1.46),.009,'oak2');uv('Wood spoon bowl',(xx-.045+i*.03,2.48,1.49),(.026,.010,.042),'oak3',16,10)
pot('Window basil',2.69,1.72,1.057,.11)
collection('12 Dining place settings',offset=(0,-.7,.25))
for x in [-.70,1.85]:
 plate('Dining stoneware plate',x,-1.37,1.11,.19)
 mug('Coffee cup',x+.27,-1.18,1.11,.052,.11)
 for dx in [-.24,.24]:rod('Dining cutlery',(x+dx,-1.53,1.125),(x+dx,-1.27,1.125),.008,'steel')
# Central fresh salad in serving bowl.
bowl('Salad bowl',.55,-1.15,1.11,.25,'darkPlate')
for i in range(18):
 a=i*2.4;r=.15*math.sqrt((i+1)/18);uv('Salad greens',(.55+r*cos(a),-1.15+r*sin(a),1.245+random.random()*.035),(.055,.038,.018),'basil' if i%2 else 'cucumber',16,8)
for i in range(5):uv('Salad cherry tomatoes',(.55+.13*cos(i*1.25),-1.15+.13*sin(i*1.25),1.28),(.027,.027,.025),'tomato',16,10)
collection('13 Handwritten recipe notebook',offset=(0,-.8,.25))
box('Handwritten notebook pages',(.0,-1.38,1.125),(.35,.27,.018),'paper',.003)
box('Notebook spine',(.0,-1.38,1.135),(.012,.27,.01),'oak2',.002)
for x in [-.105,.065]:
 for j in range(6):box('Notebook pencil recipe line',(x,-1.47+j*.032,1.138),(.105+(j%2)*.025,.003,.001),'rugline',0)
rod('Recipe pencil',(.19,-1.52,1.14),(.23,-1.26,1.14),.006,'oak2',10)
collection('14 Shared food on dining counter',offset=(0,-.65,.3))
# Tomato pasta on the left place setting, all geometry seated above the plate.
for i in range(16):
 a=i*2.4;r=.115*math.sqrt((i+1)/16)
 x=-.70+r*cos(a);y=-1.37+r*sin(a);z=1.155+.012*(i%3)
 curve('Pasta curled ribbon',[(x-.020,y-.012,z),(x,y+.015,z+.008),(x+.022,y-.009,z)],.010,'pasta')
for i in range(6):
 a=i*1.05;uv('Pasta tomato sauce',(-.70+.075*cos(a),-1.37+.075*sin(a),1.19),(.030,.023,.013),'tomato',16,8)
for dx,dy in [(-.035,.02),(.035,-.015)]:
 o=uv('Pasta basil garnish',(-.70+dx,-1.37+dy,1.209),(.035,.016,.006),'basil',16,8);o.rotation_euler.z=dx*20
# Salmon and asparagus on the right plate.
rounded_slab('Grilled salmon portion',(1.85,-1.40,1.175),.20,.105,.055,.025,'salmon')
for i in range(5):
 curve('Salmon pale flaky seam',[(1.78+i*.03,-1.445,1.204),(1.79+i*.03,-1.40,1.207),(1.78+i*.03,-1.36,1.204)],.0025,'salmonFat')
for i in range(4):
 y=-1.29+i*.016
 rod('Asparagus spear',(1.75,y,1.153),(1.93,y,1.153),.007,'basil',12)
 uv('Asparagus tip',(1.94,y,1.153),(.023,.010,.009),'leafDark',16,8)
o=cyl('Salmon lemon round',(1.93,-1.42,1.208),.032,.008,'lemon',32)
cyl('Lemon pale center',(1.93,-1.42,1.213),.026,.002,'dough',32)
# Sliced garlic bread on a small oak serving board behind the recipe notebook.
rounded_slab('Garlic bread serving board',(-.08,-.91,1.121),.60,.29,.025,.035,'oak2')
for i in range(4):
 x=-.28+i*.12
 o=uv('Toasted bread slice crust',(x,-.91,1.165),(.065,.093,.038),'crust',24,12)
 uv('Golden garlic bread crumb',(x,-.91,1.192),(.054,.080,.012),'dough',24,12)
 for j in range(4):
  uv('Garlic bread herb fleck',(x+.025*cos(j*2.4),-.91+.045*sin(j*2.4),1.204),(.006,.010,.002),'basil',12,6)
# Colorful shared roast vegetables between the salad and the right place setting.
plate('Roasted vegetable serving platter',1.23,-1.43,1.111,.225,'ceramic',1.2,.72)
for i in range(12):
 a=i*2.4;r=.16*math.sqrt((i+1)/12)
 x=1.23+r*cos(a);y=-1.43+.55*r*sin(a)
 if i%3==0:
  o=cyl('Roasted zucchini round',(x,y,1.164),.035,.024,'cucumber',24);cyl('Zucchini cut face',(x,y,1.178),.029,.003,'avocado',24)
 elif i%3==1:
  o=box('Roasted sweet potato',(x,y,1.16),(.057,.039,.035),'orange',.01);o.rotation_euler.z=a
 else:
  uv('Roasted cherry tomato',(x,y,1.166),(.032,.028,.026),'tomato',20,10)
exec(compile((HERE/'counter_appliances.py').read_text(),str(HERE/'counter_appliances.py'),'exec'))
finish('brick-loft-kitchen','Noms Brick Loft — U-shaped Home Kitchen',scale=9.2,target=(0,0,1.5))
