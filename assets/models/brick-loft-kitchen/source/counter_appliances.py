"""Countertop appliances, in their own assembly groups for future tour entrances."""
collection('15 Espresso machine and grinder',offset=(0,-.7,.25))
def espresso():
 box('Espresso machine body',(0,.025,.245),(.50,.37,.43),'cabinet',.035)
 box('Espresso machine brushed front',(0,-.169,.295),(.45,.025,.29),'steelSoft',.015)
 box('Espresso machine drip tray',(0,-.09,.035),(.49,.48,.07),'steelSoft',.016)
 box('Espresso machine dark drip grate',(0,-.16,.074),(.40,.25,.013),'black',.007)
 for i in range(10):rod('Espresso drip grate bar',(-.177+i*.039,-.27,.084),(-.177+i*.039,-.055,.084),.004,'steel',10)
 box('Espresso cup warming top',(0,.025,.469),(.46,.34,.022),'steel',.008)
 for x in [-.21,.21]:rod('Espresso cup rail',(x,-.08,.49),(x,.17,.49),.007,'steel',12)
 rod('Espresso back cup rail',(-.21,.17,.49),(.21,.17,.49),.007,'steel',12)
 # Gauge bezel, face and pointer face toward the cooking aisle.
 for n,r,y,m in [('Pressure gauge bezel',.048,-.191,'steel'),('Pressure gauge ivory dial',.039,-.201,'paper')]:
  o=cyl(n,(-.09,y,.356),r,.012,m,40);o.rotation_euler.x=pi/2
 for i in range(9):
  a=-pi*.18+i*pi*1.36/8
  rod('Pressure gauge tick',(-.09+.028*cos(a),-.209,.356+.028*sin(a)),(-.09+.033*cos(a),-.209,.356+.033*sin(a)),.0014,'black',8)
 rod('Pressure gauge needle',(-.09,-.211,.356),(-.105,-.211,.382),.002,'rust',10)
 for x in [.04,.13]:
  o=cyl('Espresso brew button',(x,-.202,.365),.020,.022,'black',24);o.rotation_euler.x=pi/2
 cyl('Espresso group head',(0,-.22,.245),.066,.068,'steel',40)
 cyl('Portafilter basket',(0,-.22,.195),.059,.025,'steelDark',40)
 rod('Portafilter black handle',(0,-.24,.196),(0,-.42,.196),.018,'black',16)
 for x in [-.022,.022]:curve('Dual espresso spout',[(x,-.22,.18),(x,-.23,.155),(x,-.245,.15)],.006,'steel')
 curve('Steam wand',[(.18,-.18,.28),(.22,-.20,.22),(.23,-.28,.13),(.21,-.30,.10)],.009,'steel')
 o=cyl('Steam control knob',(.225,-.09,.32),.027,.03,'black',24);o.rotation_euler.y=pi/2
 # Small empty cup avoids a coffee surface under the stored cup.
 lathe('Espresso cup on warming tray',(-.10,.02,.482),[(0,0),(.031,0),(.040,.066),(.033,.066),(.026,.008),(0,.008)],'ceramic',32)
 curve('Espresso cup handle',[(-.064,.02,.53),(-.040,.02,.536),(-.039,.02,.506),(-.067,.02,.50)],.005,'ceramic')
local_group('Home espresso station',(1.68,2.25,1.055),0,espresso)
def grinder():
 rounded_slab('Burr grinder base',(0,0,.018),.23,.29,.036,.025,'black')
 box('Burr grinder upright',(0,.04,.19),(.18,.16,.32),'steelSoft',.022)
 cyl('Grinder adjustment collar',(0,.04,.35),.10,.035,'black',40)
 lathe('Smoked bean hopper',(0,.04,.365),[(0,0),(.065,0),(.095,.11),(.092,.12),(.082,.12),(.056,.015),(0,.015)],'glass',40)
 for i in range(17):
  a=i*2.4;r=.065*math.sqrt((i+1)/17)
  o=uv('Coffee bean in hopper',(r*cos(a),.04+r*sin(a),.395+.018*(i%3)),(.014,.009,.008),'chocolate',12,8);o.rotation_euler.z=a
 cyl('Bean hopper lid',(0,.04,.49),.10,.018,'black',40)
 box('Grinder grounds chute',(0,-.07,.24),(.08,.10,.065),'black',.008)
 lathe('Steel dosing cup',(0,-.075,.04),[(0,0),(.045,0),(.05,.10),(.043,.10),(.038,.008),(0,.008)],'steel',32)
local_group('Coffee burr grinder',(1.22,2.30,1.055),0,grinder)
# Adjacent outlets and tucked cords make the appliances feel installed.
box('Coffee wall outlet',(1.72,2.65,1.64),(.16,.024,.11),'ivory',.01)
for x in [1.68,1.76]:
 box('Coffee outlet plug',(x,2.62,1.64),(.042,.043,.05),'black',.007)
 curve('Coffee appliance cable',[(x,2.60,1.64),(x,2.55,1.13),(x-.10,2.51,1.07),(x-.14,2.42,1.15)],.007,'black')
collection('16 Two-slot toaster',offset=(0,-.7,.25))
def toaster():
 rounded_slab('Toaster black foot',(0,0,.018),.47,.29,.036,.035,'black')
 box('Toaster cream rounded shell',(0,0,.15),(.48,.30,.27),'cream',.045)
 box('Toaster stainless top',(0,0,.285),(.41,.245,.018),'steelSoft',.018)
 for y in [-.062,.062]:
  rounded_slab('Toaster dark bread slot',(0,y,.297),.31,.043,.006,.020,'black')
  # A pair of toast slices emerging naturally from separate recessed slots.
  box('Toast browned crust',(0,y,.327),(.24,.026,.075),'crust',.018)
  box('Toast crumb face',(0,y-.014,.329),(.213,.003,.053),'dough',.012)
 for i in range(3):
  o=cyl('Toaster front button',(-.12+i*.085,-.153,.12),.012,.012,'black',20);o.rotation_euler.x=pi/2
 o=cyl('Toaster browning dial',(.13,-.158,.15),.027,.016,'steelDark',32);o.rotation_euler.x=pi/2
 rod('Toaster dial index',(.13,-.169,.15),(.14,-.169,.167),.002,'ivory',8)
 box('Toaster lever slot',(.243,0,.18),(.006,.017,.13),'black',.003)
 box('Toaster lever',(.265,0,.20),(.055,.045,.018),'black',.005)
local_group('Breakfast toaster',(-1.19,2.24,1.055),0,toaster)
curve('Toaster power lead',[(-1.34,2.38,1.09),(-1.50,2.45,1.07),(-1.52,2.62,1.10),(-1.52,2.64,1.36)],.006,'black')
box('Toaster socket',(-1.52,2.65,1.36),(.075,.025,.095),'ivory',.008)
collection('17 Electric kettle by window',offset=(-.65,-.25,.25))
def kettle():
 cyl('Kettle circular power base',(0,0,.018),.133,.036,'black',48)
 lathe('Sage electric kettle body',(0,0,.038),[(0,0),(.105,0),(.12,.04),(.118,.17),(.09,.235),(.074,.25),(0,.25)],'sage',56)
 cyl('Kettle lid',(0,0,.294),.078,.014,'steel',40)
 uv('Kettle lid knob',(0,0,.313),(.022,.022,.016),'black',20,12)
 # Tapered open pouring spout constructed along an angled local axis.
 def spout():lathe('Kettle open pouring spout',(0,0,0),[(.047,0),(.033,.08),(.025,.16),(.018,.16),(.023,.08),(.036,0)],'steelSoft',40)
 before=set(bpy.data.objects);spout();o=list(set(bpy.data.objects)-before)[0];o.location=(0,-.09,.17);o.rotation_euler.x=.85
 curve('Kettle black handle',[(0,.075,.26),(0,.17,.28),(0,.20,.20),(0,.18,.10),(0,.10,.08)],.020,'black')
 box('Kettle illuminated switch',(.085,.07,.065),(.036,.022,.012),'steelDark',.005)
local_group('Sage electric kettle',(2.47,-.43,1.055),-pi/2,kettle)
curve('Kettle power cord',[(2.58,-.43,1.08),(2.78,-.48,1.07),(2.85,-.38,1.10),(2.86,-.28,1.32)],.006,'black')
box('Kettle wall socket',(2.875,-.28,1.32),(.025,.075,.095),'ivory',.008)
