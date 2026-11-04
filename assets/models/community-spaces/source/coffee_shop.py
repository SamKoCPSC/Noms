shell(7.4,6.2,3.5,tiles=False,wall='wall')
collection('04 Espresso back bar and cabinetry')
box('Back bar recessed plinth',(-.15,2.50,.18),(5.9,.67,.22),'walnut',.015)
box('Sage rear cabinet carcass',(-.15,2.48,.65),(6,.83,.86),'sage',.025)
for i in range(8):
 x=-2.76+i*.745;box('Separate sage cabinet door',(x,2.043,.65),(.72,.035,.78),'sageLight',.013)
 rod('Brass door pull',(x+.22,2.007,.69),(x+.22,2.007,.85),.011,'brass')
box('Ivory stone worktop',(-.15,2.45,1.12),(6.14,.97,.10),'stone',.035)
for j in range(3):
 for i in range(17):box('Glazed backsplash tile',(-3.0+i*.36,3.013,1.27+j*.15),(.349,.018,.139),'ivory',.006)

collection('05 Two group espresso machine')
EX,EY,EZ=-.68,2.39,1.17
box('Espresso machine base',(EX,EY,EZ+.07),(1.52,.69,.14),'steel',.045)
box('Espresso machine rear housing',(EX,EY+.13,EZ+.37),(1.46,.36,.59),'ivory',.065)
box('Espresso machine upper fascia',(EX,EY-.12,EZ+.60),(1.47,.42,.22),'ivory',.05)
box('Cup warming tray',(EX,EY+.02,EZ+.73),(1.36,.46,.025),'steelDark',.018)
for xx in [EX-.70,EX+.70]:box('Espresso solid cheek',(xx,EY-.12,EZ+.36),(.10,.48,.49),'clay',.025)
box('Drip tray black recess',(EX,EY-.17,EZ+.155),(1.26,.38,.018),'black',.012)
for i in range(22):rod('Drip tray slotted grille',(EX-.59+i*.056,EY-.33,EZ+.17),(EX-.59+i*.056,EY-.02,EZ+.17),.005,'steel',10)
for xx in [EX-.31,EX+.31]:
 cyl('Attached brew group',(xx,EY-.16,EZ+.46),.11,.17,'steel',40)
 cyl('Portafilter basket',(xx,EY-.16,EZ+.36),.091,.055,'steel',40)
 rod('Walnut portafilter handle',(xx,EY-.19,EZ+.36),(xx,EY-.44,EZ+.34),.024,'walnut2')
 for dx in [-.035,.035]:curve('Double spout',[(xx,EY-.16,EZ+.33),(xx+dx,EY-.16,EZ+.30)],.008,'steel')
 latte('Espresso in real clearance',xx,EY-.16,EZ+.18,.055)
 for k in range(3):
  o=cyl('Brew push button',(xx-.065+k*.065,EY-.337,EZ+.61),.019,.008,'steelDark',24);o.rotation_euler.x=pi/2
for xx in [EX-.59,EX+.59]:
 o=cyl('Steam control dial',(xx,EY-.345,EZ+.61),.045,.035,'walnut',32);o.rotation_euler.x=pi/2
 curve('Articulated steam wand',[(xx,EY-.23,EZ+.48),(xx,EY-.29,EZ+.36),(xx+.055,EY-.38,EZ+.22)],.012,'steel')
for xx in [EX-.21,EX+.21]:mug('Cups on warming tray',xx,EY+.04,EZ+.75,.052,.085)
text('Machine badge','N O M S',(EX,EY-.341,EZ+.685),.065,'walnut')

collection('06 Coffee grinders and bar tools')
for x in [-2.55,-1.87]:
 box('Grinder footprint',(x,2.30,1.21),(.43,.42,.08),'steelDark',.045)
 lathe('Grinder sculpted body',(x,2.38,1.24),[(0,0),(.16,0),(.18,.12),(.13,.40),(.12,.45),(0,.45)],'ivory',48)
 cyl('Grind adjustment collar',(x,2.38,1.72),.16,.07,'steelDark',48)
 lathe('Clear bean hopper',(x,2.38,1.75),[(.10,0),(.18,.23),(.18,.33),(.173,.33),(.173,.24),(.09,.01)],'glass',48)
 for i in range(26):
  a=i*2.4;rr=.12*math.sqrt((i+.5)/26);uv('Individual coffee bean',(x+rr*cos(a),2.38+rr*sin(a),1.91+.015*(i%3)),(.018,.012,.009),'chocolate',12,8)
 cyl('Hopper lid',(x,2.38,2.095),.188,.025,'walnut2',48)
 box('Coffee dosing chute',(x,2.18,1.56),(.10,.19,.08),'steel',.012)
 curve('Portafilter fork',[(x-.075,2.13,1.40),(x-.075,2.04,1.40),(x+.075,2.04,1.40),(x+.075,2.13,1.40)],.009,'steel',poly=True)
box('Tamping mat',(1.03,2.24,1.178),(.48,.35,.012),'black',.02)
cyl('Espresso tamper foot',(1.03,2.24,1.195),.064,.025,'steel',32);lathe('Walnut tamper grip',(1.03,2.24,1.21),[(0,0),(.035,0),(.022,.04),(.045,.10),(.03,.125),(0,.13)],'walnut2',32)
jug('Milk steaming pitcher',1.36,2.27,1.17)
box('Coffee knock box',(1.78,2.31,1.25),(.27,.27,.16),'steelDark',.025)
box('Knock box hollow opening',(1.78,2.31,1.334),(.22,.22,.005),'black',.02)
rod('Knock bar',(1.65,2.31,1.35),(1.91,2.31,1.35),.016,'walnut')

collection('07 Front fluted counter and pastry cabinet')
box('Front counter body',(-1.07,.37,.61),(4.06,.79,1.02),'sageDark',.045)
for i in range(48):box('Rounded vertical counter flute',(-3.025+i*.083,.0,.62),(.054,.065,.96),'sage',.022)
rounded_slab('Rounded stone serving ledge',(-1.07,.30,1.17),4.27,1.02,.10,.13,'stone')
box('Pastry display walnut plinth',(-2.03,.27,1.26),(1.83,.77,.09),'walnut2',.035)
for z in [1.32,1.67]:
 box('Pastry case glass shelf',(-2.03,.28,z),(1.68,.65,.014),'glass',.002)
 for i in range(4):croissant('Approved rolled croissant',(-2.65+i*.41,.26,z+.012),.33,(-.1 if i%2 else .1))
for x in [-2.94,-1.12]:box('Glass pastry case end',(x,.27,1.57),(.014,.72,.60),'glass',.002)
box('Pastry case front glazing',(-2.03,-.099,1.57),(1.83,.013,.60),'glass',.001)
box('Pastry case top',(-2.03,.27,1.88),(1.85,.75,.024),'glass',.002)
for x in [-2.95,-1.11]:rod('Pastry case front bronze frame',(x,-.11,1.28),(x,-.11,1.89),.010,'brass')
for z in [1.28,1.89]:rod('Pastry case bronze rail',(-2.95,-.11,z),(-1.11,-.11,z),.01,'brass')
board('Pastry label','FRESHLY BAKED',-2.03,-.123,1.39,.67,.14,'ivory',.058)

collection('08 Pour over service and point of sale')
for x in [-.48,.07]:
 rounded_slab('Coffee weighing scale',(x,.25,1.23),.39,.33,.034,.03,'black')
 lathe('Glass coffee carafe',(x,.25,1.25),[(0,0),(.13,0),(.14,.045),(.085,.19),(.07,.23),(.075,.26),(.065,.26),(.060,.23),(.076,.19),(.13,.045),(0,.015)],'glass',48)
 lathe('Coffee in carafe',(x,.25,1.267),[(0,0),(.12,0),(.105,.095),(0,.095)],'tea',40)
 lathe('Ceramic pour over cone',(x,.25,1.51),[(.025,0),(.14,.19),(.13,.20),(.018,.025)],'ceramic',48)
 for i in range(18):
  a=2*pi*i/18;curve('Dripper spiral ridge',[(x+.033*cos(a),.25+.033*sin(a),1.55),(x+.075*cos(a+.2),.25+.075*sin(a+.2),1.62),(x+.129*cos(a+.35),.25+.129*sin(a+.35),1.69)],.003,'ivory')
lathe('Gooseneck kettle body',(.57,.32,1.23),[(0,0),(.14,0),(.15,.06),(.13,.22),(.08,.27),(0,.27)],'steelDark',48)
cyl('Kettle lid',(.57,.32,1.51),.09,.015,'walnut2',40)
curve('Kettle swan neck',[(.44,.32,1.27),(.28,.32,1.37),(.28,.32,1.55),(.22,.32,1.59)],.012,'steelDark')
curve('Kettle handle',[(.68,.32,1.46),(.83,.32,1.45),(.85,.32,1.30),(.69,.32,1.28)],.022,'walnut2')

collection('09 Coffee menu and retail shelf')
board('Cafe main sign','NOMS COFFEE',.3,2.99,2.90,2.63,.49,'sageDark',.23)
board('Coffee menu','ESPRESSO  /  FILTER  /  LATTE',.30,2.98,2.41,2.62,.35,'ivory',.105)
box('Retail wall shelf',(2.23,2.82,2.12),(1.72,.36,.065),'oak',.015)
for i in range(4):
 x=1.64+i*.38;box('Folded coffee bag',(x,2.80,2.34),(.26,.15,.36),'linen',.025)
 box('Coffee bag folded seal',(x,2.80,2.535),(.26,.05,.035),'walnut2',.008)
 box('Coffee bag origin label',(x,2.717,2.33),(.18,.007,.18),['sage','clay','honey','teal'][i],.002)
 text('Coffee bag label',['KENYA','PERU','ETHIOPIA','COLOMBIA'][i],(x,2.711,2.32),.023,'paper')

collection('10 Cafe tables and chairs')
for x,y in [(2.10,.13),(-1.20,-2.06)]:
 cyl('Round cafe tabletop',(x,y,.88),.59,.075,'oak',64)
 lathe('Pedestal table base',(x,y,.07),[(0,0),(.31,0),(.32,.025),(.13,.085),(.055,.17),(.055,.73),(.13,.77),(0,.77)],'sageDark',48)
 latte('Cafe latte',x-.12,y-.08,.93,.085);plate('Small pastry plate',x+.24,y+.05,.925,.16);croissant('Table croissant',(x+.24,y+.05,.952),.24,.3)
 chair('Cafe curved chair',(x-.9,y,.015),-pi/2);chair('Cafe curved chair',(x+.88,y,.015),pi/2)

collection('11 Pendant lights and modest greenery')
for x in [-2.22,-.45]:pendant('Terracotta pendant',x,.35,2.67)
pot('Window herb',-3.42,-.55,1.345,.11)
finish('specialty-coffee-shop','Noms — Specialty Coffee Shop',10.8,(0,0,1.1))
