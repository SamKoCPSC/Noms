"""Corner campsite reconstruction: two storeys, river, wooden lounge, folded pines."""
from pathlib import Path
HERE=Path(__file__).resolve().parent
for module in ['diorama_common.py','scene_details.py','camp_details.py']:exec(compile((HERE/module).read_text(),str(HERE/module),'exec'))
for k,h in {'cedar':'#9A6744','cedarLight':'#B78052','roof':'#3C504A','river':'#6CABB1','riverLight':'#A7CED0','windowBlue':'#B8D5D8','pineFold':'#718D46','pineFoldLight':'#8CA451','sausage':'#985238','mustard':'#D2A335','ketchup':'#A93F2B'}.items():M[k]=material(k,h,.84)
M['river'].node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.36
collection('01 Square woodland plinth',offset=(0,0,0))
rounded_slab('Earth diorama base',(0,0,-.13),8.6,7.4,.30,.16,'earth')
# Grass banks and water occupy separate surfaces; the river is recessed.
ys=[-3.58+i*7.16/36 for i in range(37)]
def rivercenter(y):return 2.93+.33*sin(y*.82)
left=[(rivercenter(y)-.60,y,.065)for y in ys];right=[(rivercenter(y)+.60,y,.065)for y in ys]
mesh('Broad meadow left bank',[(-4.17,-3.58,.065)]+left+ [(-4.17,3.58,.065)],[tuple(range(len(left)+2))],'grass')
mesh('Narrow far river bank',right+[(4.17,3.58,.065),(4.17,-3.58,.065)],[tuple(reversed(range(len(right)+2)))],'grassDark')
collection('02 Winding river and bank stones',offset=(0,0,0))
v=[]
for y in ys:v.extend([(rivercenter(y)-.60,y,.027),(rivercenter(y)+.60,y,.027)])
mesh('Continuous blue river',v,[(2*i,2*i+1,2*i+3,2*i+2)for i in range(len(ys)-1)],'river')
for bank in [left,right]:
 for i in range(len(bank)-1):
  a,b=bank[i],bank[i+1];mesh('Exposed earthen river bank',[a,b,(b[0],b[1],.02),(a[0],a[1],.02)],[(0,1,2,3)],'earth')
for i in range(20):
 y=-3.4+i*.35;x=rivercenter(y)+(-.62 if i%2 else .62)
 little_rock('River bank stone',x,y,.03,.09+.04*(i%3))
for i in range(18):
 y=-3.15+i*.36;x=rivercenter(y)+.22*sin(i*2.4)
 curve('Quiet river ripple',[(x-.18,y,.031),(x,y+.018,.031),(x+.16,y,.031)],.006,'riverLight')
collection('03 Two-storey cedar cabin',offset=(0,-.5,.2))
CX,CY=-1.85,1.76;W,D=3.20,2.60;FRONT=CY-D/2
rounded_slab('Cabin stone footing',(CX,CY,.18),3.40,2.78,.24,.06,'rock')
box('Cabin enclosed body',(CX,CY,1.94),(W,D,3.30),'cedar',.015)
for j in range(23):
 z=.35+j*.141
 box('Front cedar clapboard',(CX,FRONT-.021,z),(W,.045,.127),'cedarLight' if j%4==0 else 'cedar',.004)
 box('Right cedar clapboard',(CX+W/2+.02,CY,z),(.045,D,.127),'cedarLight' if j%4==0 else 'cedar',.004)
for x in [CX-W/2+.035,CX+W/2-.035]:box('Cabin front corner trim',(x,FRONT-.053,1.95),(.10,.08,3.35),'ivory',.006)
box('Second floor rim joist',(CX,FRONT-.07,2.06),(W,.12,.12),'walnut2',.006)
# Roof ridge runs front-to-back; actual triangular gable and separate roof planes.
mesh('Front triangular cedar gable',[(CX-W/2,FRONT-.025,3.59),(CX+W/2,FRONT-.025,3.59),(CX,FRONT-.025,4.58)],[(0,1,2)],'cedarLight')
for side in [-1,1]:
 x=CX+side*(W/2+.20)
 o=mesh('Green standing seam roof',[(CX,FRONT-.25,4.64),(x,FRONT-.25,3.54),(x,CY+D/2+.22,3.54),(CX,CY+D/2+.22,4.64)],[(0,1,2,3)],'roof');md=o.modifiers.new('Roof sheet thickness','SOLIDIFY');md.thickness=.065
 for i in range(10):
  y=FRONT-.23+i*(D+.45)/9;rod('Roof standing seam',(CX,y,4.67),(x,y,3.58),.014,'sageDark',8)
 rod('Cream gable fascia',(CX,FRONT-.28,4.62),(x,FRONT-.28,3.53),.045,'ivory',8)
box('Cabin chimney',(CX-.73,CY+.40,4.35),(.35,.40,.95),'rock',.025);box('Chimney cap',(CX-.73,CY+.40,4.85),(.43,.48,.08),'steelDark',.01)
collection('04 Cabin door and two levels of windows',offset=(0,-.5,.15))
def frontwindow(n,x,z,w=.80,h=.82):
 box(n+' blue panel',(x,FRONT-.057,z),(w,.04,h),'windowBlue',.003)
 for xx in [x-w/2,x,x+w/2]:box(n+' vertical trim',(xx,FRONT-.09,z),(.045,.07,h+.10),'ivory',.004)
 for zz in [z-h/2,z,z+h/2]:box(n+' crossbar',(x,FRONT-.09,zz),(w+.10,.07,.045),'ivory',.004)
 box(n+' sill',(x,FRONT-.15,z-h/2-.03),(w+.16,.22,.07),'walnut2',.008)
frontwindow('Ground floor window',CX-1.0,1.25,.72,.87)
frontwindow('Upper left window',CX-.84,2.79,.82,.88);frontwindow('Upper right window',CX+.78,2.79,.82,.88)
box('Walnut cabin door',(CX+.38,FRONT-.066,1.16),(.88,.075,1.68),'walnut2',.018)
box('Door blue glazing',(CX+.38,FRONT-.108,1.49),(.60,.018,.63),'windowBlue',.006)
for dx in [-.32,0,.32]:box('Door glazing muntin',(CX+.38+dx,FRONT-.124,1.49),(.03,.025,.68),'ivory',.003)
rod('Door handle',(CX+.70,FRONT-.15,1.05),(CX+.70,FRONT-.15,1.20),.017,'brass')
for z in [1.25,2.79]:
 box('Side window blue panel',(CX+W/2+.052,CY+.38,z),(.035,.86,.85),'windowBlue',.004)
 for y in [CY-.05,CY+.38,CY+.81]:box('Side window mullion',(CX+W/2+.083,y,z),(.07,.045,.93),'ivory',.004)
 for zz in [z-.425,z+.425]:box('Side window transom',(CX+W/2+.083,CY+.38,zz),(.07,.95,.05),'ivory',.004)
collection('05 Open porch and steps',offset=(0,-.5,.1))
for i in range(18):box('Porch individual cedar plank',(CX-1.70+i*.20,-.12,.335),(.191,1.18,.09),'oak2' if i%3 else 'oak3',.005)
for x in [CX-1.62,CX+1.62]:box('Porch support pier',(x,-.52,.19),(.23,.23,.28),'rock',.02)
for i in range(2):box('Porch broad stair',(CX+.38,-.86-i*.28,.255-i*.09),(1.12,.31,.12),'cedarLight',.008)
# No fence or guardrail in the clearing.
collection('06 Wooden porch lounge and side table',offset=(0,-.5,.2))
def wooden_lounge():
 for x in [-.29,.29]:
  rod('Lounge front leg',(x,-.26,.0),(x,-.23,.51),.032,'walnut2',12);rod('Lounge rear leg',(x,.48,.0),(x,.11,.44),.032,'walnut2',12)
  rod('Lounge seat bearer',(x,-.35,.40),(x,.31,.32),.034,'walnut2',12)
 for i in range(8):box('Lounge wooden seat slat',(0,-.32+i*.082,.406-i*.010),(.65,.073,.037),'oak3',.006)
 for i in range(5):
  x=(i-2)*.135;h=.77-.06*abs(i-2);o=box('Lounge wooden back slat',(x,.39,.75),(.122,.037,h),'oak2',.008);o.rotation_euler.x=-.20
 for x in [-.34,.34]:box('Lounge broad wood arm',(x,.02,.61),(.13,.70,.055),'oak3',.012);rod('Lounge arm upright',(x,-.21,.39),(x,-.21,.59),.022,'walnut2')
local_group('Slatted porch chair',(-2.82,-.12,.38),-.10,wooden_lounge)
table('Porch side table',-1.99,-.25,.78,.44,.42,'oak3');enamel_cup('Porch coffee',-1.99,-.25,.83,'ivory',.055,.10)
camp_lantern('Door lantern',-.77,.14,.40,.78)
collection('07 Simplified folded evergreen trees',offset=(0,-.4,.1))
def folded_pine(n,x,y,h,tiers=4):
 rod(n+' tapered trunk',(x,y,.07),(x,y,h*.75),.085,'walnut2',6)
 for layer in range(tiers):
  z=.15+h*(.16+layer*.17);r=h*(.22-layer*.038);hh=h*.38
  # Four broad triangular faces. Shallow ridges taper to zero at each edge.
  for side in range(4):
   N,L=10,8;v=[];f=[]
   for j in range(L+1):
    t=j/L
    for i in range(N+1):
     u=i/N;xx=(-r+2*r*u)*(1-t);yy=-r*(1-t)
     fold=.045*r*sin(pi*u)*sin(pi*t)*sin(6*pi*u+.8*layer+side*.3)
     yy-=fold;a=pi/4+side*pi/2;v.append((x+xx*cos(a)-yy*sin(a),y+xx*sin(a)+yy*cos(a),z+hh*t))
   for j in range(L):
    for i in range(N):q=j*(N+1)+i;f.append((q,q+1,q+N+2,q+N+1))
   mesh(n+' folded tier face',v,f,'pineFold' if (layer+side)%3 else 'pineFoldLight')
for args in [('Rear left pine',-3.68,2.91,3.08,4),('Rear corner pine',.48,2.93,3.78,4),('Rear right pine',1.83,3.02,3.20,4),('River bank pine',3.85,2.54,2.86,3),('Young riverside pine',3.88,.81,1.90,3)]:folded_pine(*args)
collection('08 Picnic table and benches',offset=(0,-.6,.15))
T=table('Camp picnic table',.89,.15,.87,2.14,.92,'oak3')
for y in [-.62,.92]:
 rounded_slab('Camp picnic bench',(.89,y,.49),2.12,.28,.075,.04,'oak2')
 for x in [.12,1.66]:rod('Bench leg',(x,y,.08),(x,y,.46),.037,'walnut2',12)
collection('09 Camp stove kettle and hearty food',offset=(0,-.5,.15))
box('Portable camp stove',(.35,.18,T+.05),(.68,.44,.10),'sageDark',.025)
for x in [.15,.55]:torus('Camp stove burner',(x,.18,T+.115),.12,.012,'black',steps=32)
lathe('Camp soup pot',(.15,.18,T+.12),[(0,0),(.14,0),(.15,.17),(.135,.17),(.13,.015),(0,.015)],'steelDark',32)
cyl('Camp beans surface',(.15,.18,T+.265),.13,.008,'gravy',32)
for i in range(13):a=i*2.4;r=.10*math.sqrt((i+.5)/13);uv('Baked bean',(.15+r*cos(a),.18+r*sin(a),T+.274),(.018,.011,.007),'sausage',10,6)
lathe('Camping kettle',(.55,.18,T+.12),[(0,0),(.11,0),(.13,.08),(.09,.22),(.04,.24),(0,.24)],'clay',32)
curve('Kettle bail',[(.44,.18,T+.30),(.42,.18,T+.48),(.68,.18,T+.48),(.66,.18,T+.30)],.012,'black')
curve('Kettle spout',[(.66,.18,T+.26),(.79,.18,T+.33),(.79,.18,T+.38)],.017,'clay')
rounded_slab('Hot dog sharing board',(1.20,.04,T+.025),.75,.49,.04,.04,'walnut2')
for i in range(3):
 x=.99+i*.22
 for side in [-1,1]:uv('Split hot dog bun',(x+side*.037,.04,T+.085),(.038,.16,.045),'crustLight',20,10)
 uv('Grilled sausage',(x,.04,T+.11),(.03,.163,.03),'sausage',20,10)
 curve('Mustard zigzag',[(x+.018*sin(j*2),-.085+j*.035,T+.14)for j in range(8)],.004,'mustard')
for x,y in [(.09,-.15),(1.68,.33),(1.64,-.13)]:enamel_cup('Picnic enamel mug',x,y,T,'ivory',.055,.10)
plate('Corn sharing plate',1.09,.43,T,.18)
for i in range(2):
 x=1.03+i*.12;rod('Grilled corn cob',(x,.33,T+.05),(x,.55,T+.05),.043,'honey',16)
 for j in range(6):torus('Corn kernel row',(x,.34+j*.035,T+.05),.044,.004,'butter',(pi/2,0,0),16)
box('Camping cooler',(1.45,-1.32,.34),(.72,.48,.51),'teal',.045);box('Cooler lid',(1.45,-1.32,.63),(.77,.52,.09),'ivory',.025)
for x in [1.19,1.71]:box('Cooler latch',(x,-1.58,.56),(.06,.03,.12),'steel',.008)
collection('10 Campfire and roasting treats',offset=(0,-.5,.1))
FX,FY=-.18,-2.47
cyl('Firepit dark earth',(FX,FY,.083),.58,.02,'walnut',48)
for i in range(13):a=i*2*pi/13;little_rock('Fire ring stone',FX+.54*cos(a),FY+.54*sin(a),.10,.13)
for a in [.2,2.2,4.2]:rod('Firewood log',(FX-.36*cos(a),FY-.36*sin(a),.17),(FX+.36*cos(a),FY+.36*sin(a),.23),.07,'walnut2',12)
for i in range(3):
 x=FX+(i-1)*.12;y=FY+.07*sin(i*2);v=[];N=16
 for j in range(8):
  t=j/7;r=.11*sin(pi*t)*(1.3-.7*t)
  for k in range(N):a=k*2*pi/N;v.append((x+.11*t*t+r*cos(a),y+r*.7*sin(a),.24+t*(.55+i*.08)))
 smooth(mesh('Sculpted campfire flame',v,[(j*N+k,j*N+(k+1)%N,(j+1)*N+(k+1)%N,(j+1)*N+k)for j in range(7)for k in range(N)],'flameCore' if i==1 else 'flame'))
fold_chair('Fireside canvas chair',(-1.55,-2.4,.06),-pi/2,'sageDark')
rod('Marshmallow roasting stick',(-1.23,-2.28,.65),(-.49,-2.42,.64),.009,'oak',8)
for x in [-.58,-.49]:cyl('Roasting marshmallow',(x,-2.42,.64),.032,.06,'white',16).rotation_euler.y=pi/2
rounded_slab('Smores board',(.85,-2.49,.23),.52,.35,.04,.04,'oak3')
for i in range(3):
 x=.7+i*.15
 box('Graham cracker base',(x,-2.49,.265),(.12,.12,.023),'cookie',.005);box('Chocolate square',(x,-2.49,.283),(.10,.10,.014),'chocolate',.003);pill('Smores marshmallow',(x,-2.49,.31),(.09,.09,.045),'white');box('Graham cracker top',(x,-2.49,.342),(.12,.12,.023),'cookie',.005)
collection('11 Path lanterns and stacked firewood',offset=(0,-.4,.1))
for i in range(5):little_rock('Cabin path stepping stone',-2.7+.18*sin(i),-3.17+i*.38,.06,.17)
for i in range(6):rod('Stored split firewood',(-3.45+i*.14,-1.42,.15),(-3.45+i*.14,-.97,.15),.057,'walnut2',10)
camp_lantern('Picnic lantern',2.00,-.91,.07,.82);camp_lantern('Path lantern',-3.52,-2.70,.075,.78)
for i in range(20):
 x=-3.8+(i%5)*1.12;y=3.25 if i<10 else -3.25
 for j in range(3):rod('Meadow grass tuft',(x,y,.07),(x+.04*(j-1),y+.02,.20+.025*sin(i)),.006,'grassDark',6)
bpy.context.scene['storeys']=2;bpy.context.scene['has_tent']=False;bpy.context.scene['has_fence']=False;bpy.context.scene['tree_style']='Four-sided tiers with shallow geometric folds';bpy.context.scene['menu']='Hot dogs, baked beans, corn, smores, marshmallows and coffee'
finish('riverside-cabin-campsite','Riverside Cabin Campsite — reconstructed',scale=12.5,target=(0,0,1.6))
