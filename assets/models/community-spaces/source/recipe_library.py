shell(7.6,6.6,3.65,tiles=False,wall='wall')

def cookbook(n,x,y,z,w=.12,h=.44,d=.31,idx=0,title=None):
 ma=['sage','clay','teal','honey','bookCream','navy','pink'][idx%7]
 box(n+' page block',(x,y,z+h/2),(w-.012,d-.014,h-.018),'paper',.004)
 for xx in [x-w/2,x+w/2]:box(n+' clothbound cover',(xx,y,z+h/2),(.007,d,h),ma,.003)
 box(n+' rounded spine',(x,y-d/2,z+h/2),(w,.023,h),ma,.009)
 for zz in [z+.045,z+h-.045]:box(n+' spine gilt rule',(x,y-d/2-.013,zz),(w*.80,.002,.005),'brass',.001)
 if title:text(n+' title',title,(x,y-d/2-.016,z+h*.53),.027,'paper',rot=(pi/2,0,0))

collection('04 Tall cookbook shelving')
BX,BY=-1.15,2.91
box('Library back panel',(BX,3.16,1.86),(4.62,.09,3.34),'sageDark',.025)
for x in [-3.50,-1.94,-.38,1.18]:box('Bookcase vertical divider',(x,BY,1.86),(.075,.60,3.34),'walnut2',.012)
for z in [.23,.77,1.38,1.99,2.60,3.21,3.53]:box('Library thick shelf',(BX,BY,z),(4.76,.65,.067),'walnut2',.012)
for j,z in enumerate([.81,1.42,2.03,2.64]):
 for k,center in enumerate([-2.72,-1.16,.40]):
  for i in range(8):
   x=center-.61+i*.17;h=.40+.065*((i+j+k)%3)
   cookbook('Recipe collection volume',x,2.85,z,.115,h,.34,i+j+k,['BAKE','HOME','RICE','SOUP','FIRE','BREAD','SHARE','SWEET'][i] if j==2 else None)
# Separate lower cupboard doors and hardware.
for x in [-3.10,-2.34,-1.56,-.79,-.02,.76]:
 box('Bookcase lower cabinet door',(x,2.565,.49),(.73,.033,.43),'walnut2',.012)
 uv('Cabinet brass knob',(x+.24,2.534,.52),(.016,.013,.016),'brass',16,8)
text('Library shelf sign','T H E   R E C I P E   L I B R A R Y',(BX,2.564,3.34),.112,'paper')

collection('05 Recipe card catalog')
CX,CY=2.43,2.64
box('Card catalog recessed feet',(CX,CY,.15),(1.90,.61,.17),'walnut',.015)
box('Card catalog solid case',(CX,CY,1.01),(2.08,.78,1.61),'walnut2',.03)
box('Catalog thick top',(CX,CY,1.85),(2.18,.86,.085),'oak',.025)
for j in range(5):
 for i in range(4):
  x=CX-.78+i*.52;z=.39+j*.292
  box('Individual recipe index drawer',(x,CY-.407,z),(.49,.037,.264),'oak2',.012)
  box('Brass label holder',(x,CY-.431,z+.05),(.23,.012,.085),'brass',.006)
  box('Cream drawer index label',(x,CY-.439,z+.05),(.19,.003,.052),'paper',.002)
  text('Recipe drawer index',f'{j+1:02} / {i+1:02}',(x,CY-.443,z+.037),.026,'ink')
  curve('Drawer cup pull',[(x-.075,CY-.443,z-.04),(x-.057,CY-.47,z-.068),(x+.057,CY-.47,z-.068),(x+.075,CY-.443,z-.04)],.009,'brass')
board('Library welcome plaque','KEEP THE RECIPES.\nSHARE THE STORIES.',CX,3.18,2.64,1.95,.73,'sageDark',.13)
pot('Catalog small herb',3.12,2.66,1.90,.13)
# Card box with visible dividers and individually staggered recipe cards.
box('Open recipe box base',(1.94,2.56,1.95),(.67,.43,.075),'walnut2',.02)
for x in [1.61,2.27]:box('Recipe box side',(x,2.56,2.05),(.024,.43,.24),'walnut2',.006)
for y in [2.35,2.77]:box('Recipe box end',(1.94,y,2.05),(.67,.024,.24),'walnut2',.006)
for i in range(8):
 y=2.38+i*.047;box('Individual index card',(1.94,y,2.09),(.58,.007,.28),'paper',.003)
 if i in [1,4,7]:box('Index card divider tab',(1.77+.05*i,y,2.248),(.14,.007,.042),'sageLight',.003)

collection('06 Reading table and chairs')
TZ=table('Shared recipe reading table',-.85,-.61,.95,3.75,1.50,'oak3')
for x in [-2.01,.34]:chair('Reading chair',(x,.53,.02),0,'sage')
chair('Reading chair',(-1.60,-1.80,.02),pi,'sage')
chair('Reading chair',(.18,-1.80,.02),pi,'sage')

collection('07 Open recipe book and handwritten notes')
def openbook():
 # A continuous curved spread with a central crease, multiple page edges and cloth covers.
 for side in [-1,1]:
  box('Book cloth cover',(side*.25,0,.012),(.50,.66,.024),'clay',.014)
  for page in range(5):
   pts=[];N=24
   for yy in [-.305,.305]:
    for i in range(N+1):
     u=i/N;xx=side*(.018+.46*u);zz=.027+page*.003+.065*sin(pi*u*.88)
     pts.append((xx,yy,zz))
   faces=[(i,i+1,N+1+i+1,N+1+i)for i in range(N)]
   mesh('Curved open book page',pts,faces,'paper')
  # Ink strokes follow the page surface instead of floating above it.
  for j in range(13):
   y=-.22+j*.035;points=[]
   for i in range(14):
    u=.15+i*.045;points.append((side*(.018+.46*u),y,.027+.012+.065*sin(pi*u*.88)+.0015))
   curve('Printed recipe lines',points,.0015,'sageDark',poly=True)
 curve('Book spine binding',[(0,-.32,.023),(0,0,.024),(0,.32,.023)],.008,'clay')
 curve('Recipe book ribbon',[(0,.30,.045),(.01,-.21,.046),(.04,-.38,.010),(.05,-.46,.004)],.008,'sage')
local_group('Open family recipe book',(-1.09,-.76,TZ+.003),-.17,openbook)
for i in range(3):
 x=-2.03+i*.10;y=-.34+i*.06;o=box('Loose recipe index card',(x,y,TZ+.008+i*.004),(.39,.25,.003),'paper',.004);o.rotation_euler.z=.1*i
text('Handwritten recipe card heading','FAMILY RECIPES',(-2.01,-.32,TZ+.026),.035,'walnut',rot=(0,0,.1))
rod('Wooden pencil',(-2.32,-.82,TZ+.015),(-2.03,-.55,TZ+.015),.008,'honey',12)
latte('Reading table latte',.65,-.94,TZ,.078)

collection('08 Recipe laptop and bankers lamp')
LX,LY=.36,-.29
rounded_slab('Laptop rounded keyboard deck',(LX,LY,TZ+.018),.73,.49,.029,.035,'steelDark')
box('Laptop recessed keyboard',(LX,LY+.045,TZ+.035),(.61,.25,.008),'black',.01)
for j in range(4):
 for i in range(11):box('Laptop key',(LX-.267+i*.053,LY-.040+j*.056,TZ+.043),(.043,.041,.009),'steelDark',.004)
box('Laptop spacebar',(LX,LY-.075,TZ+.044),(.24,.038,.009),'steelDark',.004)
box('Laptop touchpad',(LX,LY-.165,TZ+.037),(.24,.105,.006),'steel',.012)
def screen():
 box('Laptop display housing',(0,0,.23),(.73,.029,.46),'steelDark',.021)
 box('Laptop original recipe app screen',(0,-.018,.23),(.676,.006,.402),'paper',.009)
 text('Laptop app brand','NOMS',(-.275,-.024,.37),.053,'sageDark',align='LEFT')
 text('Laptop recipe heading','Your recipe library',(-.275,-.024,.305),.028,'ink',align='LEFT')
 for i in range(3):
  x=-.22+i*.22;box('Recipe thumbnail',(x,-.025,.213),(.175,.003,.10),['clay','sageLight','honey'][i],.008)
  for j in range(2):box('Recipe description line',(x,-.025,.14-j*.027),(.16-j*.03,.003,.005),'sage',.001)
local_group('Laptop opened display',(LX,LY+.235,TZ+.04),0,screen).rotation_euler.x=-.22
# The shade is supported by the arched brass stem.
GX,GY=-2.28,-.09
lathe('Banker lamp weighted base',(GX,GY,TZ),[(0,0),(.19,0),(.20,.022),(.16,.055),(.06,.075),(0,.075)],'brass',48)
curve('Bankers lamp stem',[(GX,GY,TZ+.06),(GX,GY,TZ+.48),(GX,GY-.10,TZ+.59),(GX,GY-.17,TZ+.55)],.018,'brass')
lathe('Green banker lamp shade',(GX,GY-.18,TZ+.48),[(.15,0),(.16,.025),(.12,.15),(.05,.18),(0,.18)],'sageDark',48,(1.6,.75,1))
uv('Banker lamp opal bulb',(GX,GY-.18,TZ+.48),(.12,.052,.03),'led',24,12)
curve('Banker lamp pull chain',[(GX+.16,GY-.15,TZ+.5),(GX+.16,GY-.15,TZ+.29)],.0025,'brass')

collection('09 Reading nook rug armchair and side table')
rounded_slab('Reading nook woven rug',(2.46,-.68,.078),2.08,2.66,.026,.13,'sageLight')
curve('Rug stitched border',rounded_points(2.46,-.68,.094,1.91,2.49,.10),.005,'linen',True,True)
def lounge():
 for x in [-.37,.37]:
  for y in [-.33,.33]:rod('Lounge wood foot',(x,y,.08),(x,y,.24),.037,'walnut2')
 pill('Lounge cushion base',(0,0,.35),(1.04,.92,.28),'sage')
 pill('Lounge soft seat',(0,-.04,.54),(.83,.77,.23),'sageLight')
 pill('Lounge attached padded back',(0,.37,.88),(.93,.23,.87),'sage')
 for x in [-.48,.48]:pill('Lounge rounded arm',(x,-.03,.69),(.22,.96,.55),'sage')
 pill('Lounge scatter cushion',(0,.14,.87),(.45,.18,.43),'clay')
local_group('Cozy recipe reading chair',(2.50,-.12,.01),-.18,lounge)
pill('Reading ottoman cushion',(2.42,-1.44,.38),(.81,.62,.21),'sageLight')
for x in [2.13,2.71]:
 for y in [-1.64,-1.24]:rod('Ottoman leg',(x,y,.10),(x,y,.29),.028,'walnut2')
cyl('Reading side table top',(3.36,-.50,.68),.29,.055,'oak',64)
cyl('Side table pedestal',(3.36,-.50,.39),.045,.54,'walnut2',32)
cyl('Side table foot',(3.36,-.50,.13),.19,.035,'walnut2',40)
book('Side table book',3.35,-.49,.714,.06,.30,.20,1)

collection('10 Rolling bookcase ladder and rail')
rod('Library ladder rail',(-3.48,2.53,3.13),(1.12,2.53,3.13),.025,'brass')
for x in [-3.30,1.0]:rod('Rail wall bracket',(x,2.54,3.13),(x,2.72,3.13),.025,'brass')
# Both feet meet the floor; rails and rungs are one supported assembly.
for x in [-3.33,-2.88]:
 rod('Library ladder side',(x,1.79,.11),(x,2.52,3.12),.028,'walnut2')
 o=cyl('Ladder rolling top wheel',(x,2.52,3.15),.057,.027,'steelDark',24);o.rotation_euler.y=pi/2
for i in range(9):
 z=.27+i*.30;y=1.79+(z-.11)/3.01*.73
 box('Rounded ladder rung',(-3.105,y,z),(.47,.14,.038),'oak',.01)
collection('11 Small returns book trolley')
TX,TY=-2.93,-2.30
for z in [.28,.69]:
 box('Book trolley shelf',(TX,TY,z),(1.0,.51,.05),'oak2',.014)
 box('Trolley rear book stop',(TX,TY+.23,z+.10),(1.0,.035,.20),'walnut2',.008)
for x in [TX-.46,TX+.46]:
 for y in [TY-.19,TY+.19]:
  rod('Trolley upright',(x,y,.16),(x,y,.94),.019,'brass')
  o=cyl('Trolley small caster',(x,y,.13),.07,.037,'black',24);o.rotation_euler.x=pi/2
for i in range(5):cookbook('Returned cookbook',TX-.32+i*.16,TY,.725,.10,.31,.29,i)
for i in range(3):box('Trolley lower stacked book',(TX,TY,.34+i*.06),(.62,.39,.052),['clay','sage','paper'][i],.008)
rod('Book trolley push grip',(TX-.46,TY+.19,.95),(TX+.46,TY+.19,.95),.026,'walnut2')
finish('recipe-library','Noms — Recipe Library',11.4,(0,0,1.18))
