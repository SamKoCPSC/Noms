"""UBC reference-led reading hall, revision 4. Run with Blender 4.3+."""
from pathlib import Path
import sys
base=Path(__file__).resolve().parent
for folder in ('models','previews'):(base.parent/folder).mkdir(exist_ok=True)
exec(compile((base/'diorama_common.py').read_text(),str(base/'diorama_common.py'),'exec'))
exec(compile((base/'scene_details.py').read_text(),str(base/'scene_details.py'),'exec'))
random.seed(1348)
for k,h in {'maple':'#B99154','mapleLight':'#C7A270','goldWood':'#BE903E','limestone':'#AAA999','windowMetal':'#646762','upholstery':'#989B95','upholsteryLight':'#A9ABA5','burgundy':'#784F4E','screen':'#24313B','codeBlue':'#9FBED3','codeGreen':'#A1C5AD','codeGold':'#D8B980','lawn':'#819563','treeBark':'#797166','treeLeaf':'#94AA74','treeLeafLight':'#ADBB91','portraitGround':'#455854','portraitBlue':'#243F4B','portraitClaret':'#653D40','portraitSkin':'#B59379','portraitSkinLight':'#C4AA8A','robe':'#253B3D','silverRail':'#C0BEB0','glassBlue':'#568CAB','glassTeal':'#65A7A3','glassPurple':'#87779E','glassLime':'#B4BF75'}.items():M[k]=material(k,h,.82)
for k in ['glassBlue','glassTeal','glassPurple','glassLime']:
 p=M[k].node_tree.nodes.get('Principled BSDF');p.inputs['Transmission Weight'].default_value=.42;p.inputs['Roughness'].default_value=.28
M['windowBlue']=material('Matte blue simulated window','#9ABECF',.86)
M['windowBlueLight']=material('Matte pale blue simulated window','#A7C8D5',.86)
M['silverRail'].node_tree.nodes.get('Principled BSDF').inputs['Metallic'].default_value=.45
exec(compile((base/'study_furniture.py').read_text(),str(base/'study_furniture.py'),'exec'))

collection('01 Reading hall floor and exterior base',offset=(0,0,0))
rounded_slab('Walnut diorama plinth',(0,0,-.14),10.65,8.65,.28,.18,'walnut2')
rounded_slab('Continuous parquet floor',(0,0,.02),10.4,8.4,.04,.06,'gap')
for j in range(28):
 y=-4.2+(j+.5)*.30;cuts=[-5.2];x=-5.2+[.55,1.15,1.75][j%3]
 while x<5.2:cuts.append(x);x+=1.8
 cuts.append(5.2)
 for a,b in zip(cuts,cuts[1:]):box('Reading hall floorboard',((a+b)/2,y,.05),(b-a-.004,.295,.035),['maple','mapleLight','oak3'][(j+int(a*10))%3],.002)

collection('02 Double height paneled and portrait wall',offset=(0,0,.8))
box('Double height concrete wall',(0,4.20,3.70),(10.55,.18,7.4),'limestone',.025)
# Honey-coloured rectangular panels, with fine horizontal and vertical joints.
for i in range(5):
 for j in range(6):box('Honey maple wall panel',(-4.76+i*.82,4.085,.62+j*1.105),(.808,.065,1.093),'goldWood',.004)
box('Timber wall vertical reveal',(-.67,4.07,3.70),(.055,.09,7.3),'walnut2',.004)
box('Hall rear skirting',(0,4.065,.17),(10.4,.08,.22),'walnut2',.008)
box('Back wall head beam',(0,4.01,7.27),(10.55,.22,.18),'stone',.01)
for x in [-5.18,-.67,5.18]:box('Concrete wall pier',(x,4.01,3.70),(.20,.24,7.4),'stone',.01)

collection('03 Monumental glazed window wall',offset=(.55,0,.5))
box('Window base',(5.20,0,.19),(.18,8.4,.38),'stone',.01)
box('Long window sill',(5.08,0,.41),(.38,8.4,.085),'stone',.01)
box('Window head beam',(5.20,0,7.27),(.26,8.4,.20),'stone',.012)
for y in [-4.16,-2.77,-1.38,0,1.38,2.77,4.16]:box('Slender full height mullion',(5.15,y,3.80),(.12,.065,6.94),'windowMetal',.005)
for z in [.48,2.66,4.87,7.13]:box('Window horizontal transom',(5.15,0,z),(.12,8.34,.055),'windowMetal',.004)
ys=[-4.16,-2.77,-1.38,0,1.38,2.77,4.16];zs=[.51,2.64,2.69,4.84,4.90,7.10]
for pane_index,(a,b) in enumerate(zip(ys,ys[1:])):
 for za,zb in zip(zs[::2],zs[1::2]):box('Blue simulated window panel',(5.18,(a+b)/2,(za+zb)/2),(.018,b-a-.07,zb-za),'windowBlue' if pane_index%2 else 'windowBlueLight',.001)
# Narrow piers preserve the tall, airy proportions of the photographed hall.
for y in [-4.2,4.2]:box('Window end column',(5.12,y,3.70),(.28,.24,7.4),'stone',.012)

collection('05 Academic portrait gallery',offset=(0,-.45,0))
def portrait(x,z,w,h,idx):
 # Original stylized academic portraits, not replicas of identifiable people.
 y=3.965
 box('Portrait dark backing',(x,y,z),(w,.060,h),'walnut',.008)
 box('Portrait gilt inner mount',(x,y-.038,z),(w-.065,.018,h-.065),'brass' if idx%3 else 'paper',.004)
 box('Portrait painted canvas',(x,y-.052,z),(w-.12,.009,h-.12),['portraitGround','portraitBlue','portraitClaret'][idx%3],.002)
 # Framed paintings use shallow relief shapes so all artwork remains self contained.
 def shape(n,pts,ma,depth=.063):mesh(n,[(x+px*w,y-depth,z+pz*h) for px,pz in pts],[tuple(range(len(pts)))],ma)
 shape('Academic gown silhouette',[(-.32,-.42),(.32,-.42),(.28,-.12),(.11,.025),(-.11,.025),(-.29,-.13)],'robe')
 shape('Academic coloured stole',[(-.14,.015),(-.075,-.05),(-.12,-.37),(-.23,-.40),(-.21,-.13)],'brass' if idx%2 else 'clay',.067)
 shape('Academic right stole',[(.14,.015),(.075,-.05),(.12,-.37),(.23,-.40),(.21,-.13)],'brass' if idx%2 else 'clay',.068)
 uv('Portrait head',(x+.025*w*(idx%3-1),y-.077,z+.18*h),(.105*w,.012,.147*h),'portraitSkin' if idx%2 else 'portraitSkinLight',20,12)
 uv('Painted hair',(x,y-.08,z+.28*h),(.11*w,.008,.065*h),'walnut' if idx%3 else 'cream',20,10)
 shape('Painted shirt collar',[(-.075,.025),(0,-.04),(.075,.025),(0,.05)],'paper',.095)
 for dx in [-.036,.035]:uv('Painted eye',(x+dx*w,y-.096,z+.19*h),(.008*w,.003,.007*h),'ink',12,6)
 curve('Portrait mouth',[(x-.025*w,y-.097,z+.10*h),(x,y-.099,z+.09*h),(x+.029*w,y-.097,z+.105*h)],.003,'walnut')
 # Suggest hands holding a book, matching the academic portrait motif.
 box('Painted book',(x+.08*w,y-.074,z-.27*h),(.24*w,.012,.12*h),'bookCream',.005)
 for dx in [-.07,.20]:uv('Painted resting hand',(x+dx*w,y-.087,z-.25*h),(.045*w,.009,.025*h),'portraitSkinLight',16,8)
 box('Small portrait name plaque',(x,y-.04,z-h/2-.055),(.30,.018,.038),'brass',.002)
for row,z in enumerate([2.14,3.72,5.30]):
 for col,x in enumerate([.20,1.75,3.30]):portrait(x,z,1.12 if col!=1 else 1.30,1.32 if col!=1 else 1.42,row*3+col)
for col,x in enumerate([.45,2.75]):portrait(x,6.60,1.18,1.05,9+col)

collection('06 Full turn spiral staircase',offset=(0,-.6,.2))
CX,CY=-3.65,2.56;RI,RO=.48,1.36;N=24;rise=.16;start=-1.5*pi;sweep=2*pi
# Wedge treads and continuous curved stringers meet the upper landing.
def wedge(n,a,b,z,ma):
 pts=[(CX+r*cos(a+(b-a)*k/8),CY+r*sin(a+(b-a)*k/8),zz) for zz in [z-.10,z] for r in [RI,RO] for k in range(9)]
 f=[]
 for k in range(8):
  f += [(k,k+1,10+k,9+k),(18+k,27+k,28+k,19+k),(k,18+k,19+k,k+1),(9+k,10+k,28+k,27+k)]
 f += [(0,9,27,18),(8,26,35,17)]
 o=mesh(n,pts,f,ma);bevel(o,.007,2)
for i in range(N):
 a=start+sweep*i/N;b=start+sweep*(i+1)/N;z=.07+(i+1)*rise
 wedge('Curved stair tread',a,b,z,'stone')
 for r in [RI+.025,RO-.025]:
  ang=(a+b)/2
  rod('Stair vertical baluster',(CX+r*cos(ang),CY+r*sin(ang),z),(CX+r*cos(ang),CY+r*sin(ang),.07+rise+rise*(N-1)*(i+.5)/N+.93),.012,'silverRail',12)
for r in [RI,RO]:
 for dz,name,width in [(.93,'Curved handrail',.025),(.42,'Curved intermediate rail',.008),(-.06,'Continuous stair stringer',.065)]:
  pts=[]
  for i in range(101):
   a=start+sweep*i/100;z=.07+rise+rise*(N-1)*i/100
   pts.append((CX+r*cos(a),CY+r*sin(a),z+dz))
  curve(name,pts,width,'silverRail')
landing_z=.07+N*rise
rod('Upper handrail wall return',(-3.65,3.92,landing_z+.93),(-3.65,4.08,landing_z+.93),.025,'silverRail',16)

collection('04 Open second floor and landing',offset=(0,-.5,.3))
# The upper floor continues through the open left cutaway edge. No doorway.
# Its L-shaped footprint leaves the final stair wedge and walking route clear.
box('Second floor landing slab',(-4.425,3.60,landing_z-.141),(1.55,1.12,.21),'stone',.007)
box('Second floor landing timber',(-4.425,3.60,landing_z-.018),(1.55,1.12,.036),'oak3',.003)
box('Second floor gallery slab',(-4.78,2.195,landing_z-.141),(.84,1.69,.21),'stone',.007)
box('Second floor gallery timber',(-4.78,2.195,landing_z-.018),(.84,1.69,.036),'oak3',.003)
# Guard only the exposed interior edges; the left edge is the section cut.
rail_z=landing_z+.93
for x in [-4.36,-4.005,-3.65]:rod('Landing guard upright',(x,3.04,landing_z),(x,3.04,rail_z),.013,'silverRail',12)
rod('Landing guard top',(-4.36,3.04,rail_z),(-3.65,3.04,rail_z),.025,'silverRail',16)
for y in [1.35,1.77,2.19,2.61]:rod('Gallery guard upright',(-4.36,y,landing_z),(-4.36,y,rail_z),.013,'silverRail',12)
rod('Gallery long guard top',(-4.36,1.35,rail_z),(-4.36,3.04,rail_z),.025,'silverRail',16)
for x in [-5.16,-4.76]:rod('Gallery end guard upright',(x,1.35,landing_z),(x,1.35,rail_z),.013,'silverRail',12)
rod('Gallery end guard top',(-5.20,1.35,rail_z),(-4.36,1.35,rail_z),.025,'silverRail',16)
for x,y in [(-5.08,1.48),(-5.08,4.02),(-3.90,4.02)]:rod('Second floor supporting column',(x,y,.07),(x,y,landing_z-.21),.045,'silverRail',20)
box('Second floor rear beam',(-4.425,4.0,landing_z-.28),(1.55,.18,.18),'silverRail',.008)
# Persist explicit dimensions for later scene review and animation work.
bpy.context.scene['stair_rotation_degrees']=360
bpy.context.scene['stair_rise']=rise
bpy.context.scene['upper_floor_height']=landing_z

for row,y in enumerate([.90,-1.75]):
 collection(f'{7+row:02} Long communal reading table {row+1}',offset=(0,-1,.25))
 table('Long honey oak reading table',-.15,y,.79,4.45,1.12,'oak3')
 box('Table rear apron',(-.15,y+.48,.64),(4.25,.055,.18),'walnut2',.008)
 for x in [-1.70,-.15,1.40]:
  woodchair('Wooden library chair',x,y+.92,0)
  woodchair('Wooden library chair',x,y-.92,pi)

collection('09 Domed brass reading lamps',offset=(0,-.7,.4))
for y in [.90,-1.75]:
 for x in [-1.70,-.15,1.40]:
  cyl('Lamp weighted foot',(x,y,.866),.10,.035,'brass',32)
  rod('Lamp slender stem',(x,y,.88),(x,y,1.23),.012,'brass',16)
  lathe('Reference domed brass shade',(x,y,1.18),[(.17,0),(.175,.018),(.16,.06),(.12,.11),(.055,.14),(0,.145)],'brass',48)
  cyl('Lamp opal diffuser',(x,y,1.183),.15,.006,'led',32)

collection('10 Student books and coding laptop',offset=(0,-.8,.3))
for x,y,i in [(-1.7,-2.05,0),(-1.65,1.20,7),(1.87,1.20,3)]:flatbook('Computer science reading',x,y,.846,i,angle=.08*(i%3-1))
for x,y in [(-.1,.58),(-.80,-1.44),(1.48,1.18)]:
 box('Student notes',(x,y,.848),(.24,.30,.002),'paper',.001)
 for i in range(8):box('Study note line',(x,y-.10+i*.025,.850),(.16-(i%3)*.025,.002,.001),'ink',0)
 rod('Study pencil',(x+.15,y-.12,.856),(x+.12,y+.12,.856),.005,'honey',10)
# One laptop faces each chosen seat; opposite-side users see the screen fronts.
for idx,(x,y,angle) in enumerate([(-1.68,.59,-.06),(-.15,1.21,pi+.05),(-.12,-2.06,-.09),(-1.70,-1.44,pi-.08)]):
 local_group('Student coding laptop '+str(idx+1),(x,y,.846),angle,lambda idx=idx:laptop(idx))
for idx,(x,y,angle) in enumerate([(-1.04,.59,-.06),(-.78,1.21,pi+.05),(1.35,.59,.06),(1.38,-2.06,-.04),(-.15,-1.44,pi-.03)]):
 open_study_book('Open study textbook '+str(idx+1),x,y,.846,idx*2,angle)

for x,y in [(-2.15,-1.45),(1.78,.65)]:
 lathe('Reusable water bottle',(x,y,.846),[(0,0),(.037,0),(.039,.16),(.026,.19),(.025,.22),(0,.22)],'teal',32)
 cyl('Bottle cap',(x,y,1.075),.028,.018,'steelDark',24)
# Low unobtrusive table signs, like the reading-room reference.
for y in [.90,-1.75]:
 box('Reading table information stand',(.66,y,.935),(.14,.045,.17),'paper',.005)
 text('Quiet reading notice','QUIET\nSTUDY',(.66,y-.025,.967),.023,'ink')

collection('11 Window facing lounge chairs and tables',offset=(-.8,0,.2))
for i,y in enumerate([-3.12,-1.13,.86,2.85]):
 lounge('Window facing lounge chair',(3.97,y,0),pi/2,'upholstery')
 tx,ty=4.76,y
 box('Window reading table',(tx,ty,.62),(.48,.65,.035),'steelDark',.008)
 for dx in [-.19,.19]:
  for dy in [-.27,.27]:rod('Window table slim leg',(tx+dx,ty+dy,.07),(tx+dx,ty+dy,.60),.015,'windowMetal',12)
 flatbook('Window reading book',tx,ty,.641,idx=i+4,w=.23,d=.29,angle=pi/2)
 if i==0:mug('Window coffee',tx+.03,ty+.23,.641,.032,.075)

collection('12 Suspended coloured art glass',offset=(0,-.35,.5))
# Thin suspension members span to the structural walls, rather than floating.
rod('Art glass rigging side rail',(-.55,-2.6,7.16),(-.55,4.03,7.16),.006,'steelDark',12)
for y in [-2.6,.15,2.85]:rod('Art glass rigging cross rail',(-.55,y,7.16),(5.09,y,7.16),.006,'steelDark',12)
# Staggered vertical ribbon panels, with faceted translucent blue/green/purple glass.
for i,(x,y,w,h) in enumerate([(4.60,-3.5,.45,2.6),(4.65,-2.25,.50,3.5),(4.60,-.65,.43,2.9),(4.65,.65,.49,3.8),(4.60,2.15,.46,3.0),(4.62,3.45,.44,2.5),(2.9,2.85,.42,2.3),(1.5,2.85,.45,2.8),(.15,2.85,.40,2.2),(2.75,.15,.42,1.9),(1.25,.15,.38,2.2)]):
 top=6.96-.12*(i%3);bottom=top-h
 # Panels parallel to the glazing for the window row, facing the room for inner rows.
 angle=-pi/2 if x>4 else 0
 def ribbon():
  pts=[(-w/2,0,0),(w/2,0,0),(w/2,0,-h+.12),(w*.29,0,-h),(-w*.29,0,-h),(-w/2,0,-h+.12)]
  o=mesh('Hanging glass ribbon',pts,[(0,1,2,3,4,5)],['glassBlue','glassTeal','glassPurple','glassLime'][i%4]);md=o.modifiers.new('Thin art glass','SOLIDIFY');md.thickness=.009
  for k in range(7):
   z=-.10-k*(h-.2)/7
   mesh('Art glass triangular facet',[(-w*.46,-.008,z),(w*.46,-.008,z-.13),((-1 if k%2 else 1)*w*.35,-.008,z-.35)],[(0,1,2)],['glassTeal','glassPurple','glassLime','glassBlue'][(i+k)%4])
   curve('Etched glass diagonal',[(-w*.47,-.012,z-.02),(w*.47,-.012,z-.35)],.006,'paper',poly=True)
   curve('Art glass curved engraving',[(-w*.38,-.014,z-.02),(w*.18,-.014,z-.15),(-w*.15,-.014,z-.31)],.005,'paper')
  for xx in [-w*.28,w*.28]:rod('Ribbon suspension wire',(xx,0,0),(xx,0,7.16-top),.0025,'steelDark',8)
 local_group('Suspended art glass strip',(x,y,top),angle,ribbon)
 # A short transverse support connects the window row to the nearest horizontal rigging.
 if x>4:rod('Window ribbon hanger support',(5.1,y,7.16),(4.45,y,7.16),.007,'steelDark',12)

collection('13 Low reference shelves and returns',offset=(0,-.6,0))
# Books remain in the design, but no longer dominate the tall portrait wall.
for x in [.48,2.12]:
 box('Low reference case back',(x,3.96,.66),(1.52,.055,1.18),'walnut',.008)
 for xx in [x-.78,x+.78]:box('Low bookcase side',(xx,3.76,.66),(.065,.47,1.2),'oak3',.008)
 for z in [.12,.53,1.16]:box('Low bookcase shelf',(x,3.74,z),(1.62,.51,.05),'oak3',.008)
 for j,z in enumerate([.15,.565]):
  for i in range(15):volume('Computer science reference',x-.65+i*.085,3.72,z,i+j*4,w=.055,h=.27+(i%3)*.025,d=.22)
for i in range(3):flatbook('Low shelf reading stack',.22,3.70,1.19+i*.041,i,w=.26,d=.31)
# Return cart placed in the clear foreground beside the tables.
TX,TY=-3.9,-3.20
for z in [.25,.61]:
 box('Returns trolley shelf',(TX,TY,z),(.78,.40,.035),'oak3',.008)
 box('Returns trolley back',(TX,TY+.18,z+.07),(.78,.025,.17),'maple',.005)
for x in [TX-.35,TX+.35]:
 for y in [TY-.16,TY+.16]:
  rod('Return trolley frame',(x,y,.12),(x,y,.88),.015,'silverRail',12)
  o=cyl('Return trolley caster',(x,y,.11),.048,.027,'black',20);o.rotation_euler.x=pi/2
rod('Return trolley grip',(TX-.35,TY+.16,.88),(TX+.35,TY+.16,.88),.02,'walnut2',12)
for i in range(8):volume('Returned CS textbook',TX-.28+i*.075,TY,.63,i,w=.055,h=.24,d=.23)

s=bpy.context.scene;s['reconstruction']='Revision 4: four coding laptops and five open CS textbooks on the study tables.'
s['design_features']='Double-height blue window panels; honey timber wall panels; academic portraits; suspended art glass; 360-degree stair to open second floor; window-facing chairs; long reading tables.'
s['reading_chairs']=12;s['window_chairs']=4;s['portraits']=11;s['art_glass_ribbons']=11;s['stair_treads']=24;s['study_laptops']=4;s['open_study_books']=5
s['portrait_note']='Original stylized academic portraits; no claimed likenesses of the photographed people.'
finish('university-library','Noms — UBC-inspired Reading Hall',15.9,(.35,0,2.85),revision=4,camera_position=(-13,-18,13),render_threads=4,samples=48)
