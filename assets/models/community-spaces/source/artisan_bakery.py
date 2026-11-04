shell(7.6,6.6,3.55,tiles=True,wall='wall')
collection('04 Hearth oven stonework')
OX,OY=-2.36,2.36
box('Bakery oven limestone body',(OX,OY,1.37),(1.95,1.16,2.61),'stone',.09)
box('Hearth oven base molding',(OX,OY,.31),(2.06,1.24,.18),'ivory',.035)
box('Oven upper cornice',(OX,OY,2.75),(2.12,1.30,.16),'ivory',.03)
box('Oven chimney',(OX,OY+.13,3.12),(1.17,.83,.65),'stone',.04)
# Dark arch is inset behind a ring of individually modeled tapered stones.
yy=OY-.591;zc=1.73;rad=.66;v=[(OX-rad,yy,1.05),(OX+rad,yy,1.05)]+[(OX+rad*cos(i*pi/32),yy,zc+rad*sin(i*pi/32))for i in range(33)]
mesh('Deep arched oven cavity',v,[tuple(range(len(v)))],'black')
for i in range(11):
 a=i*pi/11+.008;b=(i+1)*pi/11-.008;vs=[]
 for y in [yy-.13,yy+.015]:
  vs.extend([(OX+r*cos(t),y,zc+r*sin(t))for r,t in [(.665,a),(.85,a),(.85,b),(.665,b)]])
 o=mesh('Individual stone arch voussoir',vs,[(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)],'ivory');bevel(o,.008,3)
for dx in [-.75,.75]:
 for i in range(3):box('Hearth upright stone',(OX+dx,yy-.06,1.15+i*.21),(.19,.24,.20),'ivory',.012)
box('Projecting oven hearth',(OX,OY-.40,1.075),(1.94,1.03,.12),'steelDark',.025)
for xx in [OX-.36,OX+.30]:loaf('Bread baking in hearth',xx,OY-.49,1.14,.48,.15)
box('Oven fuel drawer',(OX,OY-.60,.66),(1.37,.045,.40),'steelDark',.035)
rod('Fuel drawer handle',(OX-.29,OY-.69,.70),(OX+.29,OY-.69,.70),.022,'steel')
o=cyl('Oven thermometer body',(OX,OY-.618,2.60),.081,.025,'brass',48);o.rotation_euler.x=pi/2
o=cyl('Oven thermometer ivory dial',(OX,OY-.634,2.60),.067,.006,'paper',40);o.rotation_euler.x=pi/2
rod('Oven gauge needle',(OX,OY-.641,2.60),(OX+.034,OY-.641,2.63),.003,'ink',8)

collection('05 Bread shelves and cooling rack')
box('Bakery shelf backing',(1.52,3.18,1.89),(3.63,.09,2.71),'sageDark',.022)
for xx in [-.33,1.52,3.37]:box('Bakery shelf upright',(xx,2.94,1.81),(.07,.54,2.63),'walnut2',.012)
for z in [.53,1.22,1.91,2.60,3.12]:
 box('Thick bakery display shelf',(1.52,2.91,z),(3.77,.62,.07),'walnut2',.012)
 if z<3:
  for j in range(5):
   x=[.12,.90,1.87,2.44,3.01][j]
   if z==1.91:baguette('Display baguette',x,2.92,z+.04,.48,pi/2)
   else:loaf('Shelf sourdough',x,2.91,z+.04,.52,.08*(j%2))
box('Bread shelf front fascia',(1.52,2.59,3.035),(3.77,.05,.18),'walnut2',.012)
text('Back shelf bakery name','FLOUR  /  WATER  /  TIME',(1.52,2.56,3.02),.095,'ivory')

collection('06 Baker workbench and flour drawer')
Z=table('Working butcher block',.63,.42,.97,3.0,1.20,'oak3')
box('Workbench lower tray',(.63,.42,.34),(2.65,1.02,.055),'oak2',.015)
for x in [-.28,.43,1.14]:
 bowl('Proofing basket',x,.41,.38,.25,'bamboo')
 for j in range(6):torus('Banneton coiled wicker',(x,.41,.40+j*.014),.17+j*.014,.006,'bambooDark',steps=48)
box('Workbench apron drawer',(.63,-.195,.79),(1.22,.035,.19),'oak',.012)
rod('Workbench drawer pull',(.39,-.24,.79),(.87,-.24,.79),.014,'brass')
# Rolled dough sheet has soft edges; the cutting tool stays above it.
pill('Rolled dough sheet',(.42,.31,Z+.018),(.90,.53,.035),'dough')
rod('Rolling pin barrel',(-.12,.62,Z+.065),(.62,.62,Z+.065),.055,'oak3',40)
rod('Rolling pin left handle',(-.26,.62,Z+.065),(-.12,.62,Z+.065),.024,'walnut2')
rod('Rolling pin right handle',(.62,.62,Z+.065),(.76,.62,Z+.065),.024,'walnut2')
box('Dough scraper steel blade',(1.08,.23,Z+.01),(.18,.13,.006),'steel',.006)
rod('Dough scraper grip',(.99,.30,Z+.028),(1.17,.30,Z+.028),.019,'walnut2')
bowl('Bakers mixing bowl',1.48,.47,Z,.28,'ceramic')
uv('Dough rising in bowl',(1.48,.47,Z+.08),(.21,.20,.09),'dough',32,16)
for i in range(24):
 a=i*2.399;r=.33*math.sqrt((i+.5)/24);uv('A dusting of flour',(-.44+r*cos(a),.25+r*sin(a),Z+.004),(.012,.009,.002),'paper',10,6)
lathe('Flour sifter',(-.46,.71,Z),[(0,0),(.10,0),(.11,.18),(.10,.18),(.09,.02),(0,.02)],'steel',40)

collection('07 Fluted bakery service counter')
box('Bakery front counter',(-.28,-1.80,.62),(5.65,.86,1.08),'clay',.05)
for i in range(56):box('Bakery counter rounded flute',(-3.01+i*.10,-2.235,.64),(.065,.055,1.0),'clay',.023)
rounded_slab('Bakery pale stone countertop',(-.28,-1.80,1.21),5.83,1.02,.10,.13,'stone')
text('Bakery counter lettering','N O M S  /  B A K E H O U S E',(-.28,-2.272,.66),.16,'ivory')

collection('08 Glass viennoiserie display')
PX,PY=-1.67,-1.80
box('Pastry cabinet wooden base',(PX,PY,1.30),(2.78,.80,.08),'walnut2',.025)
for z in [1.35,1.72]:
 rounded_slab('Display baking sheet',(PX,PY,z),2.57,.67,.018,.035,'steelDark')
 rounded_slab('Pastry parchment',(PX,PY,z+.012),2.51,.62,.003,.028,'paper')
 for j in range(2):
  for i in range(5):croissant('Rolled butter croissant',(PX-1.02+i*.51,PY-.17+j*.34,z+.017),.38,(-.13 if j else .10))
for x in [PX-1.39,PX+1.39]:box('Pastry display glass end',(x,PY,1.67),(.014,.79,.73),'glass',.002)
box('Pastry display glass front',(PX,PY-.40,1.67),(2.79,.014,.73),'glass',.002)
box('Pastry display glass roof',(PX,PY,2.04),(2.82,.82,.022),'glass',.002)
for x in [PX-1.40,PX+1.40]:rod('Display bronze corner',(x,PY-.41,1.30),(x,PY-.41,2.05),.009,'brass')
for z in [1.30,2.05]:rod('Display bronze rail',(PX-1.40,PY-.41,z),(PX+1.40,PY-.41,z),.009,'brass')
board('Viennoiserie name','BUTTER CROISSANTS',PX,PY-.429,1.47,1.03,.17,'ivory',.065)

collection('09 Cakes fruit danishes and takeaway bags')
for x in [.53,1.42,2.28]:
 plate('Pastry display platter',x,-1.81,1.27,.35)
 if x==.53:
  cyl('Cake pedestal foot',(x,-1.81,1.31),.18,.035,'ceramic');cyl('Cake pedestal stem',(x,-1.81,1.42),.055,.19,'ceramic');plate('Cake pedestal top',x,-1.81,1.52,.34)
  cyl('Cake sponge layer',(x,-1.81,1.66),.26,.22,'crustLight',64)
  cyl('Cake buttercream filling',(x,-1.81,1.64),.266,.035,'paper',64)
  cyl('Cake icing top',(x,-1.81,1.78),.27,.027,'paper',64)
  for i in range(7):
   a=2*pi*i/7;fruit('Cake berry',x+.19*cos(a),-1.81+.19*sin(a),1.797,'tomato',.034)
 else:
  for i in range(3):
   xx=x+(i-1)*.16;yy=-1.81+(.09 if i%2 else -.04)
   pill('Danish laminated base',(xx,yy,1.32),(.22,.24,.06),'crustLight')
   curve('Danish rolled border',rounded_points(xx,yy,1.36,.20,.22,.05),.019,'crust',True,True)
   uv('Danish custard center',(xx,yy,1.36),(.07,.075,.02),'honey',24,12)
   for j in range(3):uv('Danish berry',(xx+(j-1)*.034,yy,1.39),(.022,.023,.021),'grape',16,8)

collection('10 Flour sacks bread peel and wall menu')
for x,y in [(-3.14,-.28),(-2.94,-.90)]:
 pill('Soft folded flour sack',(x,y,.43),(.56,.47,.66),'linen')
 torus('Rolled sack mouth',(x,y,.77),.18,.025,'linen',steps=48)
 cyl('Visible flour in sack',(x,y,.76),.16,.008,'paper',40)
 text('Flour sack print','STONE\nGROUND',(x,y-.242,.46),.065,'walnut')
# Bread peel is attached to a long wooden handle, propped at the oven side.
def peel():
 rounded_slab('Bread peel blade',(0,0,0),.48,.50,.025,.16,'oak3')
 rod('Bread peel long handle',(0,.20,0),(0,1.48,0),.022,'walnut2')
o=local_group('Whole bread peel',(-3.43,1.27,.32),0,peel);o.rotation_euler.x=1.04
collection('11 Bakery pendants')
for x in [-1.6,1.25]:pendant('Bakery pendant',x,-.30,2.86)
finish('artisan-bakery','Noms — Artisan Bakery',11.4,(0,0,1.15))
