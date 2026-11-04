collection('01 Rounded night market foundation',offset=(0,0,0))
rounded_slab('Night market walnut foundation',(0,0,-.13),8.9,7.2,.29,.30,'walnut')
rounded_slab('Slate market paving',(0,0,.04),8.72,7.02,.065,.24,'navy')
for j in range(14):
 for i in range(16):box('Slate market paver',(-4.08+i*.54,-3.23+j*.49,.084),(.527,.477,.025),['navy','charcoal','steelDark'][(i+j*2)%3],.008)
collection('02 Market backdrop and timber screens',offset=(0,-.5,.15))
box('Low rear market wall',(0,3.34,1.06),(8.66,.18,1.96),'navy',.035)
for i in range(44):box('Timber screen slat',(-4.25+i*.196,3.20,1.38),(.10,.065,2.54),'walnut2',.013)
for x in [-4.22,4.22]:box('Tall festoon support pole',(x,.64,1.85),(.12,.12,3.55),'walnut2',.015)

def kiosk(n,x,color,label):
 y=2.26;w=2.48
 box(n+' counter body',(x,y,.67),(w,.83,1.14),color,.04)
 for i in range(20):box(n+' facade timber strip',(x-w/2+.08+i*.122,y-.428,.65),(.075,.035,.96),'walnut2',.011)
 box(n+' stone service ledge',(x,y-.07,1.28),(w+.13,1.02,.10),'stone',.028)
 for dx in [-1.18,1.18]:box(n+' canopy upright',(x+dx,y+.28,1.66),(.09,.09,3.1),'walnut2',.012)
 box(n+' menu header',(x,y+.25,2.64),(2.40,.12,.57),color,.026)
 text(n+' sign',label,(x,y+.181,2.69),.19,'paper')
 text(n+' caption','MADE TO SHARE',(x,y+.18,2.47),.070,'paper')
 # Low pitched canvas shade, with stitched folded edge.
 verts=[(x-1.32,y-.59,2.97),(x+1.32,y-.59,2.97),(x+1.32,y+.71,3.28),(x-1.32,y+.71,3.28)]
 o=mesh(n+' canvas roof',verts,[(0,1,2,3)],color);m=o.modifiers.new('Canvas thickness','SOLIDIFY');m.thickness=.025
 for xx in [x-1.32,x+1.32]:rod(n+' canopy edge',(xx,y-.60,2.97),(xx,y+.71,3.28),.016,'walnut2')
 for i in range(7):box(n+' short cloth valance',(x-1.13+i*.377,y-.60,2.87),(.365,.018,.20),color,.035)
 area(n+' under canopy warm light',(x,y,2.67),30,.65,(1,.66,.34),(x,y-.1,1.15))

collection('03 Steamed bun stall')
kiosk('Bun kitchen',-2.67,'clay','STEAM  /  BAO')
for x,y,z,r in [(-3.40,2.18,1.33,.26),(-2.77,2.18,1.33,.26),(-2.12,2.18,1.33,.26)]:steamer('Fresh bao bamboo steamer',x,y,z,r)
steamer('Stacked empty bamboo steamer',-3.40,2.56,1.33,.24,False)
steamer('Stacked empty bamboo steamer',-3.40,2.56,1.47,.24,False)
for xx in [-2.70,-2.37,-2.04]:
 plate('Bun tasting plate',xx,1.85,1.33,.13);bao('Single bao',xx,1.85,1.354,.088)
box('Bun chalk menu',(-3.74,2.50,1.69),(.28,.07,.61),'sageDark',.012)
text('Bun menu','BAO\n\n03',(-3.74,2.455,1.84),.073,'paper')

collection('04 Noodle and wok stall')
kiosk('Noodle kitchen',0,'teal','NOODLES')
for x in [-.76,-.18,.40]:noodle_bowl('Hot noodle bowl',x,2.02,1.33,.20)
box('Single wok burner',(.78,2.53,1.38),(.46,.38,.10),'steelDark',.025)
torus('Burner ring',(.78,2.53,1.45),.16,.014,'black',steps=40)
lathe('Round bottom wok',(.78,2.53,1.45),[(0,0),(.12,.025),(.24,.17),(.25,.20),(.24,.204),(.21,.16),(.10,.045),(0,.02)],'black',64)
rod('Wok steel handle',(.99,2.53,1.63),(1.16,2.53,1.68),.018,'steel')
rod('Wok walnut grip',(1.16,2.53,1.68),(1.30,2.53,1.72),.028,'walnut2')
for i in range(7):curve('Wok noodles',[(.64+j*.042,2.49+(i-3)*.027+.008*sin(j+i),1.56+.005*cos(j))for j in range(6)],.006,'pasta')
jug('Noodle broth pitcher',-.76,2.58,1.33)
for xx in [-.39,-.13]:jar('Chilli condiment',xx,2.60,1.33,'red',.067,.18)

collection('05 Sweets and tea stall')
kiosk('Sweet kitchen',2.67,'sageDark','SWEETS  /  TEA')
for x in [1.83,2.48]:
 rounded_slab('Sweet skewers serving board',(x,2.03,1.35),.56,.55,.025,.045,'walnut2')
 for i in range(3):
  xx=x-.17+i*.17;rod('Bamboo dessert skewer',(xx,1.79,1.40),(xx,2.28,1.40),.004,'bamboo',10)
  for j,ma in enumerate(['pink','paper','sageLight']):uv('Soft round rice sweet',(xx,1.94+j*.12,1.42),(.059,.058,.053),ma,24,12)
for x in [3.14,3.45]:
 lathe('Takeaway iced tea cup',(x,2.01,1.34),[(0,0),(.065,0),(.085,.26),(.077,.26),(.059,.012),(0,.012)],'glass',40)
 cyl('Milk tea',(x,2.01,1.46),.068,.21,'linen',40)
 for i in range(8):
  a=i*2.4;uv('Tapioca pearl',(x+.042*cos(a),2.01+.042*sin(a),1.36+.018*(i%2)),(.013,.013,.013),'chocolate',12,8)
 cyl('Tea cup sealed lid',(x,2.01,1.607),.09,.012,'ivory',40)
 rod('Tea drinking straw',(x+.025,2.01,1.46),(x+.040,2.01,1.77),.008,'clay',12)
lathe('Tea brewing pot',(2.88,2.58,1.33),[(0,0),(.13,0),(.16,.10),(.13,.24),(.07,.27),(0,.27)],'teal',48)
curve('Teapot handle',[(3.02,2.58,1.54),(3.15,2.58,1.55),(3.17,2.58,1.40),(3.02,2.58,1.38)],.017,'teal')
curve('Teapot spout',[(2.74,2.58,1.40),(2.62,2.58,1.48),(2.61,2.58,1.60)],.023,'teal')

collection('06 Shared night market dining')
T=table('Communal street dining table',-.64,-1.49,.88,3.52,1.07,'walnut2')
for y in [-2.36,-.63]:
 rounded_slab('Street dining bench',(-.64,y,.48),3.48,.37,.085,.055,'oak')
 for x in [-1.98,.70]:
  for dy in [-.12,.12]:rod('Bench leg',(x,y+dy,.10),(x,y+dy,.44),.036,'black')
for x,y in [(-1.83,-1.63),(-.73,-1.32),(.41,-1.63)]:noodle_bowl('Shared noodle meal',x,y,T,.18)
steamer('Shared bao basket',-.12,-1.42,T,.24)
for x,y in [(-1.78,-1.19),(.54,-1.14)]:
 glass('Cold tea tumbler',x,y,T,.061,.20,'tea');rod('Tea straw',(x,y,T+.07),(x+.01,y,T+.29),.005,'paper',10)
box('Table napkin dispenser',(-1.15,-1.48,T+.12),(.17,.16,.22),'steelDark',.015)
box('Napkins exposed',(-1.15,-1.48,T+.24),(.14,.045,.06),'paper',.006)

collection('07 Arrival menu and stacked tableware')
for x in [2.69,3.27]:rod('Standing menu feet',(x,-1.91,.10),(x,-1.65,1.38),.027,'walnut2')
board('Night market welcome','NOMS\nNIGHT MARKET',2.98,-1.67,1.0,.83,1.02,'navy',.10)
text('Night market invitation','GOOD FOOD\nGOOD COMPANY',(2.98,-1.716,.72),.075,'honey')
crate('Reusable bowl crate',3.38,.44,.09,1.02,.64,.37)
for x in [3.16,3.60]:
 for i in range(4):bowl('Stacked clean bowl',x,.44,.16+i*.052,.17,'ceramic')

collection('08 Paper lanterns and festoon cables')
curve('Front lantern cable',[(-4.22,.64,3.63),(0,.64,3.27),(4.22,.64,3.63)],.012,'black')
for i in range(9):
 x=-3.85+i*.96;z=3.27+.36*(x/4.22)**2
 rod('Lantern short hanger',(x,.64,z),(x,.64,z-.12),.007,'black',10)
 r=.18 if i%2 else .22;center=z-.12-r
 lathe('Rounded ribbed paper lantern',(x,.64,center),[(.07,-r),(.15,-r*.7),(r,0),(.15,r*.7),(.07,r)],'honey' if i%2 else 'clay',48)
 for j in range(9):
  t=-pi/2+.15+j*(pi-.3)/8;rr=r*cos(t);torus('Paper lantern bamboo rib',(x,.64,center+r*sin(t)),rr,.003,'bambooDark',steps=40)
 for zz in [center-r,center+r]:cyl('Lantern metal cap',(x,.64,zz),.075,.018,'black',24)
 # Real lights keep shadows consistent with moving geometry in native Blender.
 d=bpy.data.lights.new('Lantern warm glow','POINT');d.energy=10;d.color=(1,.46,.16);d.shadow_soft_size=.24;o=bpy.data.objects.new('Lantern warm glow',d);COL.objects.link(o);o.parent=ROOT;o.location=(x,.42,center-.1)
finish('night-market','Noms — Night Market',12.8,(0,0,1.12),night=True)
