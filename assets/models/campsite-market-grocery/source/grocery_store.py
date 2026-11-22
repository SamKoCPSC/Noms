shell(8.2,7.2,3.60,tiles=True,wall='wall')
# A restrained ivory-and-stone tile palette distinguishes the grocer from the bakery.
for o in bpy.data.objects:
 if o.type=='MESH' and o.name.startswith('Handmade floor tile') and o.data.materials[0].name=='clay':o.data.materials[0]=M['linen']

collection('04 Rear pantry shelving')
SX,SY=-2.15,3.17
box('Pantry shelf backing',(SX,3.46,1.59),(3.36,.08,2.96),'sageDark',.022)
for x in [-3.87,-.43]:box('Pantry cabinet end',(x,SY,1.59),(.08,.62,2.96),'oak2',.012)
for z in [.18,.82,1.46,2.10,2.74]:box('Pantry solid shelf',(SX,SY,z),(3.52,.64,.06),'oak',.012)
box('Pantry header fascia',(SX,2.823,2.99),(3.52,.055,.37),'sageDark',.016)
text('Pantry header lettering','EVERYDAY GOOD THINGS',(SX,2.786,2.97),.13,'paper')
for j,z in enumerate([.22,.86,1.50,2.14]):
 for i in range(9):
  x=-3.65+i*.37
  if j==0:pack('Rice and flour bag',x,3.11,z,.24,.23,.42,['linen','paper','sageLight'][i%3],['RICE','FLOUR','OATS'][i%3])
  elif j==1:
   for k in range(2):tin('Pantry tinned food',x,3.07+k*.20,z,'clay' if i%2 else 'teal',['BEANS','SOUP','TOMATO'][i%3],.085,.25)
  elif j==2:jar('Preserve shelf jar',x,3.11,z,['honey','red','leaf'][i%3],.095,.30)
  else:pack('Pasta and cereal carton',x,3.11,z,.23,.22,.40,['clay','honey','teal'][i%3],['PASTA','OATS','CEREAL'][i%3])
 for i in range(4):
  x=-3.46+i*.85;box('Shelf edge price tag',(x,2.843,z-.018),(.23,.006,.035),'paper',.002)

collection('05 Chilled grocery refrigerator')
RX,RY=1.86,3.10
box('Fridge rear insulated panel',(RX,RY+.34,1.54),(3.12,.07,2.91),'sageDark',.025)
for x in [RX-1.52,RX+1.52]:box('Fridge insulated side panel',(x,RY,1.54),(.08,.75,2.91),'sageDark',.025)
for z in [.13,2.96]:box('Fridge insulated top or base',(RX,RY,z),(3.12,.75,.09),'sageDark',.025)
box('Fridge interior bright back',(RX,RY+.29,1.56),(2.93,.025,2.55),'ivory',.012)
for x in [RX-1.51,RX-.50,RX+.50,RX+1.51]:box('Fridge door jamb',(x,RY-.07,1.56),(.055,.65,2.58),'steelDark',.012)
for z in [.26,.82,1.38,1.94,2.50]:box('Cold case shelf',(RX,RY-.04,z),(2.99,.57,.029),'ivory',.006)
for i in range(3):
 x=RX-1.01+i*1.01
 # Thin glazing and frames retain a clear view of the stocked shelves.
 box('Refrigerator glass door',(x,RY-.405,1.57),(.92,.014,2.51),'glass',.002)
 for xx in [x-.47,x+.47]:box('Fridge narrow door stile',(xx,RY-.43,1.57),(.025,.036,2.56),'steel',.007)
 for z in [.29,2.85]:box('Fridge horizontal door rail',(x,RY-.43,z),(.96,.036,.027),'steel',.007)
 rod('Refrigerator long vertical pull',(x+.35,RY-.492,1.00),(x+.35,RY-.492,2.03),.017,'steelDark')
 for j,z in enumerate([.29,.85,1.41,1.97]):
  for k in range(3):
   xx=x-.28+k*.28
   if j in [0,1]:bottle('Chilled milk or juice',xx,RY-.08,z,'paper' if i==0 else ['orange','teal','sageLight'][i],.37,.085,'MILK' if i==0 else 'JUICE')
   elif j==2:pack('Cold dairy carton',xx,RY-.10,z,.20,.19,.34,['sageLight','paper','teal'][i],'DAIRY')
   else:
    for h in range(2):cyl('Stacked yogurt tub',(xx,RY-.10,z+.072+h*.12),.09,.12,'paper',32);cyl('Yogurt tub pastel lid',(xx,RY-.10,z+.137+h*.12),.093,.012,['clay','sageLight','teal'][i],32)
box('Chiller header',(RX,RY-.37,3.03),(3.16,.14,.29),'sageDark',.02)
text('Chiller lettering','FRESH  /  CHILLED',(RX,RY-.447,3.02),.15,'paper')
for i in range(3):area('Refrigerator interior soft light',(RX-1.0+i,RY-.22,2.75),7,.40,(.92,1,1),(RX-1.0+i,RY-.04,1.2))

collection('06 Fresh produce display')
PX,PY=-2.66,.73
box('Produce plinth',(PX,PY,.18),(1.72,1.40,.18),'walnut',.02)
box('Produce counter body',(PX,PY,.60),(1.80,1.47,.72),'sage',.035)
rounded_slab('Produce counter top',(PX,PY,.99),1.91,1.58,.08,.07,'oak')
for i,kind in enumerate(['apple','orange','lemon']):
 x=PX-.61+i*.61;crate('Produce display bin',x,PY,1.03,.54,1.38,.25)
 for j in range(5):
  for k in range(2):fruit('Fresh '+kind,x-.13+k*.25,PY-.49+j*.245,1.10,kind,.088)
 board('Fresh produce price',kind.upper(),x,PY-.71,1.22,.47,.14,'ivory',.047)
for x in [PX-.55,PX+.55]:
 box('Produce cabinet front',(x,PY-.756,.62),(.51,.034,.61),'sageLight',.012)
 rod('Produce cabinet handle',(x-.11,PY-.791,.70),(x+.11,PY-.791,.70),.010,'brass')
board('Produce hanging sign','FRESH PICKS',PX,.73,1.75,1.58,.27,'sageDark',.12)
for x in [PX-.72,PX+.72]:rod('Produce sign support',(x,.77,1.06),(x,.77,1.90),.016,'brass')

collection('07 Central double sided grocery island')
IX,IY=.55,-.10
box('Grocery aisle recessed base',(IX,IY,.18),(2.94,1.09,.15),'walnut',.015)
box('Grocery island center divider',(IX,IY,.79),(3.06,.055,1.16),'sageDark',.015)
for x in [IX-1.54,IX+1.54]:box('Grocery island end panel',(x,IY,.79),(.065,1.28,1.20),'oak',.02)
for z in [.27,.75,1.23]:box('Double sided aisle shelf',(IX,IY,z),(3.10,1.30,.055),'oak',.012)
for side in [-1,1]:
 for row,z in enumerate([.30,.78]):
  for i in range(7):
   x=IX-1.29+i*.43;y=IY+side*.36
   def item():
    if row==0:bottle('Aisle olive oil',0,0,0,'sageDark',.34,.075,'OLIVE')
    else:pack('Aisle dry goods',0,0,0,.25,.22,.34,['clay','honey','teal','sageLight'][i%4],['RICE','PASTA','OATS','FLOUR'][i%4])
   local_group('Shelf stock item',(x,y,z),0 if side==-1 else pi,item)
 # Price strips sit on the shelf edge rather than hovering in the aisle.
 for z in [.27,.75,1.23]:
  box('Shelf edge sage trim',(IX,IY+side*.662,z),(3.08,.025,.07),'sage',.006)
  for i in range(5):box('Grocery price tag',(IX-1.13+i*.565,IY+side*.677,z),(.25,.003,.04),'paper',.002)
for i in range(6):
 x=IX-1.19+i*.47;pack('Top shelf snack packet',x,IY,1.26,.31,.28,.32,['clay','sageLight','honey'][i%3],['CRACKERS','GRANOLA','SNACKS'][i%3])
text('Island end aisle label','PANTRY',(IX,IY-.683,1.25),.062,'paper')

collection('08 Neighborhood checkout counter')
CX,CY=-2.20,-2.36
box('Checkout lower body',(CX,CY,.59),(2.90,.92,1.00),'sageDark',.045)
for i in range(29):box('Checkout fluted timber front',(CX-1.39+i*.10,CY-.472,.62),(.062,.035,.90),'oak',.018)
rounded_slab('Checkout stone counter',(CX,CY,1.14),3.03,1.04,.10,.09,'stone')
rounded_slab('Checkout belt recess',(CX+.56,CY,1.199),1.58,.70,.016,.035,'black')
for i in range(15):box('Checkout belt fine groove',(CX-.13+i*.095,CY,1.208),(.002,.63,.001),'steelDark',0)
box('Checkout scanner well',(CX-.39,CY,1.202),(.34,.42,.012),'steelDark',.018)
box('Scanner glass window',(CX-.39,CY,1.211),(.25,.31,.005),'teal',.01)
# The register screen faces the camera/customer and stands on a supported arm.
box('Register base',(CX-1.05,CY+.13,1.24),(.35,.30,.07),'steelDark',.025)
rod('Register upright',(CX-1.05,CY+.13,1.27),(CX-1.05,CY+.13,1.54),.036,'steelDark')
box('Register screen housing',(CX-1.05,CY+.12,1.62),(.55,.055,.40),'steelDark',.025)
box('Register screen',(CX-1.05,CY+.087,1.62),(.49,.007,.34),'paper',.012)
text('Register Noms heading','NOMS',(CX-1.05,CY+.078,1.69),.062,'sageDark')
text('Register total','THANK YOU',(CX-1.05,CY+.077,1.57),.044,'ink')
box('Contactless card reader',(CX-.82,CY-.29,1.235),(.19,.25,.07),'steelDark',.025)
box('Card reader display',(CX-.82,CY-.26,1.275),(.13,.09,.006),'sky',.008)
for i in range(3):box('Card reader key',(CX-.86+i*.04,CY-.34,1.277),(.024,.022,.005),'paper',.004)
pack('Shopping cereal carton',CX+.15,CY-.02,1.215,.24,.22,.35,'clay','CEREAL')
bottle('Shopping olive oil bottle',CX+.55,CY+.03,1.215,'sageDark',.31,.07,'OLIVE')
fruit('Checkout apple',CX+.91,CY-.11,1.215,'apple',.095)
box('Receipt printer',(CX-.42,CY+.30,1.29),(.25,.23,.19),'ivory',.022)
box('Receipt paper',(CX-.42,CY+.17,1.37),(.12,.16,.004),'paper',.001)
text('Grocer front counter branding','NOMS / CORNER GROCER',(CX,CY-.499,.65),.13,'ivory')

collection('09 Bread baskets and shopping trolley')
# An open bread stand near the entrance, clear of the checkout and main aisle.
BX,BY=3.02,1.02
crate('Tall bread display box',BX,BY,.12,1.00,.71,.65)
for i in range(5):
 x=BX-.34+i*.17
 def bread():baguette('Grocer crusty baguette',0,0,0,.69,0)
 o=local_group('Leaning bread in basket',(x,BY-.09,.63),0,bread);o.rotation_euler.x=1.04
board('Bread display label','BAKED TODAY',BX,BY-.38,.56,.76,.18,'sageDark',.068)
# Open metal shopping trolley with actual gaps between the basket rails.
TX,TY=2.99,-2.32
for x in [TX-.30,TX+.30]:
 for y in [TY-.40,TY+.40]:
  o=cyl('Shopping trolley caster',(x,y,.18),.08,.045,'black',24);o.rotation_euler.x=pi/2
  rod('Caster mounting fork',(x,y,.23),(x,y,.32),.017,'steel')
curve('Trolley lower frame',[(TX-.31,TY-.43,.30),(TX-.31,TY+.43,.30),(TX+.31,TY+.43,.30),(TX+.31,TY-.43,.30)],.017,'steel',True,True)
for x in [TX-.30,TX+.30]:rod('Trolley supporting upright',(x,TY+.32,.30),(x,TY+.32,1.02),.018,'steel')
for z in [.60,.76,.92,1.08]:curve('Trolley basket horizontal rail',rounded_points(TX,TY,z,.70,.98,.07),.008,'steel',True,True)
for i in range(8):
 y=TY-.40+i*.115
 for x in [TX-.35,TX+.35]:rod('Trolley basket open side wire',(x,y,.60),(x,y,1.08),.005,'steel',10)
for i in range(6):
 x=TX-.28+i*.112
 for y in [TY-.49,TY+.49]:rod('Trolley basket end wire',(x,y,.60),(x,y,1.08),.005,'steel',10)
 rod('Trolley open floor wire',(x,TY-.44,.60),(x,TY+.44,.60),.006,'steel',10)
curve('Trolley handle frame',[(TX-.31,TY+.40,1.03),(TX-.31,TY+.66,1.12),(TX+.31,TY+.66,1.12),(TX+.31,TY+.40,1.03)],.017,'steel',poly=True)
rod('Trolley sage handle grip',(TX-.29,TY+.66,1.12),(TX+.29,TY+.66,1.12),.027,'sageDark')
pack('Groceries in trolley',TX-.14,TY+.1,.61,.25,.23,.34,'honey','OATS')
crate('Stacked hand baskets',.59,-2.78,.11,.75,.49,.28)

collection('10 Grocer signs and small details')
board('Grocer welcome header','NOMS / YOUR NEIGHBORHOOD GROCER',0,3.51,3.39,5.56,.31,'ivory',.13)
pot('Grocery window herb',-3.88,.10,1.345,.12)
finish('neighborhood-grocery-store','Noms — Neighborhood Grocery Store',12.2,(0,0,1.10))
