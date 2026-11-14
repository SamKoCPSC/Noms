"""Fittings and props shared by the campsite, lantern courtyard and grocer."""
for k,h in {'grass':'#858E65','grassDark':'#5F7654','earth':'#927152','rock':'#9C9D90','pine':'#55705C','pineLight':'#718975','peach':'#D5A08C','butter':'#E4C47D','plum':'#77586C','mist':'#B7C9C1','slate':'#5C6A71'}.items():M[k]=material(k,h,.88)
M['flame']=material('Soft amber flame','#EF852D',.9,emit=.75)
M['flameCore']=material('Soft golden flame core','#F2B64D',.9,emit=1.0)
M['lanternPaper']=material('Warm paper lantern','#E9B782',.86,emit=.28)

def light_point(n,loc,power=15,color=(1,.58,.28),radius=.2):
 d=bpy.data.lights.new(n,'POINT');d.energy=power;d.color=color;d.shadow_soft_size=radius;o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;return o

def bottle(n,x,y,z,ma='teal',h=.29,r=.065,label='NOMS'):
 lathe(n+' bottle',(x,y,z),[(0,0),(.85*r,0),(r,.02),(r,h*.68),(.46*r,h*.83),(.43*r,h*.95),(0,h*.95)],ma,40)
 cyl(n+' cap',(x,y,z+h*.97),r*.47,h*.07,'ivory',32)
 box(n+' paper label',(x,y-r-.001,z+h*.43),(r*1.40,.003,h*.32),'paper',.005)
 text(n+' print',label,(x,y-r-.004,z+h*.43),.025,'ink')

def tin(n,x,y,z,ma='clay',label='NOMS',r=.065,h=.18):
 cyl(n+' round can',(x,y,z+h/2),r,h,ma,40)
 for zz in [z+.007,z+h-.004]:torus(n+' rolled rim',(x,y,zz),r,.004,'steel',steps=32)
 cyl(n+' lid',(x,y,z+h),r*.94,.003,'steel',32)
 box(n+' label',(x,y-r-.002,z+h*.53),(r*1.3,.003,h*.53),'paper',.004)
 text(n+' label text',label,(x,y-r-.004,z+h*.50),.024,'ink')

def pack(n,x,y,z,w=.19,d=.12,h=.31,ma='sage',label='PANTRY'):
 box(n+' carton',(x,y,z+h/2),(w,d,h),ma,.012)
 box(n+' front label',(x,y-d/2-.002,z+h*.55),(w*.78,.004,h*.50),'paper',.004)
 text(n+' print',label,(x,y-d/2-.005,z+h*.55),min(.034,w*.16),'ink')
 for i in range(2):box(n+' label rule',(x,y-d/2-.005,z+h*(.37-i*.05)),(w*.5,.002,.003),'sageDark',.001)

def enamel_cup(n,x,y,z,ma='ivory',r=.07,h=.12):
 lathe(n+' hollow mug',(x,y,z),[(0,0),(.9*r,0),(r,.012),(r,h),(r-.006,h),(r-.006,.012),(0,.012)],ma,40)
 torus(n+' dark rolled lip',(x,y,z+h),r-.002,.004,'navy',steps=40)
 curve(n+' attached mug handle',[(x+r*.95,y,z+h*.8),(x+r*1.63,y,z+h*.80),(x+r*1.66,y,z+h*.3),(x+r*.95,y,z+h*.24)],.009,ma)
 cyl(n+' coffee surface',(x,y,z+h*.83),r-.008,.002,'tea',40)

def camp_lantern(n,x,y,z,s=.9):
 def fn():
  cyl(n+' metal foot',(0,0,.035),.12,.07,'sageDark',40)
  lathe(n+' clear globe',(0,0,.065),[(.08,0),(.095,.03),(.082,.21),(.074,.23),(.071,.23),(.078,.21),(.089,.03),(.074,0)],'glass',40)
  lathe(n+' top shade',(0,0,.29),[(.12,0),(.12,.02),(.06,.08),(.04,.10),(0,.10)],'sageDark',40)
  uv(n+' warm mantle',(0,0,.17),(.038,.038,.065),'led',24,12)
  for xx in [-.095,.095]:curve(n+' guard',[(xx,0,.045),(xx*1.07,0,.15),(xx*.9,0,.28)],.007,'steelDark')
  curve(n+' carrying bail',[(-.08,0,.31),(-.10,0,.43),(0,0,.50),(.10,0,.43),(.08,0,.31)],.007,'steelDark')
 o=local_group(n,(x,y,z),0,fn);o.scale=(s,)*3
 light_point(n+' light',(x,y,z+.17*s),9*s,(1,.65,.34),.16*s)

def paper_lantern(n,x,y,z,r=.20):
 lathe(n+' rounded paper shade',(x,y,z),[(.07,-r),(.15,-r*.7),(r,0),(.15,r*.7),(.07,r)],'lanternPaper',48)
 for j in range(9):
  t=-pi/2+.18+j*(pi-.36)/8;torus(n+' rib',(x,y,z+r*sin(t)),r*cos(t),.0025,'bambooDark',steps=40)
 for zz in [z-r,z+r]:cyl(n+' cap',(x,y,zz),.067,.016,'black',24)
 rod(n+' hanger',(x,y,z+r),(x,y,z+r+.10),.006,'black',10)
 light_point(n+' glow',(x,y,z-.05),12,(1,.61,.32),.23)

def pine(n,x,y,h=3.0):
 rod(n+' trunk',(x,y,.10),(x,y,h*.82),.085,'walnut2',20)
 for j in range(4):
  z=.10+h*(.22+j*.17);r=h*(.25-j*.042);hh=h*.37;N=32;v=[]
  profile=[(.3,-.03),(1,0),(1.03,.08),(.92,.19),(.68,.42),(.34,.70),(.035,1)]
  for rr,zz in profile:
   for i in range(N):
    a=i*2*pi/N;rad=r*rr*(1+.035*sin(7*a+j));v.append((x+rad*cos(a),y+rad*sin(a),z+hh*zz-.035*sin(7*a+j)*rr))
  f=[tuple(reversed(range(N))),tuple(range((len(profile)-1)*N,len(profile)*N))]+[(k*N+i,k*N+(i+1)%N,(k+1)*N+(i+1)%N,(k+1)*N+i)for k in range(len(profile)-1)for i in range(N)]
  smooth(mesh(n+' softly scalloped bough',v,f,'pine' if j%2 else 'pineLight'))

def little_rock(n,x,y,z,s=.2):
 o=uv(n,(x,y,z+s*.32),(s,s*.75,s*.40),'rock',20,12);o.rotation_euler.z=random.random()*pi;return o

def fold_chair(n,loc,angle=0,ma='sage'):
 def fn():
  for x in [-.32,.32]:
   rod(n+' crossed front tube',(x,-.30,.09),(x,.23,.66),.023,'walnut2')
   rod(n+' crossed rear tube',(x,.32,.09),(x,-.28,.59),.023,'walnut2')
   rod(n+' back upright',(x,.18,.45),(x,.39,1.08),.022,'walnut2')
   o=cyl(n+' pivot pin',(x+(-.025 if x<0 else .025),.01,.36),.033,.012,'brass',24);o.rotation_euler.y=pi/2
  # Both fabric panels sag between their supporting rails.
  N=18;v=[]
  for y in [-.27,.24]:
   for i in range(N+1):
    x=-.32+i*.64/N;v.append((x,y,.59-.075*sin(pi*i/N)))
  o=mesh(n+' slung canvas seat',v,[(i,i+1,N+1+i+1,N+1+i)for i in range(N)],ma);m=o.modifiers.new('Canvas thickness','SOLIDIFY');m.thickness=.012
  v=[]
  for t in [0,1]:
   for i in range(N+1):
    x=-.32+i*.64/N;v.append((x,.22+.17*t+.035*sin(pi*i/N),.65+.40*t))
  o=mesh(n+' canvas back',v,[(i,i+1,N+1+i+1,N+1+i)for i in range(N)],ma);m=o.modifiers.new('Canvas thickness','SOLIDIFY');m.thickness=.012
  for z,y in [(1.05,.39),(.65,.22)]:rod(n+' back crossbar',(-.33,y,z),(.33,y,z),.017,'walnut2')
  for x in [-.34,.34]:rod(n+' attached arm',(x,-.23,.77),(x,.29,.80),.034,'walnut2')
 local_group(n,loc,angle,fn)
