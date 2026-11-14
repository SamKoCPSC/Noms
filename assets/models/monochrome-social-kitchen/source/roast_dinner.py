"""Reference-led roast dinner; geometric detail and exportable vertex colors."""
from mathutils import noise
for k,h in {'ribSurface':'#FFFFFF','ribCut':'#FFFFFF','potatoSurface':'#FFFFFF','pepperSauce':'#C8A76E','dinnerStone':'#443C32','dinnerRim':'#A18D69','gardenPea':'#659127','rosemary':'#435432','briocheCrust':'#B8732F','briocheCrumb':'#EDD3A0','butter':'#E9CD81','wine':'#631F2C','bottle':'#254335','soup':'#C99740'}.items():M[k]=material(k,h,.8)
for key in ['ribSurface','ribCut','potatoSurface']:
 m=M[key];nd=m.node_tree.nodes;lk=m.node_tree.links
 vc=nd.new('ShaderNodeVertexColor');vc.layer_name='FoodColor';lk.new(vc.outputs['Color'],nd.get('Principled BSDF').inputs['Base Color'])
 t=nd.new('ShaderNodeTexNoise');t.inputs['Scale'].default_value=155 if key=='ribCut' else 110
 b=nd.new('ShaderNodeBump');b.inputs['Strength'].default_value=.2;b.inputs['Distance'].default_value=.0012
 lk.new(t.outputs['Fac'],b.inputs['Height']);lk.new(b.outputs[0],nd.get('Principled BSDF').inputs['Normal'])
def blendcol(a,b,t):
 aa=rgb(a);bb=rgb(b);t=max(0,min(1,t));return tuple(aa[i]*(1-t)+bb[i]*t for i in range(3))+(1,)
def foodmesh(n,v,f,colors,mat):
 o=mesh(n,v,f,mat);at=o.data.color_attributes.new(name='FoodColor',type='FLOAT_COLOR',domain='CORNER')
 for lp in o.data.loops:at.data[lp.index].color=colors[lp.vertex_index]
 return smooth(o)
def section(t,oval=False):
 if oval:return .35*cos(t),.247+.23*sin(t)
 rr=1+.085*sin(3*t+.4)+.045*cos(5*t-1.1)
 return .35*cos(t)*rr,max(.016,.247+.247*sin(t)*rr)
def cutcolor(px,pz,r,t,seed):
 q=noise.noise(Vector((px*36+seed,pz*32,2.7)));fib=.5+.5*sin(px*560+7*sin(pz*70)+q*2)
 if r>.969:return blendcol('#301B13','#734020',.45+q*.6)
 if r>.885+.035*sin(5*t)+.025*sin(11*t) and (sin(t)>.4 or cos(t)>.65):return blendcol('#D4AB79','#F0D0A6',.5+q*.4)
 if r>.90:return blendcol('#98665A','#BD8577',fib*.5+.2)
 if pz>.32 and abs(pz-(.365-.19*(px+.04)**2))<.006+.002*sin(px*62):return blendcol('#DAAB9E','#EAC1AE',.5+q*.3)
 return blendcol('#983F49','#D58583',.44+q*.34+(fib-.5)*.23)
def cutface(n,mapper,seed=0,oval=False):
 N,L=96,24;v=[mapper(0,.247)];cs=[cutcolor(0,.247,0,0,seed)]
 for j in range(1,L+1):
  r=j/L
  for i in range(N):
   t=2*pi*i/N;xx,zz=section(t,oval);xx*=r;zz=.247+(zz-.247)*r;v.append(mapper(xx,zz));cs.append(cutcolor(xx,zz,r,t,seed))
 f=[(0,1+i,1+(i+1)%N)for i in range(N)]
 for j in range(L-1):
  a=1+j*N;b=a+N
  for i in range(N):f.append((a+i,b+i,b+(i+1)%N,a+(i+1)%N))
 return foodmesh(n,v,f,cs,'ribCut')
def roast(n,x,y,z):
 N,L=96,26;v=[];cs=[]
 for j in range(L+1):
  d=j/L;yy=y+d*.52;bulge=1+.035*sin(d*pi)-.065*d**5
  for i in range(N):
   t=2*pi*i/N;xx,zz=section(t,True);q=noise.noise(Vector((xx*53,yy*49,zz*51)));xx=xx*bulge+.007*q*cos(t);zz=max(.008,zz+.008*q)
   v.append((x+xx,yy,z+zz));m=noise.noise(Vector((xx*32,yy*33,zz*35)));cs.append(blendcol('#100C08','#492916',.38+m*.64) if m<.38 else blendcol('#512C16','#78431D',(m-.38)*2))
 f=[(j*N+i,(j+1)*N+i,(j+1)*N+(i+1)%N,j*N+(i+1)%N)for j in range(L)for i in range(N)]
 f.append(tuple(reversed(range(L*N,(L+1)*N))));foodmesh(n+' uneven pepper crust',v,f,cs,'ribSurface');cutface(n+' pink carved face',lambda xx,zz:(x+xx,y-.001,z+zz),1,oval=True)
 for i in range(36):
  a=.35+random.random()*2.45;xx,zz=section(a,True);yy=y+.04+random.random()*.43
  uv(n+' herb crust',(x+xx,yy,z+zz+.003),(.005,.01,.002),'rosemary' if i%4==0 else 'beefCrust',8,6)
def ribslice(n,x,y,z,s=1,a=0,oval=False):
 def fn():
  N=96;v=[];cs=[]
  for dz in [0,.023]:
   for i in range(N):
    t=2*pi*i/N;xx,zz=section(t,oval);v.append((xx*.29,(zz-.247)*.36,dz+.0015*sin(i*.8)));cs.append(blendcol('#3A2115','#86512C',.43+.25*sin(i*2.6)))
  f=[tuple(reversed(range(N)))]+[(i,(i+1)%N,(i+1)%N+N,i+N)for i in range(N)]
  foodmesh(n+' browned perimeter',v,f,cs,'ribSurface');cutface(n+' rosy beef grain',lambda xx,zz:(xx*.29,(zz-.247)*.36,.024),x*11+y*7,oval=oval)
 o=local_group(n,(x,y,z),a,fn);o.scale=(s,s,s);return o
def potato_piece(n,s,kind):
 N,L=24,10;quarter=kind%3!=1;span=pi if quarter else 2*pi;count=N+1 if quarter else N
 v=[(0,0,0)];cs=[rgb('#8D4B21')]
 def pos(phi,t):
  rr=cos(phi)*(1+.12*sin(3*t+kind)+.04*cos(7*t+kind));return (s*rr*cos(t),s*.8*rr*sin(t),s*.88*(sin(phi)+1))
 for j in range(1,L+1):
  phi=-pi/2+(math.asin(.46)+pi/2)*j/L
  for i in range(count):
   p=pos(phi,span*i/N);v.append(p);q=noise.noise(Vector((p[0]*130+kind,p[1]*130,p[2]*110)));cs.append(blendcol('#8E3D0D','#D79024',.45+q*.85))
 f=[(0,1+(i+1)%count,1+i)for i in range(count-1 if quarter else count)]
 for j in range(L-1):
  a=1+j*count;b=a+count
  for i in range(count-1 if quarter else count):f.append((a+i,a+(i+1)%count,b+(i+1)%count,b+i))
 center=len(v);tz=s*.88*1.46;v.append((0,0,tz));cs.append(rgb('#CF8827'));prev=None
 for ring in range(1,6):
  row=[]
  for i in range(count):
   px,py,pz=pos(math.asin(.46),span*i/N);px*=ring/5;py*=ring/5;q=noise.noise(Vector((px*150+kind*.8,py*150,3)))
   row.append(len(v));v.append((px,py,pz));cs.append(blendcol('#7E3810','#E9AD42',.4+q*.95))
  for i in range(count-1 if quarter else count):
   ni=(i+1)%count;f.append((center,row[i],row[ni]) if prev is None else (prev[i],row[i],row[ni],prev[ni]))
  prev=row
 if quarter:f.append(tuple([0]+[1+j*count for j in range(L)]+[center]+[1+j*count+count-1 for j in reversed(range(L))]))
 o=foodmesh(n+' crisp potato chunk',v,f,cs,'potatoSurface')
 for p in o.data.polygons:
  if all(abs(v[k][2]-tz)<1e-5 for k in p.vertices):p.use_smooth=False
 for k in range(2):
  x=(k-.5)*s*.34;y=(k-.5)*s*.15;rod(n+' rosemary',(x-s*.13,y,tz+.002),(x+s*.17,y+s*.09,tz+.002),.0018,'rosemary',8)
def potatoes(n,x,y,z,s=.05,count=6,cols=None):
 cols=cols or (4 if count>=9 else 3);rows=math.ceil(count/cols)
 for i in range(count):
  xx=x+(i%cols-(cols-1)/2)*s*2.03+s*.16*sin(i*3.4);yy=y+(i//cols-(rows-1)/2)*s*1.8+s*.14*cos(i*1.7)
  o=local_group(n+' irregular roasted piece',(xx,yy,z),i*2.4,lambda:potato_piece(n,s*(.85+.24*sin(i*1.7+.3)**2),i));o.rotation_euler.x=.42*sin(i*2.1+.4);o.rotation_euler.y=.33*cos(i*1.4)
  bpy.context.view_layer.update();low=min((c.matrix_world@v.co).z for c in o.children_recursive if c.type=='MESH'for v in c.data.vertices);o.location.z+=z-low

def sauce(n,x,y,z):
 v=[];f=[];S,W=28,14
 for i in range(S+1):
  t=i/S;h=(.034+.006*sin(t*13))*sin(pi*t)**.4
  for j in range(W+1):
   u=2*j/W-1;v.append((x+.15*t+u*h*.52,y-.11*t+u*h,z+.009*(1-u*u)+.003*sin(t*10)))
 for i in range(S):
  for j in range(W):a=i*(W+1)+j;f.append((a,a+1,a+W+2,a+W+1))
 smooth(mesh(n+' peppercorn cream sauce',v,f,'pepperSauce'))
 for i in range(22):
  t=(i+.5)/22;u=.65*sin(i*2.4);h=(.034+.006*sin(t*13))*sin(pi*t)**.4
  uv(n+' cracked pepper',(x+.15*t+u*h*.52,y-.11*t+u*h,z+.009*(1-u*u)+.003*sin(t*10)+.0015),(.0018,.0014,.001),'beefCrust',8,6)
def plated_dinner(n,x,y,z):
 plate(n+' dark stoneware plate',x,y,z,.235,'dinnerStone');torus(n+' warm rim',(x,y,z+.027),.231,.0025,'dinnerRim')
 for i in range(4):ribslice(n+' carved beef',x+.005+i*.043,y+.104-i*.032,z+.038+i*.006,1,.25).scale=(.60,1.02,.85)
 sauce(n,x-.005,y+.12,z+.077)
 for i in range(43):
  a=i*2.4;r=.073*math.sqrt((i+.5)/43);uv(n+' green pea',(x-.125+r*cos(a),y+.006+r*sin(a)*1.12,z+.047+(i%3)*.003),(.0105,.0105,.010),'gardenPea',12,8)
 potatoes(n,x+.016,y-.128,z+.034,.035,6)
def brioche(n,x,y,z,count=5,s=1):
 for i in range(count):
  xx=x+(i-(count-1)/2)*.058*s;prof=[(-.10,0),(.10,0),(.11,.135),(.105,.177),(.073,.205),(.035,.208),(0,.228),(-.035,.208),(-.073,.205),(-.105,.177),(-.11,.135)];N=len(prof)
  v=[(xx+dx*s,y+yy*s,z+zz*s)for dx in [-.023,.023]for yy,zz in prof]
  bevel(mesh(n+' brioche slice crust',v,[tuple(reversed(range(N))),tuple(range(N,2*N))]+[(j,(j+1)%N,(j+1)%N+N,j+N)for j in range(N)],'briocheCrust'),.003,2)
  mesh(n+' buttery crumb',[(xx+.0235*s,y+yy*.87*s,z+(.11+(zz-.11)*.85)*s)for yy,zz in prof],[tuple(range(N))],'briocheCrumb')
  for j in range(5):uv(n+' crumb pore',(xx+.024*s,y+.055*sin(j*2.4)*s,z+(.06+j*.025)*s),(.0015,.004*s,.003*s),'honey',8,6)
def fanned_brioche(n,x,y,z):
 for i in range(4):
  o=local_group(n+' angled slice',(x-.05+i*.042,y+.012*i,z),-.5+i*.3,lambda:brioche(n,0,0,0,1,.67));o.rotation_euler.y=-1.4
  bpy.context.view_layer.update();low=min((c.matrix_world@v.co).z for c in o.children_recursive if c.type=='MESH'for v in c.data.vertices);o.location.z+=z+.003+i*.012-low

def winebottle(n,x,y,z,label):
 lathe(n+' green bottle',(x,y,z),[(0,0),(.065,0),(.067,.22),(.045,.27),(.026,.30),(.026,.43),(0,.43)],'bottle',32)
 cyl(n+' foil',(x,y,z+.425),.027,.075,'wine',24)
 box(n+' label',(x,y-.066,z+.14),(.095,.007,.13),'paper',.002)
 text(n+' label text',label,(x,y-.071,z+.16),.013,'ink');text(n+' vintage','2020',(x,y-.071,z+.105),.017,'ink')
def wineglass(n,x,y,z):
 lathe(n+' bowl',(x,y,z+.14),[(.012,0),(.060,.04),(.072,.095),(.065,.16),(.062,.16),(.068,.095),(.056,.04),(.009,.006)],'glass',32)
 cyl(n+' stem',(x,y,z+.075),.007,.15,'glass',16);cyl(n+' foot',(x,y,z+.008),.058,.012,'glass',32)
 lathe(n+' wine',(x,y,z+.14),[(0,.01),(.055,.04),(.066,.09),(0,.09)],'wine',32)


def soup_bowl(n,x,y,z):
 """Individual soup portion with an inset surface and visible vegetables."""
 lathe(n+' ivory bowl',(x,y,z),[(0,0),(.045,0),(.061,.012),(.095,.056),(.105,.082),(.105,.088),(.099,.088),(.089,.056),(.054,.016),(0,.016)],'ceramic',48)
 torus(n+' rolled rim',(x,y,z+.086),.102,.003,'ivory',steps=48)
 cyl(n+' golden soup',(x,y,z+.067),.091,.004,'soup',48)
 for i in range(10):
  a=i*2.4;r=.072*math.sqrt((i+.5)/10)
  box(n+' vegetable garnish',(x+r*cos(a),y+r*sin(a),z+.071),(.011,.009,.004),'orange' if i%3 else 'leaf',.002)
 for i in range(4):
  a=i*2.4+.4;uv(n+' soup bean',(x+.042*cos(a),y+.042*sin(a),z+.071),(.009,.005,.003),'dough',10,6)
