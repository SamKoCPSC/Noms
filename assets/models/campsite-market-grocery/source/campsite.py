collection('01 Rounded woodland clearing',offset=(0,0,0))
rounded_slab('Rounded earth diorama foundation',(0,0,-.13),8.6,7.2,.30,.48,'earth')
rounded_slab('Soft mossy clearing',(0,0,.055),8.45,7.05,.09,.43,'grass')
collection('02 Pines rocks and undergrowth')
for name,x,y,h in [('Tall left pine',-3.50,2.65,3.35),('Far woodland pine',.68,2.92,3.65),('Tall right pine',3.45,2.65,3.25),('Little right pine',3.70,1.20,2.30)]:pine(name,x,y,h)
for x,y,s in [(-3.73,.64,.33),(-3.63,1.07,.22),(3.68,-2.69,.40),(3.27,-2.92,.23),(.45,3.20,.27),(-.10,3.11,.21)]:little_rock('Weathered clearing stone',x,y,.10,s)
for i in range(27):
 a=i*2.399;x=3.85*cos(a);y=3.15*sin(a)
 for j in range(3):curve('Short grass tuft',[(x,y,.10),(x+.03*(j-1),y+.02,.24+random.random()*.08)],.009,'grassDark')
collection('03 Canvas ridge tent and groundsheet')
TX,TY,TZ=-1.40,1.42,.11
def tent():
 rounded_slab('Tent fitted groundsheet',(0,0,.015),3.02,2.50,.025,.07,'sageDark')
 # Roof panels are gently tensioned fabric surfaces, not solid triangular blocks.
 N,Q=16,12
 for side in [-1,1]:
  v=[]
  for j in range(Q+1):
   yy=-1.23+j*2.46/Q
   for i in range(N+1):
    u=i/N;xx=side*1.50*(1-u);zz=.04+2.19*u-.075*sin(pi*u)*sin(pi*j/Q)
    v.append((xx,yy,zz))
  f=[(j*(N+1)+i,j*(N+1)+i+1,(j+1)*(N+1)+i+1,(j+1)*(N+1)+i)for j in range(Q)for i in range(N)]
  o=smooth(mesh('Soft canvas roof panel',v,f,'linen'));md=o.modifiers.new('Canvas thickness','SOLIDIFY');md.thickness=.018
  curve('Canvas ridge seam',[(side*.01,-1.24,2.245),(side*.01,0,2.24),(side*.01,1.24,2.245)],.012,'ivory')
  curve('Rust canvas ground hem',[(side*1.5,-1.24,.055),(side*1.5,0,.055),(side*1.5,1.24,.055)],.026,'clay')
 # Rear closure and a genuinely open front entrance.
 mesh('Tent closed rear canvas',[(-1.49,1.23,.04),(1.49,1.23,.04),(0,1.23,2.23)],[(0,1,2)],'linen')
 for side in [-1,1]:
  o=mesh('Tied back front flap',[(side*1.50,-1.24,.04),(side*.86,-1.24,.04),(side*.31,-1.19,1.48),(0,-1.24,2.23)],[(0,1,2,3)],'clay');m=o.modifiers.new('Flap thickness','SOLIDIFY');m.thickness=.015
  curve('Rolled flap edge',[(side*.86,-1.26,.04),(side*.60,-1.25,.67),(side*.31,-1.23,1.48),(0,-1.25,2.23)],.027,'linen')
  rod('Front tent pole',(side*1.49,-1.27,.04),(0,-1.27,2.24),.018,'steelDark')
  curve('Tent guy rope',[(side*1.49,-1.22,.16),(side*1.78,-1.70,.03)],.005,'paper')
  rod('Grounded tent stake',(side*1.78,-1.70,-.015),(side*1.80,-1.70,.10),.011,'steel')
 rod('Tent ridge pole',(0,-1.27,2.24),(0,1.26,2.24),.015,'steelDark')
 rounded_slab('Sleeping mattress',(-.10,.17,.12),1.35,1.86,.18,.15,'sageLight')
 pill('Soft sleeping pillow',(-.10,.87,.27),(.83,.43,.19),'paper')
 pill('Thick warm sleeping quilt',(-.10,-.08,.255),(1.30,1.10,.15),'clay')
 for i in range(7):curve('Quilt stitched channel',[(-.68,-.55+i*.15,.332),(-.10,-.55+i*.15,.337),(.48,-.55+i*.15,.332)],.003,'peach')
 for i in range(3):book('Tent reading book',.99,.50,.04,.07,.35,.24,i)
local_group('Whole canvas tent',(TX,TY,TZ),0,tent)
collection('04 Tent doormat and evening lights')
rounded_slab('Woven tent doormat',(-1.4,-.21,.113),1.38,.66,.024,.06,'linen')
for i in range(9):curve('Doormat weave',[(-2.02,-.48+i*.066,.128),(-.78,-.48+i*.066,.128)],.005,'clay')
camp_lantern('Tent entrance lantern',-.27,.16,.12,1.1)
collection('05 Stone fire ring logs and firelight')
FX,FY=-.10,-1.37
cyl('Fire pit dark earth',(FX,FY,.12),.70,.025,'walnut',64)
for i in range(15):
 a=i*2*pi/15;o=uv('Fire ring rounded stone',(FX+.64*cos(a),FY+.64*sin(a),.21),(.15,.11,.11),'rock',24,12);o.rotation_euler.z=a+pi/2
for a in [.1,2.1,4.2]:
 dx,dy=.40*cos(a),.40*sin(a);rod('Split firewood',(FX-dx,FY-dy,.21),(FX+dx,FY+dy,.27),.09,'walnut2',20)
 for side in [-1,1]:
  pos=Vector((FX+side*dx,FY+side*dy,.24+side*.03));end=uv('Firewood cut end',pos,(.088,.088,.012),'oak',24,12);end.rotation_euler=(Vector((dx,dy,.03))).to_track_quat('Z','Y').to_euler()
for i,(dx,dy,hh,rr) in enumerate([(-.16,0,.54,.12),(.14,.03,.74,.15),(0,-.14,.47,.12)]):
 N=24;v=[]
 for j in range(10):
  t=j/9;r=rr*(sin(pi*t)**.85)*(1.35-.85*t);cx=FX+dx+.15*t*t
  for k in range(N):
   a=k*2*pi/N;v.append((cx+r*cos(a),FY+dy+r*.70*sin(a),.27+t*hh))
 smooth(mesh('Soft sculpted flame',v,[(j*N+k,j*N+(k+1)%N,(j+1)*N+(k+1)%N,(j+1)*N+k)for j in range(9)for k in range(N)],'flameCore' if i==2 else 'flame'))
light_point('Campfire amber pool',(FX,FY,.61),40,(1,.43,.15),.52)
collection('06 Canvas camp chairs and blanket')
fold_chair('Left fireside chair',(-1.70,-1.37,.03),-pi/2,'sageDark')
fold_chair('Right fireside chair',(1.48,-1.41,.03),pi/2,'clay')
fold_chair('Far fireside chair',(.73,-.04,.03),-.58,'linen')
# A folded throw drapes over the left chair arm; it follows a continuous profile.
def throw():
 v=[];N=15
 for xx,zz in [(-.35,.83),(-.22,.86),(-.04,.76),(.03,.54),(.13,.51),(.20,.36)]:
  for i in range(N):v.append((xx,-.14+i*.28/(N-1),zz+.008*sin(i*2.4)))
 smooth(mesh('Woven draped camp blanket',v,[(j*N+i,j*N+i+1,(j+1)*N+i+1,(j+1)*N+i)for j in range(5)for i in range(N-1)],'peach'))
local_group('Chair throw',(-1.70,-1.37,.03),-pi/2,throw)
collection('07 Camp kitchen and picnic bench')
T=table('Camp cooking table',2.16,1.42,.92,2.20,.91,'oak')
for y in [.64,2.20]:
 rounded_slab('Picnic bench top',(2.16,y,.48),2.13,.29,.08,.05,'oak')
 for x in [1.40,2.92]:rod('Picnic bench leg',(x,y,.10),(x,y,.45),.04,'walnut2')
box('Portable camp stove',(1.59,1.43,T+.06),(.70,.45,.12),'sageDark',.03)
for x in [1.38,1.79]:
 torus('Camp burner support',(x,1.43,T+.13),.12,.014,'black',steps=40)
 for a in [0,pi/2]:rod('Camp burner grate',(x-.14*cos(a),1.43-.14*sin(a),T+.145),(x+.14*cos(a),1.43+.14*sin(a),T+.145),.008,'steelDark')
lathe('Camp kettle',(1.43,1.43,T+.15),[(0,0),(.15,0),(.17,.08),(.13,.23),(.075,.26),(0,.26)],'clay',48)
curve('Kettle carrying handle',[(1.30,1.43,T+.32),(1.28,1.43,T+.52),(1.53,1.43,T+.54),(1.57,1.43,T+.32)],.012,'steelDark')
curve('Kettle pouring spout',[(1.58,1.43,T+.22),(1.72,1.43,T+.29),(1.73,1.43,T+.36)],.018,'clay')
rounded_slab('Picnic chopping board',(2.40,1.39,T+.02),.65,.48,.025,.04,'oak3')
loaf('Picnic bread',2.38,1.40,T+.045,.43,.15)
for x,y in [(2.86,1.25),(2.76,1.64)]:enamel_cup('Camp enamel coffee',x,y,T,'ivory')
jar('Camp jam',2.05,1.69,T,'red',.065,.18)
collection('08 Cooler backpack and firewood')
box('Camp cooler body',(2.62,-.05,.39),(.83,.56,.57),'teal',.075)
box('Camp cooler insulated lid',(2.62,-.05,.72),(.88,.60,.12),'ivory',.045)
for x in [2.33,2.91]:box('Cooler metal latch',(x,-.345,.66),(.07,.035,.15),'steel',.01)
curve('Cooler carrying handle',[(2.16,-.05,.45),(2.11,-.05,.75),(2.22,-.05,.85)],.018,'steelDark')
pill('Hiking backpack',(-3.11,-.39,.47),(.59,.37,.69),'clay')
pill('Backpack front pocket',(-3.11,-.59,.38),(.43,.09,.33),'peach')
for x in [-3.29,-2.93]:curve('Backpack shoulder strap',[(x,-.30,.74),(x,-.13,.60),(x,-.16,.19),(x,-.30,.14)],.026,'sageDark')
rod('Rolled bedroll',(-3.40,-.37,.87),(-2.82,-.37,.87),.105,'linen',40)
for x in [-3.30,-2.92]:torus('Bedroll strap',(x,-.37,.87),.108,.010,'walnut2',(0,pi/2,0),40)
for i in range(5):rod('Stacked camp firewood',(2.66+i*.16,-2.55,.18),(2.66+i*.16,-1.98,.18),.066,'walnut2',16)
collection('09 Camp lantern string and grounded poles')
for x,y in [(-3.0,-.73),(3.08,.25)]:rod('Camp string light pole',(x,y,.1),(x,y,2.70),.026,'walnut2')
pts=[(-3.0,-.73,2.70),(-.1,-.24,2.29),(3.08,.25,2.70)];curve('Gently sagging camp light cable',pts,.007,'black')
for i in range(11):
 t=i/10;x=-3+6.08*t;y=-.73+.98*t;z=2.70-.41*sin(pi*t)
 rod('Small bulb socket',(x,y,z),(x,y,z-.06),.013,'black',12);uv('Warm festoon bulb',(x,y,z-.095),(.036,.036,.045),'led',20,10)
collection('10 Approach path and camp sign')
for i in range(5):little_rock('Flat approach stepping stone',-2.72+.15*sin(i),-2.95+i*.48,.06,.24)
rod('Camp sign stake',(-3.55,-2.28,.10),(-3.55,-2.28,.88),.026,'walnut2')
board('Camp welcome sign','SLOW EVENINGS',-3.55,-2.28,.86,.96,.22,'sageDark',.067)
camp_lantern('Path lantern',-2.70,-2.96,.13,.85)
finish('cozy-campsite','Noms — Cozy Campsite',12.0,(0,0,1.04),night=True)
