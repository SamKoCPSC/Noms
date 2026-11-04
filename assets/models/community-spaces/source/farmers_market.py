collection('01 Rounded market square and pavers',offset=(0,0,0))
rounded_slab('Rounded stone market foundation',(0,0,-.12),8.8,7.2,.27,.28,'walnut2')
rounded_slab('Continuous sandstone paving',(0,0,.035),8.64,7.04,.065,.22,'stone')
for j in range(15):
 for i in range(17):
  x=-4.10+i*.51;y=-3.29+j*.47
  box('Individual market paving stone',(x,y,.078),(.497,.457,.045),['stone','ivory','linen'][(i*3+j)%3],.012)
collection('02 Low timber boundary fence',offset=(0,-.4,.1))
for i in range(29):
 x=-4.16+i*.296;box('Fence pale timber picket',(x,3.37,.67),(.25,.065,1.20),'oak3',.018)
for z in [.30,.97]:box('Fence horizontal support',(0,3.40,z),(8.55,.07,.08),'walnut2',.008)
for x in [-4.28,0,4.28]:box('Fence post',(x,3.37,.75),(.13,.13,1.4),'walnut2',.01)

def stall(n,x,y,w,color,caption):
 for dx in [-w/2+.06,w/2-.06]:
  for yy in [y-.30,y+.77]:
   box(n+' structural post',(x+dx,yy,1.49),(.095,.095,2.80),'walnut2',.015)
 box(n+' rear cross beam',(x,y+.77,2.84),(w+.12,.14,.16),'walnut2',.018)
 table(n+' produce counter',x,y-.25,1.02,w,.95,'oak')
 for i in range(int(w/.11)):
  xx=x-w/2+.055+i*.11;box(n+' counter front batten',(xx,y-.68,.66),(.075,.06,.60),'oak3',.015)
 # A cloth canopy with a gentle sag and stitched scalloped valance.
 steps=12
 for i in range(steps):
  a=x-w/2+i*w/steps;b=x-w/2+(i+1)*w/steps
  path=[(y+.87,2.97),(y+.48,2.91),(y+.02,2.75),(y-.46,2.63)]
  vs=[(xx,yy,zz-.025*sin(pi*(xx-x+w/2)/w))for xx in [a,b]for yy,zz in path]
  fs=[(j,j+1,4+j+1,4+j)for j in range(3)];o=mesh(n+' striped canvas canopy',vs,fs,color if i%2==0 else 'paper');md=o.modifiers.new('Canvas thickness','SOLIDIFY');md.thickness=.008
  vv=[(a,y-.47,2.64),(b,y-.47,2.64)]+[(b-(b-a)*j/8,y-.47,2.51-.035*sin(pi*j/8))for j in range(9)]
  mesh(n+' scalloped awning edge',vv,[tuple(range(len(vv)))],color if i%2==0 else 'paper')
 board(n+' name',caption,x,y-.51,2.28,w*.78,.34,color,.14)

collection('03 Orchard produce stall')
stall('Orchard stall',-2.13,2.10,3.60,'sageDark','ORCHARD & FIELD')
for k,kind in enumerate(['apple','lemon','pear','tomato']):
 x=-3.45+k*.88;crate('Fresh '+kind+' crate',x,1.70,1.07,.80,.57,.28)
 for j in range(3):
  for i in range(4):fruit('Market '+kind,x-.26+i*.17,1.55+j*.15,1.13,kind,.073)
 board('Produce variety label',kind.upper(),x,1.395,1.245,.40,.15,'ivory',.059)
collection('04 Bread honey and preserves stall')
stall('Pantry stall',2.02,2.10,3.60,'clay','BREAD & PRESERVES')
for x in [.80,1.35]:
 crate('Bread crate',x,1.72,1.07,.50,.62,.23)
 for i in range(3):baguette('Market baguette',x-.14+i*.14,1.70,1.13,.53,.05)
for i in range(3):
 for j in range(2):jar('Honey or jam',2.05+i*.43,1.57+j*.31,1.07,'honey' if i<2 else 'red',.085,.26)
board('Honey label','LOCAL HONEY',2.27,1.395,1.245,.64,.15,'ivory',.055)

collection('05 Seasonal vegetable island')
table('Central farm table',-.72,-.55,.84,3.48,1.12,'oak')
for i in range(3):crate('Island vegetable crate',-1.85+i*1.13,-.55,.89,1.03,.92,.30)
# Carrots have tapered roots and attached leafy crowns.
for i in range(10):
 x=-2.20+(i%5)*.17;y=-.78+(i//5)*.39
 lathe('Tapered fresh carrot',(x,y,.94),[(0,0),(.015,.015),(.06,.17),(.048,.23),(0,.24)],'orange',24)
 for j in range(3):curve('Carrot tops',[(x,y,1.17),(x+.025*(j-1),y+.025,1.28),(x+.055*(j-1),y+.055,1.34)],.006,'leaf')
for i in range(6):
 x=-1.07+(i%3)*.28;y=-.77+(i//3)*.40
 uv('Cabbage heart',(x,y,1.06),(.115,.12,.13),'sageLight',24,14)
 for j in range(5):
  a=2*pi*j/5;o=uv('Cabbage cupped outer leaf',(x+.075*cos(a),y+.075*sin(a),1.02),(.095,.045,.12),'leaf' if j%2 else 'sageLight',20,12);o.rotation_euler.z=a
for i in range(8):
 x=.06+(i%4)*.21;y=-.77+(i//4)*.41
 uv('Golden market potato',(x,y,1.0),(.09,.07,.065),'crustLight',20,10)
 for j in range(3):uv('Potato eye',(x+.035*cos(j*2),y+.03*sin(j*2),1.057),(.006,.004,.002),'crust',10,6)
for x,words in [(-1.85,'CARROTS'),(-.72,'CABBAGE'),(.41,'NEW POTATOES')]:board('Vegetable marker',words,x,-1.13,1.09,.65,.17,'sageDark',.057)

collection('06 Hanging market scales and farm signage')
rod('Scale hanging hook',(-.60,2.06,2.84),(-.60,2.06,2.52),.015,'brass')
o=cyl('Round scale body',(-.60,2.06,2.35),.17,.08,'steel',48);o.rotation_euler.x=pi/2
o=cyl('Scale ivory dial',(-.60,2.014,2.35),.148,.009,'paper',48);o.rotation_euler.x=pi/2
for i in range(12):
 a=i*2*pi/12;rod('Scale tick',(-.60+.12*sin(a),2.007,2.35+.12*cos(a)),(-.60+.137*sin(a),2.007,2.35+.137*cos(a)),.002,'ink',8)
rod('Scale needle',(-.60,1.999,2.35),(-.54,1.999,2.43),.003,'clay',8)
for dx in [-.16,.16]:curve('Scale bowl chain',[(-.60,2.06,2.18),(-.60+dx,2.06,1.91)],.004,'steel')
bowl('Hanging weighing pan',-.60,2.06,1.84,.21,'steel')
# Foreground A board rests on the paving.
for dx in [-.39,.39]:
 rod('A board front foot',(2.32+dx,-2.55,.10),(2.32+dx,-2.36,1.22),.025,'walnut2')
 rod('A board back foot',(2.32+dx,-1.98,.10),(2.32+dx,-2.36,1.22),.025,'walnut2')
board('Community market chalkboard','GROWN\nTO SHARE',2.32,-2.40,.80,.81,.82,'sageDark',.15)
text('Market chalkboard hours','SATURDAYS  /  9–2',(2.32,-2.447,.49),.057,'paper')

collection('07 Handcart baskets and eggs')
CX,CY=-3.13,-2.12
crate('Low wooden market cart',CX,CY,.38,1.24,.78,.47)
for xx in [CX-.63,CX+.63]:
 o=cyl('Cart rubber wheel',(xx,CY,.32),.23,.085,'charcoal',40);o.rotation_euler.y=pi/2
 o=cyl('Cart wheel wooden hub',(xx+(.05 if xx>CX else -.05),CY,.32),.12,.008,'oak',32);o.rotation_euler.y=pi/2
rod('Handcart steel axle',(CX-.67,CY,.32),(CX+.67,CY,.32),.021,'steelDark')
for xx in [CX-.48,CX+.48]:rod('Handcart handle rail',(xx,CY-.35,.50),(xx,CY-1.0,.98),.025,'walnut2')
rod('Handcart grip',(CX-.48,CY-1.0,.98),(CX+.48,CY-1.0,.98),.031,'walnut2')
for i in range(6):fruit('Cart oranges',CX-.37+(i%3)*.33,CY-.19+(i//3)*.33,.45,'orange',.12)
# Egg carton lid and two separated rows of eggs.
box('Open egg carton base',(1.98,-.45,.22),(.66,.40,.12),'linen',.032)
for j in range(2):
 for i in range(4):uv('Free range egg',(1.74+i*.16,-.54+j*.18,.31),(.058,.057,.078),'paper',20,12)
box('Open egg carton standing lid',(1.98,-.21,.42),(.66,.04,.34),'linen',.03)
for i in range(2):pot('Market herb pot',3.20+i*.40,-.15,.1,.15)
collection('08 Market bunting')
curve('Bunting cord',[(-4.0,3.11,3.14),(0,3.11,2.99),(4.0,3.11,3.14)],.009,'walnut2')
for i in range(17):
 x=-3.85+i*.48;z=2.99+.15*(x/4)**2;mesh('Fabric triangular pennant',[(x,3.11,z),(x+.29,3.11,z),(x+.145,3.11,z-.30)],[(0,1,2)],['sage','clay','honey','paper'][i%4])
finish('farmers-market','Noms — Farmers’ Market',12.5,(0,0,1.0))
