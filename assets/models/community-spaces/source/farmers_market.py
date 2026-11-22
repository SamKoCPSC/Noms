# Direct mesh creation avoids repeated scene-wide operator updates in this larger scene.
# Dimensions, bevels and materials match the shared primitives.
import bmesh

def _market_primitive(n, loc, mat, builder):
 bm=bmesh.new();builder(bm);data=bpy.data.meshes.new(n);bm.to_mesh(data);bm.free()
 o=bpy.data.objects.new(n,data);COL.objects.link(o);o.parent=ROOT;o.location=loc
 assign(o,mat);return o

def box(n,loc,dims,m,b=.02):
 def build(bm):
  bmesh.ops.create_cube(bm,size=1)
  for v in bm.verts:
   for i in range(3):v.co[i]*=dims[i]
 o=_market_primitive(n,loc,m,build)
 if b:bevel(o,b)
 return o

def uv(n,loc,sc,m,seg=24,rings=12):
 def build(bm):
  bmesh.ops.create_uvsphere(bm,u_segments=seg,v_segments=rings,radius=1)
  for v in bm.verts:
   for i in range(3):v.co[i]*=sc[i]
 return smooth(_market_primitive(n,loc,m,build))

def cyl(n,loc,r,h,m,vertices=64):
 def build(bm):bmesh.ops.create_cone(bm,cap_ends=True,cap_tris=False,segments=vertices,radius1=r,radius2=r,depth=h)
 o=_market_primitive(n,loc,m,build);bevel(o,min(.007,h*.15),3);return smooth(o)

def stall(n,x,y,w,color,caption,kind):
 frame='steelDark' if kind=='cheese' else 'walnut2'
 post_top=2.64 if kind in ['cheese','pantry'] else (2.78 if kind=='bread' else 2.89)
 post_h=post_top-.09
 for dx in [-w/2+.06,w/2-.06]:
  for yy in [y-.30,y+.77]:
   box(n+' structural post',(x+dx,yy,(post_top+.09)/2),(.075,.075,post_h) if kind=='cheese' else (.095,.095,post_h),frame,.015)
 box(n+' rear cross beam',(x,y+.77,post_top-.08),(w+.12,.14,.16),frame,.018)
 table(n+' produce counter',x,y-.55,1.02,w,.95,'stone' if kind=='cheese' else 'oak')
 # Each counter has its own joinery and silhouette, independent of canopy color.
 if kind=='fruit':
  for i in range(3):box(n+' horizontal crate counter plank',(x,y-.99,.40+i*.20),(w-.10,.065,.15),'oak3',.014)
  for xx in [x-w/2+.12,x,x+w/2-.12]:box(n+' counter upright',(xx,y-1.03,.64),(.10,.07,.70),'walnut2',.01)
 elif kind=='vegetables':
  for z in [.35,.88]:box(n+' barn frame rail',(x,y-.99,z),(w-.08,.09,.10),'oak3',.012)
  for center in [-.82,.82]:
   for sign in [-1,1]:rod(n+' crossed timber brace',(x+center-.70,y-1.0,.61-sign*.22),(x+center+.70,y-1.0,.61+sign*.22),.042,'walnut2',12)
 elif kind=='bread':
  box(n+' bakery painted counter',(x,y-.55,.60),(w-.09,.83,.76),'cream',.025)
  for i in range(int((w-.28)/.148)+1):box(n+' bakery beadboard seam',(x-w/2+.14+i*.148,y-.977,.60),(.012,.008,.66),'creamPiping',.002)
  box(n+' bakery oak kickboard',(x,y-1.0,.27),(w-.05,.06,.10),'walnut2',.01)
 elif kind=='cheese':
  rounded_slab(n+' rounded dairy cabinet',(x,y-.55,.60),w-.09,.87,.76,.13,'ivory')
  for xx in [x-1.72,x-.86,x,x+.86,x+1.72]:
   box(n+' teal inset panel',(xx,y-.995,.61),(.75,.025,.53),'teal',.035)
   box(n+' small brass drawer handle',(xx,y-1.015,.80),(.22,.025,.025),'brass',.008)
 elif kind=='pantry':
  box(n+' honey kiosk oak front',(x,y-.97,.62),(w-.10,.07,.70),'oak2',.02)
  for xx in [-1.74,-1.16,-.58,0,.58,1.16,1.74]:
   curve(n+' honeycomb front motif',[(x+xx+.23*cos(i*pi/3),y-1.016,.63+.23*sin(i*pi/3))for i in range(6)],.025,'honey',True,True)
 elif kind=='bites':
  for z in [.31,.82]:box(n+' food cart rail',(x,y-.99,z),(w-.10,.08,.08),'sageDark',.012)
  for i in range(int((w-.34)/.29)+1):
   xx=x-w/2+.17+i*.29
   for sign in [-1,1]:rod(n+' diagonal lattice',(xx-.12,y-1.0,.34),(xx+.12,y-1.0,.79),.016,'sageLight',10) if sign==1 else rod(n+' diagonal lattice',(xx+.12,y-1.005,.34),(xx-.12,y-1.005,.79),.016,'sageLight',10)
  for xx in [-w/2+.30,w/2-.30]:
   o=cyl(n+' cart wheel',(x+xx,y-.99,.38),.25,.07,'walnut2',32);o.rotation_euler.x=pi/2
   o=cyl(n+' wheel hub',(x+xx,y-1.035,.38),.07,.015,'brass',24);o.rotation_euler.x=pi/2
   for i in range(8):
    a=i*pi/4;rod(n+' wheel spoke',(x+xx,y-1.034,.38),(x+xx+.20*cos(a),y-1.034,.38+.20*sin(a)),.012,'oak3',8)
 # Roof construction varies: striped cloth, open pergola, gable, vault, tent and scalloped canvas.
 if kind in ['fruit','bites']:
  for i in range(12):
   a=x-w/2+i*w/12;b=x-w/2+(i+1)*w/12
   path=[(y+.87,2.97),(y+.48,2.91),(y+.02,2.75),(y-.46,2.63)]
   ma=(color if i%2==0 else 'paper') if kind=='fruit' else 'pink'
   vs=[(xx,yy,zz-.025*sin(pi*(xx-x+w/2)/w))for xx in [a,b]for yy,zz in path]
   o=mesh(n+' canvas awning',vs,[(j,j+1,5+j,4+j)for j in range(3)],ma);md=o.modifiers.new('Canvas thickness','SOLIDIFY');md.thickness=.008
   vv=[(a,y-.47,2.64),(b,y-.47,2.64)]+[(b-(b-a)*j/8,y-.47,2.51-(.09 if kind=='bites' else .035)*sin(pi*j/8))for j in range(9)]
   mesh(n+' scalloped awning edge',vv,[tuple(range(len(vv)))],ma)
 elif kind=='vegetables':
  for yy in [y-.48,y+.88]:box(n+' pergola beam',(x,yy,2.78),(w+.16,.13,.17),'walnut2',.018)
  for i in range(13):
   xx=x-w/2+i*w/12;box(n+' open pergola roof slat',(xx,y+.20,2.91),(.14,1.55,.12),'oak3',.016)
  for dx in [-w/2+.07,w/2-.07]:
   rod(n+' pergola knee brace',(x+dx,y-.30,2.25),(x+dx*.68,y-.30,2.76),.04,'walnut2',12)
 elif kind=='bread':
  vs=[(x-w/2-.06,y-.51,2.72),(x+w/2+.06,y-.51,2.72),(x-w/2-.06,y+.20,3.23),(x+w/2+.06,y+.20,3.23),(x-w/2-.06,y+.91,2.72),(x+w/2+.06,y+.91,2.72)]
  o=mesh(n+' pitched bakery roof',vs,[(0,1,3,2),(2,3,5,4)],'clay');md=o.modifiers.new('Roof thickness','SOLIDIFY');md.thickness=.055
  for i in range(14):
   xx=x-w/2+i*w/13;curve(n+' roof standing seam',[(xx,y-.51,2.74),(xx,y+.20,3.25),(xx,y+.91,2.74)],.016,'rust',poly=True)
  for dx in [-w/2-.07,w/2+.07]:curve(n+' gable oak fascia',[(x+dx,y-.54,2.72),(x+dx,y+.20,3.24),(x+dx,y+.94,2.72)],.045,'oak3',poly=True)
 elif kind=='cheese':
  pts=[(x-w/2-.05+(w+.10)*i/32,2.65+.45*sin(pi*i/32))for i in range(33)]
  vs=[(xx,yy,zz)for yy in [y-.51,y+.91]for xx,zz in pts]
  o=mesh(n+' curved dairy canopy',vs,[(i,i+1,34+i,33+i)for i in range(32)],'teal');md=o.modifiers.new('Metal canopy thickness','SOLIDIFY');md.thickness=.035
  for yy in [y-.53,y+.93]:curve(n+' curved cream canopy trim',[(xx,yy,zz)for xx,zz in pts],.028,'ivory',poly=True)
 elif kind=='pantry':
  vs=[(x-w/2-.05,y-.52,2.68),(x+w/2+.05,y-.52,2.68),(x+w/2+.05,y+.92,2.68),(x-w/2-.05,y+.92,2.68),(x,y+.20,3.35)]
  o=mesh(n+' peaked cream market tent',vs,[(0,1,4),(1,2,4),(2,3,4),(3,0,4)],'linen');md=o.modifiers.new('Canvas thickness','SOLIDIFY');md.thickness=.012
  for i in range(4):curve(n+' tent stitched rib',[vs[i],vs[4]],.014,'paper',poly=True)
  for a,b in [(0,1),(1,2),(2,3),(3,0)]:rod(n+' tent eave trim',vs[a],vs[b],.028,'honey',12)
  box(n+' cream tent valance',(x,y-.52,2.60),(w+.10,.02,.16),'linen',.01)
 board(n+' name',caption,x,y-.51,2.28,w*.78,.38,color,.16)


# Revision 9: open L-shaped market corner and communal picnic tables.
# Local stall coordinates keep food, signs, canopy and fixtures together.
M['slate']=material('Weathered slate','#58606A',.88)
M['sandstone']=material('Warm civic sandstone','#C8AE88',.90)
M['mortar']=material('Recessed limestone joints','#AE9C81',.92)
M['petal']=material('Violet market flowers','#9A78A9',.86)
collection('01 Rounded town square and paving',offset=(0,0,0))
rounded_slab('Market square stone plinth',(0,0,-.14),13.0,12.8,.32,.4,'walnut2')
rounded_slab('Continuous limestone paving',(0,0,.035),12.85,12.65,.08,.35,'mortar')
for j in range(25):
 for i in range(25):
  x=-6.12+i*.51;y=-6.0+j*.50
  box('Individual sandstone paver',(x,y,.088),(.495,.485,.055),['stone','ivory','linen'][(i+j*7)%3],.012)
# Three stalls face forward across the back; three turn inward along the left edge.
def produce(kind,x,y,z):
 crate(kind+' display crate',x,y,z,.88,.62,.27)
 for row in range(3):
  for col in range(4):fruit('Fresh '+kind,x-.29+col*.19,y-.19+row*.18,z+.06,kind,.084)
 board(kind+' price card',kind.upper()+'  $3',x,y-.34,z+.17,.53,.14,'ivory',.065)

def vegcrate(x,y,z,kind):
 crate(kind+' harvest crate',x,y,z,.88,.63,.26)
 for j in range(2):
  for i in range(4):
   xx=x-.29+i*.19;yy=y-.15+j*.30
   if kind=='CARROTS':
    lathe('Tapered carrot root',(xx,yy,z+.04),[(0,0),(.018,.01),(.063,.18),(.049,.25),(0,.26)],'orange',16)
    for k in [-1,0,1]:curve('Carrot leafy crown',[(xx,yy,z+.28),(xx+k*.04,yy+.02,z+.40),(xx+k*.065,yy+.04,z+.44)],.007,'leaf')
   elif kind=='CABBAGE':
    uv('Cabbage heart',(xx,yy,z+.16),(.105,.105,.12),'sageLight',16,10)
    for k in range(4):
     a=k*pi/2;o=uv('Cupped cabbage leaf',(xx+.06*cos(a),yy+.06*sin(a),z+.13),(.08,.038,.10),'leaf' if k%2 else 'sageLight',12,8);o.rotation_euler.z=a
   else:uv('New potato',(xx,yy,z+.12),(.095,.07,.065),'crustLight',16,8)
 board(kind+' price card',kind+'  $2',x,y-.35,z+.16,.61,.14,'ivory',.059)

def food_bottle(n,x,y,z,ma='ceramic',label='MILK'):
 lathe(n+' shaped bottle',(x,y,z),[(0,0),(.075,0),(.085,.035),(.085,.25),(.045,.31),(.045,.40),(0,.40)],ma,24)
 cyl(n+' screw cap',(x,y,z+.405),.049,.035,'sageDark' if label!='MILK' else 'red',24)
 box(n+' label',(x,y-.083,z+.17),(.12,.008,.13),'paper',.008)
 text(n+' label text',label,(x,y-.089,z+.18),.031,'ink')

def small_berry(n,x,y,z,color='red'):
 uv(n,(x,y,z+.055),(.054,.048,.061),color,16,10)
 if color=='red':
  for i in range(5):
   a=i*2*pi/5;uv(n+' green crown',(x+.03*cos(a),y+.03*sin(a),z+.106),(.025,.012,.005),'leaf',10,6)
  for i in range(5):uv(n+' seed',(x+.043*cos(i*2.4),y+.039*sin(i*2.4),z+.062),(.004,.004,.006),'dough',8,4)

def mushroom(n,x,y,z):
 cyl(n+' stalk',(x,y,z+.07),.027,.14,'paper',12)
 uv(n+' cap',(x,y,z+.15),(.085,.08,.045),'bookCream',16,8)

def corn(n,x,y,z):
 uv(n+' cob',(x,y,z+.075),(.075,.21,.065),'lemon',16,8)
 for i in range(7):
  for j in range(3):uv(n+' kernels',(x+(j-1)*.045,y-.15+i*.05,z+.12),(.024,.03,.025),'honey',10,6)
 for dx in [-.085,.085]:uv(n+' husk',(x+dx,y+.04,z+.05),(.035,.23,.035),'leaf',12,8)

def cheese_wedge(n,x,y,z):
 vs=[(x-.16,y-.14,z),(x+.17,y-.14,z),(x+.01,y+.18,z),(x-.16,y-.14,z+.16),(x+.17,y-.14,z+.16),(x+.01,y+.18,z+.16)]
 o=mesh(n+' cheddar wedge',vs,[(0,2,1),(3,4,5),(0,1,4,3),(1,2,5,4),(2,0,3,5)],'honey');bevel(o,.008,2)
 for dx,dy in [(-.08,-.07),(.07,-.06),(0,.07)]:cyl(n+' cheese pore',(x+dx,y+dy,z+.163),.018,.004,'crustLight',12)

def label_tag(words,x,y,z,width=.78):board('Food variety card',words,x,y,z,width,.14,'ivory',.055)

def fill_stand(kind):
 if kind=='fruit':
  for x in [-1.08,0,1.08]:crate('Orchard mixed produce bin',x,-.22,1.07,.94,.67,.24)
  for i,k in enumerate(['apple','pear','apple','pear','apple','pear']):
   fruit('Orchard '+k,-1.37+(i%3)*.28,-.39+(i//3)*.30,1.12,k,.09)
  for i,k in enumerate(['orange','lemon','orange','lemon']):fruit('Mixed citrus '+k,-.28+(i%2)*.24,-.38+(i//2)*.27,1.12,k,.095)
  uv('Watermelon green half',(.27,-.12,1.23),(.19,.19,.12),'basil',24,12)
  cyl('Watermelon pale rind',(.27,-.12,1.31),.18,.012,'sageLight',32)
  cyl('Watermelon rosy cut flesh',(.27,-.12,1.319),.162,.009,'red',32)
  for i in range(7):
   a=i*2.4;uv('Watermelon dark seed',(.27+.10*cos(a),-.12+.10*sin(a),1.326),(.009,.018,.003),'ink',10,6)
  for xx,ma in [(.83,'red'),(1.22,'navy')]:
   box('Berry paper punnet',(xx,-.37,1.16),(.33,.29,.11),'linen',.02)
   for j in range(6):small_berry('Strawberry' if ma=='red' else 'Blueberry',xx-.095+(j%3)*.09,-.44+(j//3)*.12,1.20,ma)
  for j in range(12):uv('Purple grape bunch',(.87+(j%4)*.085,-.04+(j//4)*.065,1.18+(j%2)*.05),(.05,.048,.05),'grape',12,8)
  for x,t in [(-1.08,'APPLES / PEARS'),(0,'CITRUS / MELON'),(1.08,'BERRIES / GRAPES')]:label_tag(t,x,-.585,1.23)
 elif kind=='vegetables':
  for x in [-1.08,0,1.08]:crate('Vegetable mixed harvest crate',x,-.22,1.07,.94,.67,.24)
  for i in range(4):
   xx=-1.39+i*.16
   lathe('Fresh carrot root',(xx,-.32,1.11),[(0,0),(.02,.01),(.052,.20),(.04,.25),(0,.26)],'orange',16)
   for k in [-1,0,1]:curve('Fresh carrot greens',[(xx,-.32,1.36),(xx+k*.04,-.30,1.47),(xx+k*.06,-.26,1.51)],.006,'leaf')
  for i,ma in enumerate(['red','lemon']):
   xx=-.92+i*.21
   for j in range(4):
    a=j*pi/2;uv('Lobed sweet pepper',(xx+.045*cos(a),-.13+.045*sin(a),1.24),(.063,.06,.10),ma,12,8)
   curve('Pepper curved stalk',[(xx,-.13,1.33),(xx,-.13,1.38),(xx+.025,-.13,1.40)],.012,'basil')
  for xx in [-.24,.18]:
   uv('Aubergine',(xx,-.39,1.20),(.08,.19,.08),'grape',20,10)
   uv('Aubergine green calyx',(xx,-.22,1.21),(.066,.045,.055),'basil',12,8)
  for xx in [-.20,.19]:
   rod('Broccoli stalk',(xx,-.02,1.13),(xx,-.02,1.29),.035,'sageLight',12)
   for j in range(7):
    a=j*2.4;uv('Broccoli floret',(xx+.075*cos(a),-.02+.065*sin(a),1.30+(j%2)*.025),(.07,.065,.065),'basil',12,8)
  corn('Sweet corn',.87,-.25,1.13)
  for j in range(4):mushroom('Farm mushroom',1.15+(j%2)*.17,-.38+(j//2)*.25,1.13)
  for x,t in [(-1.08,'CARROTS / PEPPERS'),(0,'GREENS / AUBERGINE'),(1.08,'CORN / MUSHROOMS')]:label_tag(t,x,-.585,1.23)
 elif kind=='bread':
  for x in [-1.08,0,1.08]:crate('Mixed bakery tray',x,-.22,1.07,.94,.67,.18)
  for i in range(3):baguette('Crusty baguette',-1.38+i*.18,-.25,1.13,.56,.12)
  loaf('Round seeded sourdough',-.82,-.11,1.15,.41)
  for j in range(10):uv('Sourdough sesame seed',(-.96+(j%5)*.06,-.19+(j//5)*.09,1.39),(.012,.006,.003),'paper',8,4)
  for xx in [-.24,.23]:
   curve('Twisted pretzel',[(xx,-.40,1.17),(xx-.15,-.17,1.17),(xx-.10,.02,1.17),(xx+.08,-.02,1.17),(xx-.08,-.30,1.18),(xx+.10,-.43,1.17),(xx+.17,-.25,1.17),(xx,-.09,1.17)],.038,'crust')
   for k in range(5):box('Pretzel coarse salt',(xx-.10+k*.05,-.15,1.21),(.014,.012,.009),'paper',.002)
  for j in range(4):
   xx=.85+(j%2)*.40;yy=-.39+(j//2)*.32
   cyl('Fruit tart pastry shell',(xx,yy,1.17),.13,.08,'crustLight',24)
   cyl('Tart vanilla cream',(xx,yy,1.214),.112,.014,'dough',24)
   for k in range(3):small_berry('Tart berry',xx+.05*cos(k*2*pi/3),yy+.05*sin(k*2*pi/3),1.22,'red' if j%2 else 'navy')
  for x,t in [(-1.08,'SOURDOUGH / BATONS'),(0,'SOFT PRETZELS'),(1.08,'FRESH BERRY TARTS')]:label_tag(t,x,-.585,1.22)
 elif kind=='cheese':
  for x in [-1.08,0,1.08]:rounded_slab('Dairy serving board',(x,-.22,1.09),.96,.65,.045,.08,'walnut2')
  cyl('Brie whole wheel',(-1.25,-.20,1.20),.22,.17,'paper',32)
  cheese_wedge('Aged cheddar',-.86,-.33,1.12)
  box('Blue cheese block',(-.23,-.21,1.23),(.28,.29,.22),'dough',.02)
  for j in range(7):curve('Blue cheese marbling',[(-.34+j*.036,-.30,1.344),(-.32+j*.036,-.22,1.344),(-.35+j*.036,-.12,1.344)],.005,'sageDark')
  for yy in [-.39,-.08]:
   box('Butter paper parcel',(.24,yy,1.18),(.27,.23,.12),'paper',.012)
   box('Butter gold wrapper band',(.24,yy,1.246),(.09,.23,.008),'honey',.002)
  for xx in [.85,1.15]:food_bottle('Farm milk bottle',xx,-.08,1.12)
  for xx in [.84,1.20]:
   cyl('Yogurt tub',(xx,-.42,1.20),.085,.16,'ceramic',24)
   cyl('Yogurt berry lid',(xx,-.42,1.285),.09,.018,'pink' if xx<1 else 'teal',24)
  for x,t in [(-1.08,'BRIE / CHEDDAR'),(0,'BLUE / FARM BUTTER'),(1.08,'MILK / YOGURT')]:label_tag(t,x,-.585,1.18)
 elif kind=='pantry':
  for x in [-1.08,0,1.08]:rounded_slab('Pantry display tray',(x,-.22,1.09),.95,.65,.035,.06,'oak3')
  for xx,ma,word in [(-1.31,'honey','HONEY'),(-.98,'red','JAM'),(-.65,'basil','PICKLES')]:
   for yy in [-.37,-.04]:
    jar(word+' preserve jar',xx,yy,1.11,ma,.10,.25 if word!='PICKLES' else .31)
    text(word+' jar type',word,(xx,yy-.105,1.24),.026,'ink')
  for xx,ma,word in [(-.20,'basil','OIL'),(.13,'tomato','SAUCE')]:food_bottle('Pantry '+word,xx,-.10,1.11,ma,word)
  for xx in [-.19,.16]:
   box('Paper spice sachet',(xx,-.44,1.22),(.22,.10,.22),'linen',.02)
   text('Spice label','SPICE',(xx,-.496,1.24),.034,'ink')
  box('Mixed farm egg carton',(1.08,-.20,1.15),(.76,.53,.09),'linen',.025)
  for j in range(8):uv('Mixed brown and white egg',(.81+(j%4)*.18,-.35+(j//4)*.25,1.25),(.068,.064,.089),'dough' if j%2 else 'paper',16,10)
  for x,t in [(-1.08,'HONEY / JAM / PICKLES'),(0,'OIL / SAUCE / SPICES'),(1.08,'FREE RANGE EGGS')]:label_tag(t,x,-.585,1.18)
 elif kind=='bites':
  steamer('Market dumpling bamboo steamer',-1.08,-.22,1.10,.35,True)
  rounded_slab('Skewer serving tray',(0,-.22,1.10),.93,.66,.045,.07,'darkPlate')
  for xx in [-.27,0,.27]:
   rod('Bamboo kebab skewer',(xx,-.54,1.17),(xx,.11,1.17),.012,'bambooDark',10)
   for j,ma in enumerate(['beefCrust','tomato','cucumber','beefCrust']):box('Grilled skewer bite',(xx,-.35+j*.12,1.20),(.16,.10,.11),ma,.025)
  rounded_slab('Samosa serving tray',(1.08,-.22,1.10),.94,.65,.045,.07,'ceramic')
  for j in range(4):
   xx=.85+(j%2)*.40;yy=-.37+(j//2)*.29
   v=[(xx-.13,yy-.09,1.13),(xx+.13,yy-.09,1.13),(xx,yy+.12,1.13),(xx,yy,1.35)]
   o=mesh('Golden triangular samosa',v,[(0,2,1),(0,1,3),(1,2,3),(2,0,3)],'crustLight');bevel(o,.018,3)
  for x,t in [(-1.08,'STEAMED DUMPLINGS'),(0,'GRILLED SKEWERS'),(1.08,'GOLDEN SAMOSAS')]:label_tag(t,x,-.585,1.17)
 # Stacked reserve crates and shopper packaging, shared by all six vendors.
 box('Stall lower storage shelf',(0,.12,.34),(2.85,.63,.065),'oak2',.013)
 for x in [-1.1,1.1]:crate('Reserve wooden crate',x,.10,.39,.63,.53,.36)
 box('Kraft shopping bag',(1.34,.48,1.33),(.28,.20,.48),'linen',.015)
 for y in [.40,.56]:curve('Paper bag loop handle',[(1.24,y,1.55),(1.25,y,1.68),(1.42,y,1.68),(1.44,y,1.55)],.009,'walnut2')

def decorate_stand(kind,w):
 px=w/2-.06
 # Canvas shopping tote hangs directly from the right-hand post.
 box('Reusable canvas tote',(px,-.42,.98),(.29,.12,.36),'linen',.055)
 for yy in [-.48,-.36]:curve('Tote attached fabric handles',[(px-.11,yy,1.13),(px-.06,yy,1.36),(px+.06,yy,1.36),(px+.11,yy,1.13)],.012,'sageDark')
 text('Tote leaf motif','LOCAL',(px,-.487,1.01),.054,'sageDark')
 # A small string of warm bulbs tucked underneath each canvas canopy.
 curve('Awning festoon cable',[(-w/2+.14,-.30,2.58),(0,-.30,2.46),(w/2-.14,-.30,2.58)],.007,'walnut2')
 for i in range(9):
  xx=-w/2+.30+i*(w-.60)/8;zz=2.46+.12*(xx/(w/2-.14))**2
  rod('Festoon bulb socket',(xx,-.30,zz),(xx,-.30,zz-.06),.018,'brass',10)
  uv('Warm festoon bulb',(xx,-.30,zz-.09),(.036,.036,.045),'led',12,8)

# Four 4.9m carts, two along each edge. Foods retain their original proportions.
def expanded_cart_stock(kind):
 for xx in [-1.98,1.98]:
  rounded_slab('Expanded counter end display',(xx,-.22,1.09),.65,.65,.035,.06,'oak3' if kind!='cheese' else 'walnut2')
 if kind=='bread':
  for j in range(4):
   xx=-2.14+(j%2)*.29;yy=-.38+(j//2)*.31
   curve('Golden bakery bagel',[(xx+.085*cos(i*2*pi/32),yy+.085*sin(i*2*pi/32),1.17)for i in range(32)],.039,'crustLight',True,True)
   for k in range(7):
    a=k*2.4;uv('Bagel sesame',(xx+.08*cos(a),yy+.08*sin(a),1.205),(.011,.005,.003),'paper',8,4)
  for j in range(4):
   xx=1.83+(j%2)*.30;yy=-.38+(j//2)*.31
   cyl('Cinnamon bun',(xx,yy,1.18),.125,.12,'crustLight',24)
   curve('Cinnamon spiral',[(xx+(.015+i*.0014)*cos(i*.26),yy+(.015+i*.0014)*sin(i*.26),1.244)for i in range(70)],.008,'chocolate',poly=True)
  label_tag('SESAME BAGELS',-1.98,-.585,1.18,.61);label_tag('CINNAMON BUNS',1.98,-.585,1.18,.61)
 elif kind=='cheese':
  for j in range(3):
   xx=-2.15+(j%2)*.30;yy=-.38+(j//2)*.31
   cyl('Goat cheese round',(xx,yy,1.19),.105,.13,'paper',24)
   for k in range(6):
    a=k*2.4;uv('Goat cheese herb fleck',(xx+.06*cos(a),yy+.06*sin(a),1.259),(.014,.007,.003),'basil',8,4)
  cyl('Waxed gouda wheel',(1.98,-.16,1.24),.24,.24,'red',32)
  cyl('Gouda cut top',(1.98,-.16,1.365),.21,.014,'honey',32)
  cheese_wedge('Gouda sample',1.98,-.43,1.12)
  label_tag('HERBED GOAT',-1.98,-.585,1.18,.61);label_tag('AGED GOUDA',1.98,-.585,1.18,.61)
 elif kind=='pantry':
  for xx in [-2.15,-1.82]:
   box('Kraft flour bag',(xx,-.15,1.36),(.28,.27,.48),'linen',.035)
   box('Flour bag folded seal',(xx,-.15,1.615),(.28,.10,.04),'bookCream',.008)
   text('Flour bag label','FLOUR',(xx,-.291,1.39),.043,'ink')
  for j,ma in enumerate(['rust','pasta','bookInk','honey']):
   xx=1.82+(j%2)*.32;yy=-.38+(j//2)*.31
   jar('Dry beans and lentils jar',xx,yy,1.11,ma,.11,.28)
  label_tag('STONEGROUND',-1.98,-.585,1.18,.61);label_tag('BEANS / LENTILS',1.98,-.585,1.18,.61)
 elif kind=='bites':
  for j in range(4):
   xx=-2.14+(j%2)*.30;yy=-.37+(j//2)*.29
   box('Spring roll filling',(xx,yy,1.18),(.19,.08,.045),'leaf',.02)
   o=uv('Golden spring roll',(xx,yy,1.20),(.12,.055,.055),'crustLight',20,10);o.rotation_euler.z=.24 if j%2 else -.18
  for xx,ma in [(1.81,'tomato'),(2.14,'basil')]:
   bowl('Street food dipping bowl',xx,-.38,1.11,.12,'ceramic')
   cyl('Chilli or herb dipping sauce',(xx,-.38,1.19),.10,.01,ma,24)
  for j in range(3):bao('Sweet steamed bun',1.74+j*.23,-.06,1.12,.10)
  label_tag('SPRING ROLLS',-1.98,-.585,1.17,.61);label_tag('BUNS / DIPS',1.98,-.585,1.17,.61)

stands=[('02 Farmhouse dairy cart',-1.55,4.95,'teal','FARMHOUSE DAIRY','cheese',0),
 ('03 Village bakery cart',3.60,4.95,'clay','THE VILLAGE BAKER','bread',0),
 ('04 Farm pantry and eggs cart',-5.4,1.60,'pink','FARM PANTRY','pantry',pi/2),
 ('05 Ready to eat market bites cart',-5.4,-3.55,'sage','MARKET BITES','bites',pi/2)]
for n,x,y,col,caption,kind,angle in stands:
 collection(n,offset=((.7 if x>0 else -.7),-1.2,.3),start=25,end=70)
 def make(col=col,caption=caption,kind=kind):
  stall(caption,0,0,4.90,col,caption,kind)
  local_group(caption+' counter merchandise',(0,-.30,0),0,lambda:fill_stand(kind))
  local_group(caption+' expanded end displays',(0,-.30,0),0,lambda:expanded_cart_stock(kind))
  decorate_stand(kind,4.90)
 local_group(n+' complete trading stand',(x,y,0),angle,make)

collection('06 Market trees and planters',offset=(0,0,.5))
for x,y in [(-5.35,5.25)]:
 rounded_slab('Town tree planter',(x,y,.31),1.10,1.10,.50,.12,'sandstone')
 rounded_slab('Tree planter soil',(x,y,.57),.91,.91,.035,.1,'walnut')
 rod('Tree trunk',(x,y,.57),(x,y,3.70),.11,'walnut2')
 for j in range(9):
  a=j*2.4;rr=.47 if j<6 else .24;xx=x+rr*cos(a);yy=y+rr*sin(a);zz=3.5+(j%3)*.36
  rod('Branch',(x,y,2.65),(xx,yy,zz),.043,'walnut2')
  uv('Rounded tree foliage cluster',(xx,yy,zz),(.48,.43,.52),['leaf','sageDark','sageLight'][j%3],16,10)
  for k in range(7):
   a2=k*2.4;uv('Leaf cluster detail',(xx+.34*cos(a2),yy+.32*sin(a2),zz+.18*sin(k)),(.14,.12,.16),'leaf' if k%2 else 'sageLight',12,8)

collection('07 Picnic tables and attached benches',offset=(0,-1,.25))
def picnic_table(n,x,y,menu):
 def make():
  # Four tabletop boards; benches on both sides, tied into the A-frame.
  for i in range(4):
   rounded_slab(n+' tabletop plank',(0,-.39+i*.26,.91),2.78,.245,.085,.045,'oak3')
  for side in [-1,1]:
   for i in range(2):
    rounded_slab(n+' bench seat plank',(0,side*.92+(i-.5)*.19,.52),2.78,.18,.085,.035,'oak')
  for xx in [-.92,.92]:
   for side in [-1,1]:
    rod(n+' splayed table leg',(xx,side*.64,.14),(xx,side*.32,.86),.067,'walnut2')
    rod(n+' bench support leg',(xx,side*.94,.14),(xx,side*.88,.47),.055,'walnut2')
   box(n+' cross member',(xx,0,.40),(.12,2.12,.12),'walnut2',.018)
  box(n+' longitudinal stretcher',(0,0,.35),(2.05,.12,.13),'walnut2',.015)
  # Four distinct lunches, all supported by the tabletop at z=.955.
  def juice_cup(xx,yy):
   lathe(n+' ceramic juice cup',(xx,yy,.955),[(0,0),(.066,0),(.078,.16),(.070,.17),(.062,.025),(0,.025)],'ceramic',24)
   cyl(n+' orange juice surface',(xx,yy,1.09),.065,.006,'orange',24)
  def pitcher(xx,yy):
   lathe(n+' lemonade pitcher',(xx,yy,.955),[(0,0),(.11,0),(.14,.04),(.14,.29),(.12,.40),(.11,.40),(.12,.05),(0,.05)],'ceramic',32)
   curve(n+' pitcher handle',[(xx+.12,yy,1.27),(xx+.24,yy,1.27),(xx+.25,yy,1.05),(xx+.13,yy,1.02)],.022,'ceramic')
   cyl(n+' lemonade surface',(xx,yy,1.32),.11,.008,'lemon',24)
  if menu==1:
   rounded_slab(n+' cheese board',(.22,0,.979),1.05,.60,.045,.09,'walnut2')
   loaf(n+' sourdough',.46,.04,1.01,.48)
   box(n+' sliced farmhouse cheese',(-.10,-.08,1.07),(.24,.22,.12),'dough',.025)
   for j in range(8):uv(n+' red grapes',(-.10+(j%3)*.065,.12+(j//3)*.055,1.04+(j%2)*.035),(.042,.04,.04),'grape',12,8)
   pitcher(-.83,.17);juice_cup(-.80,-.24);juice_cup(.98,-.18)
  elif menu==2:
   bowl(n+' shared salad',.54,.04,.955,.34,'sageDark')
   for j in range(15):
    a=j*2.4;rr=.21*((j%4+1)/4);xx=.54+rr*cos(a);yy=.04+rr*sin(a)
    uv(n+' salad lettuce',(xx,yy,1.055),(.09,.06,.03),'basil' if j%2 else 'sageLight',12,8)
    if j%3==0:uv(n+' cherry tomato',(xx,yy,1.09),(.045,.045,.04),'tomato',12,8)
   for yy in [-.22,.20]:
    plate(n+' sandwich plate',-.40,yy,.955,.23)
    for z,ma,depth in [(1.01,'dough',.06),(1.055,'basil',.022),(1.08,'tomato',.025),(1.11,'dough',.06)]:
     rounded_slab(n+' sandwich layer',(-.40,yy,z),.31,.25,depth,.05,ma)
   juice_cup(1.05,.20);juice_cup(-1.01,-.18)
  elif menu==3:
   rounded_slab(n+' pastry serving tray',(.15,0,.975),.94,.59,.04,.08,'ceramic')
   for j in range(4):
    xx=-.10+(j%2)*.40;yy=-.15+(j//2)*.30
    lathe(n+' cupcake paper',(xx,yy,1.00),[(0,0),(.085,0),(.105,.12),(0,.12)],'linen',24)
    uv(n+' cupcake icing',(xx,yy,1.13),(.105,.105,.055),'paper',20,10)
    uv(n+' berry topping',(xx,yy,1.19),(.04,.035,.04),'red',12,8)
   mug(n+' coffee cup',-.85,.17,.955,.08,.15);mug(n+' second coffee cup',.97,-.16,.955,.08,.15)
   box(n+' pastry paper bag',(-.80,-.19,1.03),(.32,.24,.15),'linen',.035)
  else:
   lathe(n+' open picnic basket',(.22,.02,.955),[(0,0),(.31,0),(.36,.19),(.34,.20),(.29,.04),(0,.04)],'bamboo',32,scale=(1.35,.84,1))
   for z in [.985,1.03,1.075,1.12]:
    curve(n+' woven basket band',[(.22+.47*cos(j*2*pi/32),.02+.28*sin(j*2*pi/32),z)for j in range(32)],.008,'bambooDark',True,True)
   for j,k in enumerate(['apple','orange','pear','apple','lemon']):fruit(n+' picnic '+k,-.02+(j%3)*.21,-.08+(j//3)*.19,1.02,k,.095)
   curve(n+' basket carrying handle',[(-.24,.02,1.12),(-.08,.02,1.48),(.52,.02,1.48),(.68,.02,1.12)],.018,'bambooDark')
   pitcher(-.85,.14);juice_cup(-.82,-.23)
   jar(n+' strawberry preserve',1.02,.14,.955,'red',.085,.22)
  box(n+' folded napkins',(-.30,-.35,.97),(.22,.18,.025),'paper',.006)
 picnic=local_group(n,(x,y,0),0,make)
 picnic.scale=(1.12,1.12,1.12)
 # Keep feet resting on the same paving after uniform furniture scaling.
 picnic.location.z=-.016
for i,(x,y) in enumerate([(-1.6,1.6),(2.4,1.6),(-1.6,-2.6),(2.4,-2.6)],1):picnic_table('Picnic table '+str(i),x,y,i)

collection('08 Corner market bunting',offset=(0,0,1))
# The bunting follows the L shape above the trading edge, leaving the picnic area open.
for x,y in [(-4.15,-6.03),(-4.15,3.93),(6.17,3.93)]:rod('Bunting timber mast',(x,y,.13),(x,y,3.55),.04,'walnut2')
for a,b in [((-4.15,-6.03),(-4.15,3.93)),((-4.15,3.93),(6.17,3.93))]:
 mid=((a[0]+b[0])/2,(a[1]+b[1])/2,3.22)
 curve('Sagging bunting cord',[(*a,3.55),mid,(*b,3.55)],.009,'walnut2')
 for i in range(23):
  t=(i+.2)/23;tt=t+.024;x=a[0]+(b[0]-a[0])*t;y=a[1]+(b[1]-a[1])*t
  xx=a[0]+(b[0]-a[0])*tt;yy=a[1]+(b[1]-a[1])*tt;z=3.22+.33*(2*t-1)**2
  mesh('Canvas festival pennant',[(x,y,z),(xx,yy,z),((x+xx)/2,(y+yy)/2,z-.25)],[(0,1,2)],['sage','clay','honey','paper','pink'][i%5])

bpy.context.scene['concept_reference']='Proposal 20 revised to an open corner market with picnic seating'
bpy.context.scene['stall_count']=4
bpy.context.scene['cart_width_m']=4.9
bpy.context.scene['stalls_per_edge']=2
bpy.context.scene['picnic_table_count']=4
bpy.context.scene['layout']='Four enlarged carts, two per edge; enlarged picnic seating; no civic building'
bpy.context.scene['platform_size_m']=[13.0,12.8]
bpy.context.scene['previous_platform_size_m']=[13.4,14.2]
finish('farmers-market','Noms — Corner Farmers Market',18.2,(0,0,1.0),revision=9,camera_position=(10,-16,13),render_threads=6,samples=32)
