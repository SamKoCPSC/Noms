"""Revision 2: bakery production equipment and expanded retail frontage."""
collection('18 Reversible dough sheeter',offset=(.6,-.8,.3))
SX,SY=-2.12,.65
# Cabinet-supported reversible sheeter, with two extended fabric conveyor tables.
box('Sheeter motor cabinet',(SX,SY,.61),(.58,.66,.93),'ivory',.04)
for x in [SX-.23,SX+.23]:
 for y in [SY-.25,SY+.25]:
  box('Sheeter caster fork',(x,y,.15),(.07,.06,.09),'steelSoft',.01)
  o=cyl('Sheeter caster wheel',(x,y,.11),.052,.035,'black',24);o.rotation_euler.x=pi/2
box('Sheeter stainless base rail',(SX,SY,.22),(.66,.70,.08),'steelSoft',.015)
for dx in [-.77,.77]:
 rounded_slab('Sheeter conveyor frame',(SX+dx,SY,1.05),1.04,.72,.055,.045,'steelSoft')
 rounded_slab('Sheeter cream woven conveyor',(SX+dx,SY,1.084),.98,.62,.016,.035,'linen')
 for y in [SY-.34,SY+.34]:rod('Sheeter edge rail',(SX+dx-.48,y,1.05),(SX+dx+.48,y,1.05),.015,'steelInside')
 for x in [SX+dx-.44,SX+dx+.44]:rod('Conveyor end roller',(x,SY-.30,1.07),(x,SY+.30,1.07),.025,'steelSoft')
 # Braces attach the long tables to the motor cabinet.
 rod('Sheeter folding table support',(SX+( .23 if dx>0 else -.23),SY,.65),(SX+dx,SY,1.02),.016,'steelInside')
box('Sheeter roller head',(SX,SY,1.28),(.44,.78,.44),'ivory',.055)
for x in [SX-.235,SX+.235]:
 rod('Polished dough compression roller',(x,SY-.29,1.15),(x,SY+.29,1.15),.065,'steelSoft',40)
 # Guard bars are attached around both feeding mouths.
 for j in range(6):rod('Sheeter safety grille',(x,SY-.28+j*.112,1.17),(x+(.15 if x>SX else -.15),SY-.28+j*.112,1.27),.007,'steelInside',10)
 rod('Sheeter guard crossbar',(x+(.15 if x>SX else -.15),SY-.31,1.27),(x+(.15 if x>SX else -.15),SY+.31,1.27),.009,'steelInside',12)
box('Sheeter front control label',(SX,SY-.397,1.38),(.31,.01,.10),'steelSoft',.007)
text('Sheeter model','LAMINATE  /  500',(SX,SY-.407,1.38),.028,'ink')
# Visible adjustment wheel, scale pointer and emergency stop.
wheel=(SX+.12,SY-.43,1.19)
torus('Sheeter thickness handwheel',wheel,.095,.011,'steelInside',(pi/2,0,0),40)
for a in [0,2*pi/3,4*pi/3]:rod('Handwheel spoke',wheel,(wheel[0]+.084*cos(a),wheel[1],wheel[2]+.084*sin(a)),.006,'steelSoft',12)
o=cyl('Emergency stop collar',(SX-.12,SY-.435,1.25),.041,.025,'honey',32);o.rotation_euler.x=pi/2
o=cyl('Red emergency stop',(SX-.12,SY-.453,1.25),.026,.03,'red',32);o.rotation_euler.x=pi/2
pill('Laminated dough emerging on belt',(SX-.78,SY,1.108),(.65,.43,.026),'dough')
rod('Sheeter foot pedal stem',(SX,SY-.18,.3),(SX,SY-.55,.12),.018,'steelInside')
box('Sheeter foot control',(SX,SY-.55,.12),(.22,.15,.045),'black',.012)
curve('Sheeter tucked cable',[(SX,SY+.30,.50),(SX-.2,SY+.43,.15),(SX-.55,SY+.47,.11),(SX-.55,1.96,.15)],.009,'black')

collection('19 Stainless ingredient refrigerator',offset=(.5,-.8,.2))
FX,FY=3.08,2.40
box('Ingredient fridge insulated cabinet',(FX,FY,1.44),(1.32,1.11,2.50),'steelSoft',.035)
for x in [FX-.49,FX+.49]:
 for y in [FY-.36,FY+.36]:cyl('Ingredient fridge adjustable foot',(x,y,.14),.045,.14,'black',24)
box('Ingredient fridge door gasket',(FX,1.832,1.52),(1.22,.03,2.20),'black',.016)
box('Ingredient fridge upper door',(FX,1.80,1.91),(1.18,.065,1.37),'steelLight',.025)
box('Ingredient fridge lower freezer door',(FX,1.80,.79),(1.18,.065,.78),'steelLight',.025)
for a,b in [(1.42,2.12),(.57,.98)]:
 rod('Fridge attached vertical pull',(FX-.42,1.71,a),(FX-.42,1.71,b),.022,'steelInside')
 for z in [a,b]:rod('Fridge handle bracket',(FX-.42,1.71,z),(FX-.42,1.80,z),.015,'steelInside')
box('Fridge control screen',(FX,1.815,2.66),(.45,.025,.09),'screen',.007)
text('Fridge temperature','3 C  /  -18 C',(FX,1.797,2.64),.036,'digit')
for j in range(8):box('Fridge base ventilation',(FX-.46+j*.13,1.833,.28),(.075,.012,.018),'steelInside',.003)
# Small dated kitchen labels and a temperature log give it a working bakery feel.
for x,z,label in [(FX-.18,2.18,'BUTTER'),(FX+.18,2.18,'CREAM'),(FX,1.88,'COLD STORAGE')]:
 box('Refrigerator ingredient label',(x,1.763,z),(.28,.003,.095),'paper',.004)
 text('Fridge label '+label,label,(x,1.759,z-.008),.022,'ink')

collection('20 Glass cake and drinks refrigerator',offset=(.5,-.8,.2))
GX,GY=4.58,2.40
for x in [GX-.61,GX+.61]:box('Retail fridge insulated side',(x,GY,1.43),(.08,1.07,2.46),'ivory',.018)
box('Retail fridge back',(GX,GY+.49,1.43),(1.26,.08,2.46),'ivory',.015)
for z in [.26,2.65]:box('Retail fridge cap',(GX,GY,z),(1.30,1.10,.12),'ivory',.02)
for x in [GX-.49,GX+.49]:
 for y in [GY-.35,GY+.35]:cyl('Retail fridge adjustable foot',(x,y,.14),.045,.14,'black',24)
for z in [.53,1.02,1.51,2.0]:
 box('Chilled cake glass shelf',(GX,GY-.02,z),(1.09,.89,.012),'glass',.002)
 for x in [GX-.54,GX+.54]:rod('Cake shelf support',(x,GY-.40,z-.02),(x,GY+.36,z-.02),.008,'steelSoft',12)
for x in [GX-.58,GX+.58]:box('Retail fridge vertical frame',(x,1.842,1.45),(.045,.05,2.24),'steelSoft',.008)
for z in [.33,2.56]:box('Retail fridge door rail',(GX,1.842,z),(1.20,.05,.05),'steelSoft',.008)
box('Retail fridge clear door',(GX,1.851,1.45),(1.12,.009,2.18),'glass',.002)
rod('Retail fridge pull',(GX+.46,1.76,1.11),(GX+.46,1.76,1.72),.018,'steelInside')
for z in [1.11,1.72]:rod('Retail fridge pull bracket',(GX+.46,1.76,z),(GX+.46,1.85,z),.014,'steelInside')
box('Cake fridge branding strip',(GX,1.832,2.66),(.89,.025,.11),'sageDark',.008)
text('Cake fridge label','CAKES  /  COLD DRINKS',(GX,1.813,2.64),.042,'paper')
for x in [GX-.51,GX+.51]:box('Refrigerator interior light',(x,1.91,1.48),(.009,.018,2.0),'led',.003)

def layer_cake(n,x,y,z,r=.21):
 plate(n+' serving board',x,y,z,r*1.17)
 cyl(n+' golden sponge',(x,y,z+.12),r,.19,'crustLight',48)
 for zz in [z+.08,z+.15,z+.223]:cyl(n+' cream layer',(x,y,zz),r+.003,.021,'paper',48)
 for j in range(7):
  a=j*2*pi/7
  uv(n+' strawberry',(x+r*.72*cos(a),y+r*.72*sin(a),z+.252),(.027,.025,.035),'red',16,10)
  uv(n+' berry leaf',(x+r*.72*cos(a),y+r*.72*sin(a),z+.28),(.025,.015,.003),'leaf',12,6)
for z in [1.04,1.53,2.02]:
 for dx in [-.28,.28]:layer_cake('Chilled strawberry cake',GX+dx,GY-.12,z,.19)
for j in range(6):
 x=GX-.43+j*.17
 lathe('Cold milk bottle',(x,GY-.17,.545),[(0,0),(.047,0),(.05,.035),(.048,.18),(.029,.23),(.027,.26),(0,.26)],'ivory',32)
 cyl('Milk bottle cap',(x,GY-.17,.81),.031,.02,'sageDark',24)

collection('21 Storefront bread wall and bakery sign',offset=(.5,-.6,.3))
def bread_wall():
 # Local front points toward +X after rotation onto the left wall.
 board('Bakery wall brand','NOMS  /  BAKEHOUSE',0,-.02,3.08,2.88,.32,'sageDark',.145)
 for z in [2.05,2.60]:
  box('Retail wall oak shelf',(0,-.25,z),(2.82,.48,.06),'oak3',.01)
  for x in [-1.15,1.15]:
   rod('Bread shelf angled bracket',(x,-.03,z-.22),(x,-.43,z-.04),.012,'steelInside',12)
  rod('Bread shelf retaining rail',(-1.37,-.48,z+.11),(1.37,-.48,z+.11),.009,'steelInside',12)
  for x in [-1.37,1.37]:rod('Bread shelf rail attachment',(x,-.48,z+.03),(x,-.48,z+.11),.009,'steelInside',12)
  for i in range(6):
   x=-1.16+i*.46
   if z>2.3:loaf('Wall sourdough loaf',x,-.24,z+.033,.38,.1)
   else:baguette('Wall artisan baguette',x,-.24,z+.033,.39,pi/2)
 for x,label in [(-.85,'SOURDOUGH'),(.85,'BAKED TODAY')]:
  box('Wall shelf bread ticket',(x,-.505,2.08),(.62,.01,.12),'paper',.005)
  text('Wall shelf ticket '+label,label,(x,-.513,2.064),.048,'ink')
local_group('Left wall bakery display',(-3.82,-1.93,0),pi/2,bread_wall)

collection('22 Checkout and takeaway counter',offset=(.5,-.8,.2))
CX,CY=-.35,-2.74
box('Checkout oak body',(CX,CY,.62),(1.60,1.08,.88),'oak3',.02)
box('Checkout recessed foot',(CX,CY,.18),(1.49,.96,.18),'gap',.01)
rounded_slab('Checkout pale countertop',(CX,CY,1.095),1.65,1.15,.07,.04,'stone')
# POS terminal with a tilted screen, attached pole and contactless reader.
rounded_slab('Till base',(CX+.28,CY+.20,1.145),.30,.27,.04,.035,'steelInside')
rod('Till fixed neck',(CX+.28,CY+.20,1.16),(CX+.28,CY+.24,1.36),.025,'steelInside')
o=box('POS tablet body',(CX+.28,CY+.23,1.47),(.39,.045,.27),'black',.025);o.rotation_euler.x=-.20
o=box('POS luminous screen',(CX+.28,CY+.201,1.47),(.35,.006,.23),'screen',.008);o.rotation_euler.x=-.20
text('Till screen text','NOMS\nBAKERY',(CX+.28,CY+.186,1.49),.043,'paper')
box('Card reader',(CX+.38,CY-.30,1.17),(.14,.21,.065),'steelInside',.016)
box('Card reader screen',(CX+.38,CY-.30,1.207),(.095,.12,.003),'digit',.005)
# Paper takeaway bags with folded gussets and attached twisted handles.
for i in range(3):
 x=CX-.47+i*.13;y=CY+.15+i*.025
 box('Kraft bakery bag',(x,y,1.30),(.23,.10,.33),'crustLight',.008)
 for yy in [y-.045,y+.045]:curve('Attached paper bag handle',[(x+.065,yy,1.46),(x+.065,yy,1.56),(x-.065,yy,1.56),(x-.065,yy,1.46)],.004,'walnut2')
 text('Bakery bag print','NOMS',(x,y-.057,1.31),.038,'paper')
# Tongs and flat paper bags sit on a dedicated service mat.
rounded_slab('Service mat',(CX-.36,CY-.31,1.137),.46,.30,.008,.02,'sageDark')
for i in range(3):box('Flat takeaway paper bag',(CX-.39,CY-.30,1.15+i*.004),(.29,.23,.003),'paper',.002)
curve('Serving tongs',[(CX-.17,CY-.42,1.16),(CX-.11,CY-.20,1.16),(CX-.05,CY-.42,1.16)],.009,'steelSoft')

collection('23 Refrigerated pastry counter',offset=(.5,-.8,.2))
TX,TY=1.52,-2.74
box('Patisserie refrigerated base',(TX,TY,.62),(2.12,1.08,.88),'oak3',.025)
box('Patisserie base kick',(TX,TY,.18),(2.0,.97,.18),'steelInside',.015)
for i in range(12):box('Patisserie front air vent',(TX-.82+i*.15,TY-.547,.35),(.10,.01,.015),'steelInside',.003)
rounded_slab('Chilled display metal worktop',(TX,TY,1.095),2.17,1.15,.07,.04,'steelLight')
for z in [1.16,1.52]:
 rounded_slab('Patisserie display shelf',(TX,TY,z),2.01,.91,.023,.025,'steelSoft')
 for x in [TX-.96,TX+.96]:rod('Chilled case shelf support',(x,TY+.40,1.12),(x,TY+.40,z),.014,'steelInside')
for x in [TX-1.04,TX+1.04]:
 box('Patisserie glass end',(x,TY,1.52),(.008,1.05,.79),'glass',.002)
 for y in [TY-.52,TY+.52]:rod('Patisserie case upright',(x,y,1.13),(x,y,1.92),.012,'steelInside')
box('Patisserie clear front',(TX,TY-.52,1.52),(2.08,.008,.79),'glass',.002)
box('Patisserie glass roof',(TX,TY,1.92),(2.12,1.08,.008),'glass',.002)
for y in [TY-.52,TY+.52]:rod('Patisserie top frame',(TX-1.04,y,1.92),(TX+1.04,y,1.92),.012,'steelInside')

def fruit_tart(n,x,y,z,r=.12):
 cyl(n+' fluted pastry base',(x,y,z+.028),r,.05,'crustLight',40)
 torus(n+' rolled crust rim',(x,y,z+.058),r-.012,.012,'crust',steps=40)
 cyl(n+' custard',(x,y,z+.051),r-.024,.012,'honey',32)
 for j in range(5):
  a=j*2*pi/5
  uv(n+' glazed blueberry',(x+.05*cos(a),y+.05*sin(a),z+.074),(.025,.025,.024),'grape',16,10)
for z in [1.184,1.544]:
 for x in [TX-.70,TX-.32,TX+.10]:
  for y in [TY-.23,TY+.16]:fruit_tart('Blueberry custard tart',x,y,z,.13)
 for y in [TY-.23,TY+.15]:
  plate('Cake slice plate',TX+.67,y,z,.19)
  box('Chocolate slice sponge',(TX+.67,y,z+.085),(.25,.20,.12),'chocolate',.012)
  for zz in [z+.07,z+.145]:box('Chocolate slice cream',(TX+.67,y,zz),(.252,.202,.022),'paper',.006)
  uv('Cake slice strawberry',(TX+.67,y,z+.19),(.035,.03,.04),'red',16,10)
for x,label in [(TX-.53,'FRUIT TARTS'),(TX+.67,'CAKE SLICES')]:
 box('Pastry ticket holder',(x,TY-.53,1.22),(.54,.012,.12),'paper',.006)
 text('Pastry ticket '+label,label,(x,TY-.541,1.204),.038,'ink')

collection('24 Bread baskets and bakery menu',offset=(.6,-.8,.2))
# Low open retail stand at the right end leaves a staff entrance behind it.
BX,BY=3.46,-2.70
crate('Freestanding bread crate',BX,BY,.38,.95,.75,.45)
for x in [BX-.35,BX+.35]:
 for y in [BY-.25,BY+.25]:rod('Bread stand leg',(x,y,.1),(x,y,.40),.03,'oak2')
for i in range(5):
 def upright_bread():baguette('Basket baguette',0,0,0,.78)
 o=local_group('Leaning retail baguette',(BX-.31+i*.16,BY+.10,.55),0,upright_bread);o.rotation_euler.x=1.13
# A-frame chalkboard at the customer entrance, complete with paired legs and hinge.
MX,MY=4.75,-3.25
for dx in [-.36,.36]:
 rod('Menu A-frame front leg',(MX+dx,MY-.28,.10),(MX+dx,MY,1.14),.027,'oak2')
 rod('Menu A-frame back leg',(MX+dx,MY+.30,.10),(MX+dx,MY,1.14),.027,'oak2')
rod('Menu board top hinge',(MX-.39,MY,1.14),(MX+.39,MY,1.14),.025,'steelInside')
board('Front-of-shop chalk menu','BAKED TODAY\n\nSourdough\nButter croissants\nFruit tarts\n\nFRESH / DAILY',MX,MY-.17,.70,.76,.86,'sageDark',.071)
# A restrained wall menu faces the retail area above the front displays.
def wall_menu():board('Bakery opening-hours board','GOOD BREAD\nTAKES TIME\n\n7 AM - 5 PM',0,0,0,1.07,.72,'ivory',.075)
local_group('Shop hours',(-3.83,.40,2.45),pi/2,wall_menu)
