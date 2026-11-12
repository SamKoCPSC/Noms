"""Rework the neighborhood grocery into approved concept 03. Blender 4.3+."""
from pathlib import Path
import ast
HERE=Path(__file__).resolve().parent
SHARED=HERE.parent.parent/'campsite-market-grocery'/'source'
for helper in ['diorama_common','scene_details','new_details']:
    code=(SHARED/(helper+'.py')).read_text()
    if helper=='scene_details':
        code=code.replace("o.location=(10,-14,10)","o.location=(14,-22,18)")
        code=code.replace("scene.view_settings.exposure=.15","scene.view_settings.exposure=-.20")
        code=code.replace("scene.render.threads=2","scene.render.threads=4")
        code=code.replace("scene.cycles.samples=64","scene.cycles.samples=48")
        code=code.replace("1400 if not night else 650,6","3500 if not night else 650,10")
        code=code.replace("1000 if not night else 700,5","2500 if not night else 700,9")
        code=code.replace("1200 if not night else 450,5","3500 if not night else 450,9")
        code=code.replace("scene.render.resolution_x=1600;scene.render.resolution_y=1400","scene.render.resolution_x=1900;scene.render.resolution_y=1700")
        code=code.replace("scene.render.resolution_percentage=65;scene.cycles.samples=24","scene.render.resolution_percentage=48;scene.cycles.samples=16")
    exec(compile(code,str(SHARED/(helper+'.py')),'exec'))

# Direct data primitives avoid operator-driven dependency updates for shelf stock.
def box(n,loc,dims,m,b=.02):
    x,y,z=[v/2 for v in dims]
    o=mesh(n,[(-x,-y,-z),(x,-y,-z),(x,y,-z),(-x,y,-z),(-x,-y,z),(x,-y,z),(x,y,z),(-x,y,z)],
           [(3,2,1,0),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)],m)
    o.location=loc
    if b:bevel(o,b,2)
    return o

def cyl(n,loc,r,h,m,vertices=32):
    v=[(r*cos(i*2*pi/vertices),r*sin(i*2*pi/vertices),z) for z in [-h/2,h/2] for i in range(vertices)]
    f=[tuple(reversed(range(vertices))),tuple(range(vertices,vertices*2))]+[(i,(i+1)%vertices,(i+1)%vertices+vertices,i+vertices) for i in range(vertices)]
    o=mesh(n,v,f,m);o.location=loc;bevel(o,min(.004,h*.12),2);return smooth(o)

def uv(n,loc,sc,m,seg=20,rings=12):
    v=[(sin(pi*j/rings)*cos(2*pi*i/seg),sin(pi*j/rings)*sin(2*pi*i/seg),cos(pi*j/rings)) for j in range(rings+1) for i in range(seg)]
    f=[(j*seg+i,j*seg+(i+1)%seg,(j+1)*seg+(i+1)%seg,(j+1)*seg+i) for j in range(rings) for i in range(seg)]
    o=mesh(n,v,f,m);o.location=loc;o.scale=sc;return smooth(o)

def torus(n,loc,r,minor,m,rot=(0,0,0),steps=40):
    v=[((r+minor*cos(j*pi/4))*cos(i*2*pi/steps),(r+minor*cos(j*pi/4))*sin(i*2*pi/steps),minor*sin(j*pi/4)) for i in range(steps) for j in range(8)]
    f=[(i*8+j,((i+1)%steps)*8+j,((i+1)%steps)*8+(j+1)%8,i*8+(j+1)%8) for i in range(steps) for j in range(8)]
    o=mesh(n,v,f,m);o.location=loc;o.rotation_euler=rot;return smooth(o)

# Font labels are geometry in GLB, with no unnecessary extrusion on tiny print.
def text(n,words,loc,size=.12,mat='ivory',rot=(pi/2,0,0),align='CENTER'):
    d=bpy.data.curves.new(n,'FONT');d.body=words;d.align_x=align;d.size=size;d.resolution_u=2
    o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.rotation_euler=rot;assign(o,mat);return o

OUT=HERE.parent
for folder in ['models','previews']:(OUT/folder).mkdir(exist_ok=True)
M['arcade']=material('Matte cream arcade plaster','#EADBC7',.9)
M['niche']=material('Warm recessed plaster','#D9C3A4',.93)
M['tileA']=material('Warm ivory limestone tile','#DFD4C1',.85)
M['tileB']=material('Subtle beige limestone tile','#D8CAB5',.86)
M['cheese']=material('Golden cheese rind','#DDB551',.82)
M['cheeseCut']=material('Pale cut cheese','#EDDC9B',.85)
M['ham']=material('Rose cured ham','#BE7C70',.82)
M['salami']=material('Salami cut face','#9E564D',.8)
M['riceBag']=material('Rice sack canvas','#C9BA95',.96)
W,D,H=13,12,5.3
collection('01 Rounded platform and limestone floor',offset=(0,0,0))
rounded_slab('Rounded supermarket foundation',(0,0,-.14),W+.18,D+.18,.28,.28,'oak2')
rounded_slab('Continuous floor grout bed',(0,0,.017),W,D,.05,.20,'creamPiping')
for i in range(16):
    for j in range(15):
        box('Individual limestone floor tile',(-W/2+(i+.5)*W/16,-D/2+(j+.5)*D/15,.059),(W/16-.012,D/15-.012,.032),'tileA' if (i+j)%3 else 'tileB',.004)
collection('02 Cream deli arcade and rear wall',offset=(0,-.6,.3))
box('Rear continuous niche wall',(0,6,H/2),(13.15,.15,H),'niche',.02)
box('Rear continuous cornice',(0,5.91,H-.035),(13.2,.28,.16),'arcade',.012)
# Three deeply outlined arches, each with a solid upper spandrel rather than a floating arch.
for x,w in [(-6.375,.25),(-2.15,.40),(2.15,.40),(6.375,.25)]:
    box('Arcade supporting pier',(x,5.87,2.60), (w,.18,5.20),'arcade',.008)
for cx,r in [(-4.3,1.95),(0,1.95),(4.3,1.95)]:
    spring=2.6;front=5.78;back=5.96
    # Strip mesh fills from the curved intrados to the wall crown.
    N=48;verts=[]
    for y in [front,back]:
        for i in range(N+1):
            a=pi-i*pi/N;x=cx+r*cos(a);z=spring+r*sin(a)
            verts.extend([(x,y,z),(x,y,H-.10)])
    L=2*(N+1);faces=[]
    for i in range(N):
        a=2*i
        faces += [(a,a+2,a+3,a+1),(L+a+1,L+a+3,L+a+2,L+a),(a,L+a,L+a+2,a+2)]
    faces += [(0,1,L+1,L),(2*N,L+2*N,L+2*N+1,2*N+1)]
    mesh('Solid arched plaster spandrel',verts,faces,'arcade')
    curve('Soft raised arch molding',[(cx+r*cos(pi-i*pi/64),front-.015,spring+r*sin(pi-i*pi/64)) for i in range(65)],.026,'ivory',poly=True)
box('Rear oak skirting',(0,5.87,.19),(13,.055,.24),'oak2',.006)
collection('03 Asian pantry wall',offset=(.6,0,.2))
box('Left continuous cutaway wall',(-6.5,0,H/2),(.16,12,H),'arcade',.02)
box('Left sage department panel',(-6.39,1.0,2.70),(.045,9.2,4.68),'sageDark',.018)
box('Left top cream coping',(-6.49,0,H-.035),(.25,12,.16),'arcade',.012)
text('Asian pantry large wall lettering','ASIAN PANTRY',(-6.355,.90,4.25),.40,'paper',rot=(pi/2,0,pi/2))
text('Asian pantry subheading','RICE  /  NOODLES  /  TEA',(-6.351,.90,3.88),.13,'paper',rot=(pi/2,0,pi/2))

# Fittings use local front=-Y; rotating +90 degrees faces them into the room.
def asian_shelves():
    width=6.6
    box('Asian shelving solid back',(0,.31,1.75),(width,.06,3.2),'oak2',.012)
    box('Asian bookcase top canopy',(0,0,3.40),(6.68,.78,.06),'oak3',.01)
    for x in [-3.3,-1.1,1.1,3.3]:box('Asian shelving upright',(x,0,1.75),(.075,.72,3.25),'oak',.012)
    for z in [.18,.86,1.54,2.22,2.90]:
        box('Asian pantry shelf',(0,0,z),(width,.77,.065),'oak3',.009)
        box('Asian shelf price strip',(0,-.398,z),(width,.026,.065),'sage',.004)
    for row,z in enumerate([.215,.895,1.575,2.255]):
        for i in range(18):
            x=-3.10+i*.363
            if row==0:
                pill('Small rice sack',(x,0,z+.22),(.28,.34,.44),'riceBag')
                text('Rice sack label','RICE',(x,-.174,z+.23),.06,'ink')
            elif row==1:
                bottle('Soy sauce and sesame oil',x,0,z,'beefCrust' if i%2 else 'honey',.43,.09,'SOY' if i%2 else 'SESAME')
            else:pack('Asian noodle and tea box',x,0,z,.28,.32,.45,['clay','teal','honey','sageLight'][i%4],['NOODLES','GREEN TEA','UDON','MISO'][i%4])
    for i in range(9):
        x=-3+i*.74
        pack('Top shelf Asian biscuits',x,0,2.935,.38,.30,.38,['clay','sageLight','honey'][i%3],['MOCHI','RICE CRACKERS','TEA'][i%3])
collection('04 Stocked Asian pantry shelving',start=35,end=80)
local_group('Asian pantry fixture',(-5.99,1.10,0),pi/2,asian_shelves)
collection('05 Rice sacks and bamboo steamers',start=70,end=105)
for i in range(3):
    pill('Large rice sack',(-5.22,-2.65+i*.53,.38),(.52,.42,.62),'riceBag')
    text('Large rice sack print','RICE\n5 kg',(-4.945,-2.65+i*.53,.37),.09,'ink',rot=(pi/2,0,pi/2))
for x,y in [(-5.95,-1.4),(-5.95,-.7)]:
    for j in range(3):steamer('Bamboo steamer merchandise',x,y,3.43+j*.14,.23,False)

def refrigerator(width,doors,kind):
    box('Cold case insulated backing',(0,.32,1.53),(width,.08,2.92),'sageDark',.016)
    box('Cold case interior',(0,.265,1.50),(width-.12,.025,2.66),'ivory',.008)
    for x in [-width/2,width/2]:box('Cold case side casing',(x,0,1.53),(.075,.78,2.92),'steelDark',.012)
    for z in [.12,2.96]:box('Cold case top base',(0,0,z),(width,.78,.09),'sageDark',.012)
    for z in [.28,.86,1.44,2.02]:box('Cold case white shelf',(0,0,z),(width-.10,.60,.035),'ivory',.006)
    dw=width/doors
    for i in range(doors):
        x=-width/2+(i+.5)*dw
        box('Transparent refrigerator door',(x,-.410,1.55),(dw-.07,.012,2.52),'glass',.002)
        for xx in [x-dw/2+.015,x+dw/2-.015]:box('Refrigerator door stile',(xx,-.43,1.55),(.035,.045,2.60),'steelDark',.006)
        for z in [.25,2.85]:box('Refrigerator door rail',(x,-.43,z),(dw,.045,.035),'steelDark',.006)
        rod('Refrigerator door attached handle',(x+dw*.34,-.49,1.02),(x+dw*.34,-.49,2.02),.017,'steel')
        for row,z in enumerate([.30,.88,1.46,2.04]):
            for j in range(3):
                xx=x+(j-1)*dw*.25
                if kind=='DUMPLINGS':pack('Frozen dumpling pack',xx,0,z,.22,.31,.33,'paper' if row%2 else 'sageLight','GYOZA' if i%2 else 'BAO')
                elif i==0:pack('Milk carton',xx,0,z,.23,.25,.40,'paper','MILK')
                elif i==1:bottle('Fresh juice',xx,0,z,'orange' if row%2 else 'sageLight',.40,.09,'JUICE')
                else:bottle('Chilled beverage',xx,0,z,'teal' if j%2 else 'clay',.39,.085,'SODA')
    box('Cold case department header',(0,-.37,3.10),(width+.10,.16,.30),'sageDark',.015)
    text('Cold department title',kind,(0,-.457,3.07),.17,'paper')
collection('06 Dairy and beverages refrigerator bank',start=42,end=90)
local_group('Rear dairy refrigerator',(3.80,5.32,0),0,lambda:refrigerator(4.2,4,'DAIRY  /  BEVERAGES'))
collection('07 Asian frozen dumpling case',start=44,end=91)
local_group('Asian freezer fixture',(-5.99,-4.30,0),pi/2,lambda:refrigerator(1.80,2,'DUMPLINGS'))

collection('08 Deli back counter and pantry',start=40,end=85)
box('Deli preparation base',(-2.65,5.30,.62),(5.3,.70,1.10),'oak2',.025)
box('Deli preparation stone top',(-2.65,5.30,1.20),(5.42,.80,.08),'stone',.015)
for x in [-4.6,-3.7,-2.8,-1.9,-1]:
    box('Deli preparation cupboard door',(x,4.94,.64),(.84,.025,.91),'oak3',.012)
    rod('Deli cupboard brass pull',(x+.23,4.91,.72),(x+.23,4.91,.93),.012,'brass')
for x in [-4.7,-.65]:
    for z in [2.22,2.91]:
        box('Deli wall pantry shelf',(x,5.53,z),(1.20,.39,.05),'oak3',.008)
        for i in range(4):bottle('Deli olive oil bottle',x-.43+i*.28,5.52,z+.03,'sageDark',.39,.08,'OLIVE')
rod('Charcuterie hanging rail',(-3.8,5.49,3.20),(-1.7,5.49,3.20),.026,'steelDark')
for i in range(5):
    x=-3.65+i*.44
    curve('Attached charcuterie hook',[(x,5.49,3.23),(x,5.42,3.26),(x,5.42,3.02)],.009,'steel')
    uv('Hanging cured ham',(x,5.42,2.69),(.14,.13,.32),'beefCrust',20,14)
    uv('Hanging ham tied neck',(x,5.42,2.98),(.055,.055,.09),'fat',16,8)
text('Large deli wall lettering','DELI',(-2.65,5.755,4.83),.45,'ink')
text('Deli wall subheading','CHEESE  /  CHARCUTERIE  /  OLIVES',(-2.65,5.75,4.59),.115,'ink')

collection('09 Glazed European deli service counter',start=50,end=95)
DX,DY=-2.6,3.23
box('Deli recessed plinth',(DX,DY,.20),(5.25,1.06,.25),'walnut2',.02)
box('Deli oak cabinet',(DX,DY,.62),(5.45,1.15,.72),'oak3',.025)
for i in range(45):box('Deli cabinet fluted front',(DX-2.62+i*.119,DY-.585,.65),(.065,.025,.65),'oak2',.009)
box('Deli chilled display bed',(DX,DY,1.04),(5.40,1.10,.10),'ivory',.014)
# Sloped glazing: constant-thickness solid panel, joined to transparent end cheeks.
angle=-.22
o=box('Sloping deli front glass',(DX,DY-.47,1.43),(5.36,.014,.76),'glass',.002);o.rotation_euler.x=angle
box('Deli glass top',(DX,DY+.03,1.80),(5.38,.95,.014),'glass',.002)
for x in [DX-2.70,DX+2.70]:
    pts=[(x,DY-.55,1.06),(x,DY-.39,1.80),(x,DY+.50,1.80),(x,DY+.50,1.06)]
    o=mesh('Deli transparent end cheek',pts,[(0,1,2,3)],'glass');o.modifiers.new('Glass thickness','SOLIDIFY').thickness=.012
    rod('Deli rear glass support',(x,DY+.50,1.06),(x,DY+.50,1.80),.021,'steel')
rod('Deli front top rail',(DX-2.70,DY-.39,1.80),(DX+2.70,DY-.39,1.80),.016,'steel')
box('Deli staff service ledge',(DX,DY+.59,1.11),(5.5,.25,.06),'stone',.012)
for i in range(8):
    x=DX-2.32+i*.66
    box('Deli individual display tray',(x,DY,1.105),(.57,.72,.035),'steel',.008)
    if i<3:
        cyl('Whole cheese wheel',(x,DY+.10,1.24),.23,.23,'cheese',40)
        cyl('Cheese cut top',(x,DY+.10,1.358),.218,.008,'cheeseCut',40)
        # Triangular cut cheese wedge at front of each tray.
        verts=[(x-.20,DY-.32,z) for z in [1.13,1.27]]+[(x+.18,DY-.32,z) for z in [1.13,1.27]]+[(x,DY-.08,z) for z in [1.13,1.27]]
        mesh('Cut triangular cheese wedge',verts,[(0,2,4),(1,5,3),(0,1,3,2),(2,3,5,4),(4,5,1,0)],'cheeseCut')
    elif i<6:
        for j in range(6):
            yy=DY-.25+j*.10
            o=cyl('Layered charcuterie slice',(x,yy,1.15+j*.006),.17,.025,'ham' if i%2 else 'salami',32)
            for k in range(5):
                a=k*2*pi/5
                uv('Salami fat speck',(x+.105*cos(a),yy+.105*sin(a),1.168+j*.006),(.014,.011,.003),'fat',10,6)
    else:
        bowl('Deli olive serving bowl',x,DY,1.13,.23,'ivory')
        for j in range(13):
            a=j*2.4;r=.15*(j/13)**.5
            uv('Deli marinated olive',(x+r*cos(a),DY+r*sin(a),1.24),(.033,.045,.029),'sageDark' if j%2 else 'grape',12,8)
    board('Deli food label',['GOUDA','BRIE','CHEDDAR','PROSCIUTTO','SALAMI','HAM','OLIVES','ANTIPASTI'][i],x,DY-.50,1.14,.53,.105,'ivory',.035)

collection('10 Bakery baskets beside deli',start=68,end=105)
for z in [.42,1.16,1.90]:
    crate('Deli bread basket',-5.95,5.12,z,.68,.90,.30)
    for i in range(3):
        local_group('Basket baguette',(-6.16+i*.20,5.12,z+.15),pi/2,lambda:baguette('Scored bread loaf',0,0,0,.63,0))
for y in [4.75,5.49]:rod('Bread rack upright',(-5.95,y,.10),(-5.95,y,2.25),.028,'oak2')

# Long double-sided gondolas reuse the existing grocer's pantry product designs.
def aisle(index):
    length=4.2;width=.98
    box('Aisle recessed base',(0,0,.18),(width-.06,length-.06,.20),'walnut2',.014)
    box('Aisle center divider',(0,0,1.00),(.045,length,1.60),'sageDark',.01)
    for yy in [-length/2,length/2]:box('Aisle rounded oak end',(0,yy,1.0),(width,.075,1.62),'oak3',.018)
    for z in [.30,.83,1.36]:
        box('Aisle double sided shelf',(0,0,z),(width+.05,length,.045),'oak3',.008)
        for side in [-1,1]:
            box('Aisle shelf edge strip',(side*.53,0,z),(.022,length,.060),'sageDark',.004)
            for k in range(7):
                yy=-1.85+k*.61
                box('Aisle small price ticket',(side*.544,yy,z),(.002,.23,.037),'paper',0)
    for side in [-1,1]:
        for row,z in enumerate([.325,.855,1.385]):
            for j in range(11):
                yy=-1.80+j*.36
                labels=[['PASTA','FLOUR','OATS'],['BEANS','TOMATO','SOUP'],['CRACKERS','CEREAL','GRANOLA']]
                ma=['clay','teal','honey','sageLight','bookCream'][(j+row+index)%5]
                def item():
                    if row==1:tin('Grocery canned food',0,0,0,ma,labels[row][j%3],.10,.30)
                    else:pack('Grocery packaged food',0,0,0,.28,.30,.36,ma,labels[row][j%3])
                local_group('Individually faced grocery item',(side*.28,yy,z),side*pi/2,item)
    text('Aisle end category',['PANTRY','EVERYDAY','SNACKS'][index],(0,-2.143,1.53),.115,'paper')
for i,x in enumerate([-2.55,0,2.55]):
    collection(f'{11+i:02} Grocery aisle {i+1}',start=54+i*6,end=95+i*6)
    local_group('Grocery gondola '+str(i+1),(x,-.65,0),0,lambda i=i:aisle(i))

collection('14 Fresh produce island',start=59,end=100)
PX,PY=-2.65,-4.75
box('Produce island base',(PX,PY,.48),(4.80,1.52,.80),'sageDark',.025)
box('Produce island oak rim',(PX,PY,.91),(4.92,1.64,.08),'oak3',.012)
for i in range(6):
    x=PX-2+i*.80
    for j in range(2):
        y=PY-.39+j*.78;z=.95+j*.12
        crate('Individual produce crate',x,y,z,.74,.70,.22)
        for a in range(3):
            for b in range(3):
                xx=x-.23+a*.23;yy=y-.22+b*.22
                if i==0:
                    uv('Lettuce heart',(xx,yy,z+.12),(.12,.12,.13),'sageLight',16,10)
                    for k in range(4):
                        ang=k*pi/2
                        uv('Lettuce outer leaf',(xx+.06*cos(ang),yy+.06*sin(ang),z+.16),(.09,.07,.12),'leaf',12,8)
                else:fruit('Fresh market produce',xx,yy,z+.04,['apple','orange','lemon','pear','tomato'][i-1],.10)
    board('Produce price label',['LETTUCE','APPLES','ORANGES','LEMONS','PEARS','TOMATOES'][i],x,PY-.825,1.06,.69,.14,'ivory',.05)
# Reuse the detailed checkout assembly from the original grocery master at a compact scale.
original=(SHARED/'grocery_store.py').read_text()
checkout_code=original[original.index('CX,CY=-2.20,-2.36'):original.index("collection('09 Bread")]
checkout_code=checkout_code.replace('CX,CY=-2.20,-2.36','CX,CY=0,0').replace('NOMS / CORNER GROCER','NOMS MARKET')
for i,x in enumerate([2.00,4.15]):
    collection(f'{15+i:02} Checkout lane {i+1}',start=63+i*7,end=109+i*7)
    o=local_group('Compact checkout '+str(i+1),(x,-4.77,0),0,lambda:exec(compile(checkout_code,'original_grocer_checkout','exec'),globals()))
    o.scale=(.55,.90,1)
collection('17 Shopping carts and hand baskets',start=84,end=125)
cartcode=original[original.index('TX,TY=2.99,-2.32'):original.index("pack('Groceries in trolley'")]
cartcode=cartcode.replace('TX,TY=2.99,-2.32','TX,TY=0,0')
for i in range(2):
    local_group('Open wire shopping cart '+str(i+1),(5.85,-4.85+i*1.25,0),0,lambda:exec(compile(cartcode,'original_grocer_cart','exec'),globals()))
for j in range(3):
    crate('Stacked hand shopping basket',5.65,-2.78,.10+j*.16,.65,.48,.27)

# Uniform matte surfaces, preserving transmissive glazing and metal hardware.
for key,m in M.items():
    p=m.node_tree.nodes.get('Principled BSDF')
    if p and key not in ['glass','water','tea','cola','led']:
        p.inputs['Roughness'].default_value=max(p.inputs['Roughness'].default_value,.50 if p.inputs['Metallic'].default_value else .78)
scene=bpy.context.scene
scene['design_reference']='Approved concept 03 — Modern European Arcade'
scene['source_model']='campsite-market-grocery/neighborhood-grocery-store'
scene['departments']='European deli; Asian pantry and dumplings; produce; groceries; dairy and beverages'
scene['grocery_aisles']=3;scene['checkout_lanes']=2
finish('european-arcade-supermarket','Noms — Modern European Arcade Supermarket',21.0,(0,0,1.3))
