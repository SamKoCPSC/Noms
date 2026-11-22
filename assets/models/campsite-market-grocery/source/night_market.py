collection('01 Lantern courtyard paving',offset=(0,0,0))
rounded_slab('Rounded courtyard stone plinth',(0,0,-.13),9.4,7.4,.28,.30,'walnut')
rounded_slab('Continuous blue slate paving',(0,0,.045),9.23,7.23,.07,.24,'slate')
for j in range(15):
 for i in range(19):box('Individual courtyard paver',(-4.37+i*.485,-3.37+j*.48,.089),(.475,.470,.022),['slate','navy','steelDark'][(i*3+j)%3],.007)
collection('02 Courtyard plaster and timber backdrop')
box('Warm courtyard back wall',(0,3.51,1.18),(9.2,.15,2.22),'stone',.04)
box('Low courtyard side wall',(-4.51,1.06,.76),(.15,4.9,1.39),'stone',.035)
for z in [.35,.71,1.07]:box('Side wall brick course',(-4.423,1.06,z),(.008,4.87,.012),'linen',.002)
for i in range(21):box('Rear courtyard narrow timber slat',(-4.28+i*.423,3.416,1.39),(.17,.04,2.57),'walnut2',.012)
for x in [-4.1,4.1]:
 box('Rear lantern support',(x,3.30,1.88),(.10,.10,3.55),'walnut2',.015)

def cart(n,w=2.8,color='teal',title='NOODLES',awning=True):
 for x in [-w/2+.30,w/2-.30]:
  for y in [-.28,.28]:
   o=cyl(n+' rubber cart wheel',(x,y,.245),.155,.065,'black',32);o.rotation_euler.x=pi/2
   o=cyl(n+' brass wheel hub',(x,y-.037,.245),.062,.009,'brass',24);o.rotation_euler.x=pi/2
 box(n+' metal lower frame',(0,0,.45),(w,.68,.10),'steelDark',.018)
 box(n+' painted panel',(0,0,.83),(w,.75,.71),color,.03)
 for i in range(int(w/.18)):
  x=-w/2+.11+i*.18;box(n+' facade panel divider',(x,-.39,.83),(.018,.026,.63),'walnut2',.007)
 rounded_slab(n+' stone service top',(0,-.045,1.23),w+.16,.94,.10,.07,'stone')
 for x in [-w/2+.05,w/2-.05]:box(n+' canopy timber post',(x,.26,1.75),(.075,.075,3.05),'walnut2',.01)
 if awning:
  # Alternating sewn canvas panels follow a curved roof profile.
  N=10
  for i in range(N):
   a=-w/2-.12+i*(w+.24)/N;b=a+(w+.24)/N
   path=[(.54,3.21),(.16,3.16),(-.27,3.02),(-.63,2.88)]
   v=[(x,y,z)for x in [a,b]for y,z in path]
   o=mesh(n+' striped canopy',v,[(j,j+1,4+j+1,4+j)for j in range(3)],color if i%2==0 else 'linen');m=o.modifiers.new('Canvas thickness','SOLIDIFY');m.thickness=.014
   box(n+' scalloped canopy valance',((a+b)/2,-.63,2.80),(b-a-.004,.025,.17),color if i%2==0 else 'linen',.025)
 board(n+' fascia',title,0,-.46,2.51,w*.88,.32,color,.13 if w<3 else .17)
 area(n+' under canopy light',(0,-.06,2.48),28,.65,(1,.70,.42),(0,0,1.2))

collection('03 Noodle cart along courtyard side')
def noodle_cart():
 cart('Noodle cart',3.10,'teal','HAND PULLED NOODLES')
 for x in [-1.00,-.39,.22]:noodle_bowl('Noodle counter bowl',x,-.13,1.28,.20)
 box('Wok induction base',(.99,.08,1.33),(.48,.45,.09),'steelDark',.022)
 lathe('Small cooking wok',(.99,.08,1.39),[(0,0),(.12,.03),(.24,.16),(.25,.19),(.24,.195),(.22,.16),(.10,.045),(0,.02)],'black',64)
 rod('Wok attached handle',(1.22,.08,1.56),(1.40,.08,1.60),.025,'walnut2')
 for i in range(6):curve('Wok cooked noodles',[(.84+j*.047,.03+i*.021+.007*sin(i+j),1.50+.004*sin(j))for j in range(6)],.005,'pasta')
 for x in [-.95,-.60]:jar('Noodle chilli jar',x,.28,1.28,'red',.068,.21)
 jug('Noodle broth pitcher',-.22,.30,1.28)
local_group('Side noodle cart',(-3.15,1.00,0),pi/2,noodle_cart)

collection('04 Bun and skewer cart on rear wall')
def feast_cart():
 cart('Shared bites cart',4.75,'clay','BAO  /  GRILLED SKEWERS')
 for x in [-1.76,-1.10,-.44]:steamer('Bamboo bao basket',x,-.03,1.28,.27)
 for i in range(2):steamer('Steamer stacked tier',-1.82,.32,1.28+.145*i,.23,False)
 for x in [.46,1.38]:
  rounded_slab('Grilled skewer platter',(x,-.10,1.30),.73,.58,.025,.05,'darkPlate')
  for i in range(3):
   xx=x-.23+i*.23;rod('Bamboo skewer',(xx,-.38,1.35),(xx,.21,1.35),.005,'bamboo',10)
   for j,ma in enumerate(['beefCrust','leaf','beefCrust','red']):
    yy=-.25+j*.105;pill('Grilled skewer bite',(xx,yy,1.385),(.14,.082,.075),ma)
 for x in [.25,.62,1.0]:tin('Soda on ice counter',x,.30,1.28,'teal' if x==.62 else 'redCan','SODA',.065,.20)
 board('Shared bites price sign','SHARE A PLATE',1.84,.22,1.63,.72,.39,'sageDark',.065)
local_group('Rear street food cart',(.62,2.73,0),0,feast_cart)

collection('05 Bubble tea cart and curved parasol')
def tea_cart():
 cart('Tea cart',1.62,'sageDark','MILK TEA',False)
 # Tea cart uses its own umbrella rather than tall redundant canopy poles.
 for o in list(bpy.data.objects):
  if o.name.startswith('Tea cart canopy timber post'):bpy.data.objects.remove(o,do_unlink=True)
 for i in range(3):
  x=-.52+i*.48
  lathe('Iced milk tea cup',(x,-.16,1.29),[(0,0),(.077,0),(.093,.29),(.087,.29),(.070,.015),(0,.015)],'glass',40)
  cyl('Milk tea drink',(x,-.16,1.43),.077,.25,'linen',40)
  for j in range(8):
   a=j*2.4;uv('Milk tea tapioca pearl',(x+.048*cos(a),-.16+.048*sin(a),1.31+.018*(j%2)),(.013,.013,.013),'chocolate',12,8)
  cyl('Milk tea lid',(x,-.16,1.587),.096,.012,'ivory',40)
  rod('Milk tea wide straw',(x,-.16,1.42),(x+.015,-.16,1.76),.008,'clay',12)
 for x in [-.42,.05,.51]:jar('Tea jar',x,.24,1.28,'tea',.09,.29)
 # Reposition the cart sign to the actual front panel.
 for suffix in [' walnut frame',' painted face',' lettering']:
  o=bpy.data.objects.get('Tea cart fascia'+suffix)
  if o:o.location.z-=1.68
 rod('Umbrella supported pole',(.63,.31,.45),(.63,.31,3.15),.023,'brass')
 N=48;v=[]
 for j in range(7):
  u=j/6;r=1.04*u;z=3.15-.41*u**1.4
  for i in range(N):
   a=i*2*pi/N;v.append((.63+r*cos(a),.31+r*sin(a),z-.035*u*sin(6*a)**2))
 for sector in range(6):
  faces=[(j*N+i,j*N+(i+1)%N,(j+1)*N+(i+1)%N,(j+1)*N+i)for j in range(6)for i in range(sector*8,(sector+1)*8)]
  smooth(mesh('Parasol sewn fabric panel',v,faces,'peach' if sector%2 else 'linen'))
 for i in range(6):
  a=i*2*pi/6;curve('Parasol bronze rib',[(.63,.31,3.14),(.63+.55*cos(a),.31+.55*sin(a),2.98),(.63+1.04*cos(a),.31+1.04*sin(a),2.73)],.008,'brass')
local_group('Milk tea parasol cart',(3.16,-.53,0),-.12,tea_cart)

collection('06 Round shared courtyard tables')
for x,y in [(-.85,-1.88),(1.10,-1.78)]:
 cyl('Round shared table',(x,y,.90),.60,.065,'oak',64)
 lathe('Shared table metal pedestal',(x,y,.105),[(0,0),(.28,0),(.30,.025),(.09,.075),(.055,.12),(.055,.73),(0,.76)],'steelDark',48)
 noodle_bowl('Shared supper noodles',x-.20,y-.04,.938,.17)
 plate('Small shared bao plate',x+.24,y+.07,.94,.18);bao('Shared supper bun',x+.24,y+.07,.962,.11)
 enamel_cup('Courtyard tea mug',x+.04,y+.31,.94,'ivory',.061,.11)
 for yy in [y-.88,y+.88]:
  cyl('Round timber street stool',(x,yy,.51),.23,.075,'oak3',48)
  for i in range(3):
   a=i*2*pi/3;rod('Stool angled leg',(x+.20*cos(a),yy+.20*sin(a),.10),(x+.14*cos(a),yy+.14*sin(a),.475),.023,'steelDark')

collection('07 Lantern canopy across the courtyard')
for x,y in [(-4.1,-1.05),(4.1,.62)]:box('Front lantern pole',(x,y,1.88),(.10,.10,3.55),'walnut2',.015)
curve('Front lantern festoon',[(-4.1,-1.05,3.65),(0,-.215,3.22),(4.1,.62,3.65)],.010,'black')
for i in range(10):
 t=(i+.3)/9.6;x=-4.1+8.2*t;y=-1.05+1.67*t;z=3.65-.43*sin(pi*t)-.30
 paper_lantern('Courtyard front paper lantern',x,y,z,.20 if i%2 else .22)
curve('Rear lantern festoon',[(-4.1,3.30,3.65),(0,3.30,3.42),(4.1,3.30,3.65)],.009,'black')
for i in range(7):
 x=-3.70+i*1.22;z=3.65-.23*sin(pi*(x+4.1)/8.2)-.29;paper_lantern('Rear paper lantern',x,3.30,z,.18)

collection('08 Welcome sign and street details')
for x in [-3.70,-2.86]:
 rod('Welcome board front leg',(x,-2.86,.10),(x,-2.64,1.28),.026,'walnut2')
 rod('Welcome board rear leg',(x,-2.28,.10),(x,-2.64,1.28),.026,'walnut2')
board('Night courtyard sign','NOMS\nNIGHT MARKET',-3.28,-2.67,.89,1.05,.91,'navy',.11)
text('Night courtyard welcome','GOOD FOOD / GOOD COMPANY',(-3.28,-2.718,.62),.040,'honey')
crate('Return crockery crate',3.75,-2.48,.11,.93,.62,.34)
for x in [3.56,3.95]:
 for i in range(4):bowl('Returned ceramic bowl',x,-2.48,.18+i*.05,.15)
pot('Courtyard herb pot',-4.08,-1.93,.11,.16)
finish('night-market-lantern-courtyard','Noms — Lantern Courtyard Night Market',13.3,(0,0,1.1),night=True)
