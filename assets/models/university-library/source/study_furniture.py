titles=['ALGORITHMS','DATA STRUCTURES','OPERATING SYSTEMS','DATABASES','NETWORKS','COMPILERS','COMPUTER GRAPHICS','MACHINE LEARNING','DISCRETE MATH','SOFTWARE DESIGN','TYPE SYSTEMS','DISTRIBUTED SYSTEMS']
colors=['navy','sageDark','burgundy','teal','bookCream','clay']
def volume(name,x,y,z,idx=0,w=.055,h=.27,d=.20):
 ma=colors[idx%len(colors)]
 box(name+' page block',(x,y,z+h/2),(w-.009,d-.012,h-.012),'paper',.002)
 for xx in [x-w/2,x+w/2]:box(name+' bound cover',(xx,y,z+h/2),(.004,d,h),ma,.002)
 box(name+' spine',(x,y-d/2,z+h/2),(w,.015,h),ma,.004)
 for zz in [z+.025,z+h-.026]:box(name+' gilt band',(x,y-d/2-.009,zz),(w*.72,.002,.003),'brass',.001)
 # Rotated baseline fits a full title along the spine.
 text(name+' spine title',titles[idx%12],(x-.012,y-d/2-.009,z+.047),.019,'paper',rot=(pi/2,-pi/2,0),align='LEFT')

def flatbook(name,x,y,z,idx=0,w=.25,d=.32,angle=0):
 def fn():
  box(name+' lower cover',(0,0,.003),(w,d,.006),colors[idx%6],.003)
  box(name+' page edges',(0,0,.019),(w-.008,d-.012,.026),'paper',.002)
  box(name+' upper cover',(0,0,.036),(w,d,.006),colors[idx%6],.003)
  words=titles[idx%12].replace(' ','\n')
  text(name+' cover title',words,(0,.045,.040),.028,'ink' if idx%6==4 else 'paper',rot=(0,0,0))
  text(name+' edition','COMPUTING\nREFERENCE',(0,-.09,.040),.012,'ink' if idx%6==4 else 'paper',rot=(0,0,0))
 local_group(name,(x,y,z),angle,fn)

def woodchair(n,x,y,angle=0):
 def fn():
  rounded_slab(n+' sculpted wood seat',(0,0,.51),.49,.49,.055,.09,'oak3')
  for xx in [-.18,.18]:
   for yy in [-.17,.17]:rod(n+' tapered leg',(xx*1.32,yy*1.32,.07),(xx,yy,.49),.024,'oak',16)
  for xx in [-.225,.225]:rod(n+' back stile',(xx,.17,.47),(xx,.24,1.09),.023,'oak',16)
  for xx in [-.15,-.075,0,.075,.15]:rod(n+' back spindle',(xx,.20,.59),(xx,.24,1.04),.011,'oak',12)
  curve(n+' shaped top rail',[(-.24,.23,1.045),(0,.26,1.10),(.24,.23,1.045)],.038,'oak3')
  for xx in [-.21,.21]:rod(n+' side stretcher',(xx,-.20,.25),(xx,.20,.25),.014,'oak',12)
 local_group(n,(x,y,0),angle,fn)

def laptop(variant=0):
 rounded_slab('Laptop beveled base',(0,0,.014),.34,.235,.022,.014,'steelDark')
 box('Recessed laptop keyboard',(0,.024,.026),(.295,.125,.003),'black',.004)
 for j in range(4):
  for i in range(11):box('Individual laptop key',(-.132+i*.0264,-.022+j*.028,.030),(.021,.022,.005),'steelDark',.002)
 box('Laptop trackpad',(0,-.074,.027),(.115,.047,.003),'steel',.004)
 def display():
  box('Laptop display shell',(0,0,.109),(.34,.014,.218),'steelDark',.009)
  box('Code editor screen',(0,-.009,.111),(.312,.003,.185),'screen',.003)
  box('Editor sidebar',(-.133,-.011,.11),(.035,.002,.17),'navy',.001)
  for i in range(11):
   z=.18-i*.013
   box('Syntax colored code',(-.02+(i%3)*.012,-.012,z),(.17-((i+variant)%4)*.022,.001,.003),['codeBlue','codeGreen','codeGold'][(i+variant)%3],0)
  text('Editor tab',['main.rs','search.py','query.sql','notes.ts'][variant%4],(-.10,-.013,.192),.009,'paper',align='LEFT')
 local_group('Open laptop hinge',(0,.108,.025),0,display).rotation_euler.x=-.20

def lounge(n,loc,angle,mat):
 def fn():
  for x in [-.30,.30]:
   for y in [-.31,.31]:rod(n+' wood leg',(x,y,.07),(x,y,.29),.028,'walnut2',16)
  pill(n+' base',(0,0,.35),(.82,.80,.23),mat)
  pill(n+' seat',(0,-.055,.49),(.67,.68,.19),'upholsteryLight')
  back=pill(n+' sloping back',(0,.30,.83),(.74,.20,.78),mat);back.rotation_euler.x=-.06
  for x in [-.37,.37]:pill(n+' soft arm',(x,0,.62),(.16,.79,.42),mat)
  curve(n+' seat piping',rounded_points(0,-.055,.54,.65,.66,.11),.003,'creamPiping',True,True)
 local_group(n,loc,angle,fn)


def open_study_book(name,x,y,z,idx=0,angle=0):
 """A small open CS textbook, with curved page edges and surface-following ink."""
 def fn():
  def height(u,layer=3):return .014+layer*.0025+.014*sin(pi*u)+.003*(1-u)
  for side in [-1,1]:
   box(name+' cloth cover',(side*.115,0,.004),(.23,.326,.008),colors[idx%6],.003)
   for layer in range(4):
    v=[];steps=20
    for yy in [-.152,.152]:
     for k in range(steps+1):
      u=k/steps;v.append((side*(.008+.213*u),yy,height(u,layer)))
    faces=[(k,k+1,steps+k+2,steps+k+1) for k in range(steps)]
    if side<0:faces=[tuple(reversed(f)) for f in faces]
    o=mesh(name+' curved page',v,faces,'paper')
    if layer==0:
     md=o.modifiers.new('Bound page block','SOLIDIFY');md.thickness=.009
   # Printed lines follow the curved paper instead of intersecting it.
   for line in range(9):
    yy=.060-line*.020;end=.77-(line%3)*.055
    curve(name+' printed text',[(side*(.008+.213*u),yy,height(u)+.0007) for u in [.16+(end-.16)*k/12 for k in range(13)]],.00065,'ink',poly=True)
   text(name+' running header',titles[idx%12].replace(' ','\n'),(side*.116,.107,.039),.012,'ink',rot=(0,0,0))
   text(name+' page number',str(24+idx*8+(1 if side>0 else 0)),(side*.182,-.140,height(.82)+.0008),.007,'ink',rot=(0,0,0))
  curve(name+' center binding',[(0,-.159,.016),(0,0,.018),(0,.159,.016)],.003,colors[idx%6])
  # A narrow fabric bookmark trails onto the desk, without dropping through it.
  curve(name+' ribbon bookmark',[(.003,.135,.022),(.003,-.125,.023),(.012,-.18,.005),(.024,-.20,.004)],.002,'clay')
  # Small diagram on the right page above the paragraph text.
  for u,yy in [(.30,.088),(.58,.072),(.82,.089)]:
   cyl(name+' diagram node',(.008+.213*u,yy,height(u)+.0015),.006,.001,'teal',16)
  curve(name+' diagram link',[(.008+.213*u,yy,height(u)+.0008) for u,yy in [(.30,.088),(.58,.072),(.82,.089)]],.0008,'teal',poly=True)
 return local_group(name,(x,y,z),angle,fn)
