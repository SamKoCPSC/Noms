"""Build approved concept 05: shared corporate workspace and pantry. Blender 4.3+."""
from pathlib import Path
HERE=Path(__file__).resolve().parent
SHARED=HERE.parent.parent/'campsite-market-grocery'/'source'
for helper in ['diorama_common','scene_details','new_details']:
    code=(SHARED/(helper+'.py')).read_text()
    if helper=='scene_details':
        code=code.replace('scene.cycles.max_bounces=7;scene.cycles.transmission_bounces=6','scene.cycles.max_bounces=20;scene.cycles.transmission_bounces=16')
        code=code.replace('scene.render.threads=2','scene.render.threads=4')
        code=code.replace('o.location=(10,-14,10)','o.location=(11,-15,12)')
        code=code.replace('scene.render.resolution_x=1600;scene.render.resolution_y=1400','scene.render.resolution_x=1800;scene.render.resolution_y=1600')
        code=code.replace('scene.render.resolution_percentage=65;scene.cycles.samples=24','scene.render.resolution_percentage=50;scene.cycles.samples=16')
    exec(compile(code,str(SHARED/(helper+'.py')),'exec'))
exec(compile((HERE/'primitives.py').read_text(),str(HERE/'primitives.py'),'exec'))

OUT=HERE.parent
for folder in ['models','previews']:(OUT/folder).mkdir(exist_ok=True)
for k,h in [('officeWall','#E5DFD2'),('carpet','#BDB5A7'),('carpetAlt','#B7B0A3'),('felt','#8A927C'),('taskMesh','#555D59'),('screen','#263B49'),('codeBlue','#99C5DC'),('codeGold','#D9BC7E'),('codeGreen','#8EB7A4'),('steelSink','#9EA7A5'),('ham','#C6877D'),('cheese','#E5C36D'),('jugBlue','#9BBAC7')]:
    M[k]=material(k,h,.88 if k not in ['screen','steelSink','jugBlue'] else .5)
M['water'].node_tree.nodes.get('Principled BSDF').inputs['IOR'].default_value=1.333
M['waterJug']=material('Translucent blue cooler bottle','#A9C5D3',.18,0,.65)
W,D,H=8.8,7.6,3.6
collection('01 Rounded foundation and carpet floor',offset=(0,0,0))
rounded_slab('Rounded oak office platform',(0,0,-.13),W+.16,D+.16,.26,.22,'oak2')
rounded_slab('Continuous office floor',(0,0,.018),W,D,.05,.16,'carpet')
for i in range(16):
    for j in range(14):
        x=-W/2+(i+.5)*W/16;y=-D/2+(j+.5)*D/14
        box('Commercial carpet tile',(x,y,.049),(W/16-.003,D/14-.003,.020),'carpet' if (i+j)%3 else 'carpetAlt',.001)
# A practical washable strip under the pantry.
for i in range(11):
    box('Pantry resilient floor tile',(-1.80+i*.56,3.07,.067),(.555,1.36,.025),'stone',.002)
collection('02 Rear kitchenette wall',offset=(0,-.45,.2))
box('Continuous rear office wall',(0,3.80,H/2),(8.96,.15,H),'officeWall',.015)
box('Rear oak skirting',(0,3.705,.16),(8.8,.045,.20),'oak2',.006)
collection('03 Window wall',offset=(.45,0,.2))
# An actual opening in the wall, with no opaque wall behind the glazing.
ya,yb,za,zb=.73,3.13,1.28,3.32
for cy,dy,cz,dz in [(0,D,za/2,za),(0,D,(H+zb)/2,H-zb),((-3.8+ya)/2,ya+3.8,(za+zb)/2,zb-za),((yb+3.8)/2,3.8-yb,(za+zb)/2,zb-za)]:
    box('Left wall around window',(-4.4,cy,cz),(.15,dy,dz),'officeWall',.008)
box('Left oak skirting',(-4.305,0,.16),(.045,7.6,.20),'oak2',.006)
box('Daylight glazing',(-4.405,(ya+yb)/2,(za+zb)/2),(.015,yb-ya,zb-za),'sky',.003)
for y in [ya,(ya+yb)/2,yb]:box('Slim window upright',(-4.32,y,(za+zb)/2),(.12,.045,zb-za+.10),'steelDark',.006)
for z in [za,zb]:box('Window horizontal frame',(-4.32,(ya+yb)/2,z),(.12,yb-ya+.10,.045),'steelDark',.006)
box('Oak window sill',(-4.25,(ya+yb)/2,za),(.32,yb-ya+.18,.065),'oak3',.012)
collection('04 Humorous framed company posters',start=30,end=65)
def poster_ink(n,points,r=.015,ma='sageDark'):
    o=curve(n,[(x,0,z) for x,z in points],r,ma,poly=True)
    o.location.y=-.031;o.scale.y=.10
def poster_head(n,x,z,r=.055,ma='clay'):
    o=cyl(n,(x,-.032,z),r,.003,ma,40)
    o.rotation_euler.x=pi/2
def offline_poster():
    # Editable vector-like geometry keeps both the lettering and joke sharp in GLB.
    box('Offline poster oak frame',(0,0,0),(1.80,.045,1.30),'oak2',.009)
    box('Offline poster cream paper',(0,-.025,0),(1.71,.004,1.21),'paper',.002)
    text('Offline poster headline',"LET'S TAKE THIS",(0,-.029,.36),.135,'sageDark')
    text('Offline poster punchline','OFFLINE.',(0,-.029,.13),.22,'clay')
    # One foot remains on the line; the other steps down beyond its endpoint.
    def ink(n,points,r=.015,ma='sageDark'):
        o=curve(n,[(x,0,z) for x,z in points],r,ma,poly=True)
        o.location.y=-.031;o.scale.y=.10
    ink('Poster horizontal line',[(-.68,-.28),(.06,-.28)],.009)
    head=cyl('Poster stepping figure head',(.24,-.032,-.035),.059,.003,'clay',40)
    head.rotation_euler.x=pi/2
    ink('Poster figure torso',[(.23,-.115),(.18,-.24)],.021,'clay')
    ink('Poster balancing back arm',[(.215,-.135),(.11,-.16),(.055,-.105)],.013,'clay')
    ink('Poster balancing front arm',[(.215,-.135),(.32,-.19),(.39,-.14)],.013,'clay')
    ink('Poster foot still on line',[(.18,-.24),(.10,-.20),(.025,-.265),(-.03,-.265)],.015,'clay')
    ink('Poster leg stepping off line',[(.18,-.24),(.29,-.33),(.37,-.46),(.46,-.46)],.015,'clay')
local_group('Lets take this offline framed poster',(-4.28,-2.62,2.40),pi/2,offline_poster)
def circle_back_poster():
    box('Circle back poster oak frame',(0,0,0),(1.80,.045,1.30),'oak2',.009)
    box('Circle back poster cream paper',(0,-.025,0),(1.71,.004,1.21),'paper',.002)
    text('Circle back poster headline',"LET'S CIRCLE",(0,-.029,.36),.15,'sageDark')
    text('Circle back poster punchline','BACK ON THAT.',(0,-.029,.15),.16,'clay')
    # A circular arrow around a walking figure: returning to exactly the same place.
    angles=[(.25+i/64*1.82)*pi for i in range(65)]
    points=[(.34*cos(a),-.25+.27*sin(a)) for a in angles]
    poster_ink('Circle back looping arrow',points,.011)
    x,z=points[-1]
    poster_ink('Circle back arrowhead',[(x-.075,z-.055),(x,z),(x+.045,z-.075)],.012)
    poster_head('Circle back walking figure head',.025,-.105,.043)
    poster_ink('Circle back walking torso',[(.015,-.17),(-.025,-.29)],.018,'clay')
    poster_ink('Circle back swinging arms',[(-.12,-.20),(-.06,-.23),(.005,-.19),(.09,-.24),(.14,-.20)],.012,'clay')
    poster_ink('Circle back forward stride',[(-.025,-.29),(-.105,-.35),(-.15,-.42),(-.19,-.42)],.013,'clay')
    poster_ink('Circle back rear stride',[(-.025,-.29),(.075,-.34),(.135,-.405),(.19,-.405)],.013,'clay')
local_group('Lets circle back on that framed poster',(-4.28,-.56,2.40),pi/2,circle_back_poster)
def bandwidth_poster():
    box('Bandwidth poster oak frame',(0,0,0),(1.60,.045,1.30),'oak2',.009)
    box('Bandwidth poster cream paper',(0,-.025,0),(1.51,.004,1.21),'paper',.002)
    text('Bandwidth poster headline','DO WE HAVE',(0,-.029,.37),.145,'sageDark')
    text('Bandwidth poster question','ENOUGH',(0,-.029,.19),.14,'sageDark')
    text('Bandwidth poster punchline','BANDWIDTH?',(0,-.029,-.015),.17,'clay')
    # Five coworkers crowd onto a short, narrow band; arms spill over both ends.
    poster_ink('Narrow bandwidth band',[(-.40,-.48),(.40,-.48)],.026)
    for i in range(5):
        x=-.39+i*.195;z=-.17+(i%2)*.025
        ma='clay' if i%2==0 else 'slate'
        poster_head('Bandwidth crowded figure head',x,z,.044,ma)
        poster_ink('Bandwidth crowded figure torso',[(x,z-.065),(x,z-.17)],.016,ma)
        poster_ink('Bandwidth crowded figure arms',[(x-.11,z-.07),(x-.07,z-.12),(x,z-.085),(x+.07,z-.12),(x+.11,z-.07)],.011,ma)
        poster_ink('Bandwidth crowded figure legs',[(x-.055,-.455),(x,z-.17),(x+.055,-.455)],.012,ma)
bandwidth=local_group('Do we have enough bandwidth framed poster',(-2.98,3.68,2.58),0,bandwidth_poster)
bandwidth.scale=(.82,.82,.82)
for o in ROOT.children_recursive:
    if o.type=='FONT':o.data.resolution_u=8

# Standard corporate workstation, expressed in a local coordinate system.
def workstation(index):
    rounded_slab('Oak workstation desktop',(0,0,.80),1.72,.82,.065,.045,'oak3')
    for x in [-.73,.73]:
        for y in [-.30,.30]:box('Black desk frame leg',(x,y,.43),(.045,.045,.73),'steelDark',.005)
        box('Desk side frame stretcher',(x,0,.68),(.04,.64,.04),'steelDark',.004)
    box('Under desk cable tray',(0,.24,.68),(1.28,.18,.07),'steelDark',.009)
    box('Under desk compact PC',(.55,.10,.59),(.22,.32,.15),'charcoal',.013)
    for i in range(6):box('Mini PC ventilation slit',(.47+i*.027,-.064,.60),(.008,.003,.065),'black',.001)
    rounded_slab('Desk mat',(0,-.11,.838),1.20,.47,.008,.025,['sageDark','linen','slate'][index])
    # Articulated monitor stand and continuous cable into the cable tray.
    box('Monitor stand foot',(0,.18,.85),(.33,.23,.025),'steelDark',.012)
    rod('Monitor stand support',(0,.23,.87),(0,.23,1.19),.027,'steelDark')
    box('Monitor rear housing',(0,.24,1.30),(.87,.045,.50),'charcoal',.018)
    box('Code editor screen',(0,.213,1.30),(.804,.006,.438),'screen',.007)
    box('Editor sidebar',(-.31,.208,1.29),(.14,.002,.40),'navy',.001)
    text('Screen editor title',['workspace.rs','recipe-editor.ts','api-tests.rs'][index],(-.21,.205,1.47),.024,'paper',align='LEFT')
    # Different real source snippets, rather than the same decorative token grid.
    snippets=[
        ['use crate::recipes::Recipe;', '', 'pub fn find_recipes(', '    recipes: &[Recipe],', '    query: &str,', ') -> Vec<&Recipe> {', '    recipes.iter()', '        .filter(|recipe| {', '            recipe.title.contains(query)', '        })', '        .collect()', '}'],
        ['type Ingredient = {', '  name: string;', '  grams: number;', '};', '', 'export function scaleRecipe(', '  items: Ingredient[], factor: number', ') {', '  return items.map(item => ({', '    ...item, grams: item.grams * factor', '  }));', '}'],
        ['#[tokio::test]', 'async fn saves_a_recipe() {', '    let app = test_app().await;', '    let recipe = sample_recipe();', '', '    let result = app.post("/recipes")', '        .json(&recipe)', '        .send().await;', '', '    assert_eq!(result.status(), 201);', '    assert!(app.recipe_exists().await);', '}'],
    ]
    for j,line in enumerate(snippets[index]):
        z=1.423-j*.027
        text('Editor line number',str(j+1),(-.228,.204,z),.013,'steel',align='RIGHT')
        if line:
            text('Editor code line',line,(-.205,.204,z),.017,['codeBlue','codeGold','codeGreen'][(j+index)%3],align='LEFT')
    curve('Monitor power cable',[(0,.265,1.1),(.13,.29,.83),(.20,.30,.69)],.007,'black')
    # Recessed keyboard tray with visible perimeter walls and inset keycaps.
    box('Keyboard lower case',(-.11,-.13,.853),(.56,.205,.026),'charcoal',.009)
    box('Keyboard recessed plate',(-.11,-.13,.870),(.524,.172,.009),'black',.003)
    for y in [-.229,-.031]:box('Keyboard case front rear rim',(-.11,y,.877),(.56,.014,.028),'steelDark',.003)
    for x in [-.383,.163]:box('Keyboard case side rim',(x,-.13,.877),(.014,.184,.028),'steelDark',.003)
    for row in range(4):
        for col in range(13):
            if row==0 and 4<=col<=8:continue
            keymat=(['ivory','clay','mist'][index] if col>10 else ['steelDark','ivory','charcoal'][index])
            box('Inset keyboard key',(-.35+col*.039,-.194+row*.041,.881),(.032,.032,.015),keymat,.004)
    box('Keyboard spacebar',(-.116,-.194,.881),(.188,.032,.015),['steelDark','ivory','mist'][index],.004)
    # Contoured mouse, distinct buttons and scroll wheel.
    uv('Mouse rounded shell',(.40,-.11,.861),(.061,.10,.032),'charcoal',24,12)
    for x in [.379,.421]:
        o=pill('Mouse primary button',(x,-.15,.883),(.036,.074,.012),'steelDark')
    cyl('Mouse scroll wheel',(.40,-.17,.889),.010,.012,'black',20).rotation_euler.y=pi/2
    if index==0:
        # A tidy desk: phone, insulated travel coffee and a single plant.
        box('Minimalist phone case',(-.64,-.17,.846),(.14,.25,.017),'charcoal',.015)
        box('Minimalist phone glass',(-.64,-.17,.855),(.124,.226,.002),'screen',.010)
        box('Phone speaker',(-.64,-.073,.857),(.036,.004,.002),'black',.001)
        text('Phone clock','09:41',(-.64,-.15,.858),.030,'paper',rot=(0,0,0))
        lathe('Insulated travel tumbler',(-.64,.20,.837),[(0,0),(.048,0),(.055,.02),(.064,.20),(0,.20)],'sageDark',40)
        cyl('Travel tumbler lid',(-.64,.20,1.046),.065,.018,'charcoal',32)
        box('Travel lid sip opening',(-.64,.168,1.057),(.033,.014,.003),'black',.005)
        pot('Minimalist desk plant',.65,.20,.836,.065)
    elif index==1:
        # A planner's desk: open ruled notebook, pen cup and loose sticky notes.
        box('Planner notebook cloth cover',(.65,-.10,.845),(.29,.32,.016),'bookRust',.004)
        for x in [.58,.72]:
            box('Planner open page',(x,-.10,.856),(.135,.30,.009),'paper',.003)
            for j in range(8):
                box('Notebook ruled line',(x,-.215+j*.031,.862),(.108,.0015,.001),'mist',0)
            for j in [1,3,4,6]:
                box('Notebook handwritten note',(x-.012,-.215+j*.031,.863),(.059+(j%2)*.02,.003,.001),'navy',0)
        rod('Planner notebook binding',(.65,-.25,.860),(.65,.05,.860),.003,'walnut2',10)
        rod('Planner pen',(.78,-.21,.869),(.78,-.005,.869),.004,'navy',10)
        lathe('Stationery pencil cup',(-.64,.21,.836),[(0,0),(.055,0),(.057,.12),(.049,.12),(.047,.012),(0,.012)],'clay',32)
        for j in range(4):
            x=-.665+(j%2)*.04;y=.19+(j//2)*.035;z=1.025+(j%3)*.013
            rod('Planner pencil',(x,y,.849),(x+.012,y+.006,z),.004,['honey','navy','sageDark','bookRust'][j],10)
            rod('Pencil graphite tip',(x+.012,y+.006,z),(x+.012,y+.006,z+.008),.002,'charcoal',8)
        mug('Planner morning coffee',-.64,-.10,.837,.055,.09)
        for j in range(2):
            box('Loose desk sticky note',(-.53+j*.15,-.315,.840),(.125,.10,.004),['honey','pink'][j],.002)
            for k in range(2):box('Sticky note writing',(-.53+j*.15,-.33+k*.021,.843),(.07,.002,.001),'navy',0)
    else:
        # A music lover's desk: laid-down headphones, coffee and a cookie break.
        hx,hy=-.63,-.04
        arc=[(hx+.125*cos(a),hy+.135*sin(a),.875) for a in [i*pi/16 for i in range(17)]]
        curve('Desk headphones padded headband',arc,.014,'charcoal')
        curve('Headphones metal outer band',[(x,y,z+.011) for x,y,z in arc],.005,'steelDark')
        for x in [hx-.117,hx+.117]:
            rod('Headphone attached yoke',(x,hy,.873),(x,hy-.06,.873),.008,'steelDark',12)
            pill('Headphone earcup shell',(x,hy-.065,.866),(.075,.105,.047),'slate')
            pill('Headphone ear cushion',(x,hy-.065,.893),(.065,.093,.018),'charcoal')
            pill('Headphone cushion opening',(x,hy-.065,.903),(.035,.061,.003),'black')
        curve('Headphone desk cable',[(hx-.12,hy-.12,.85),(-.78,-.25,.846),(-.75,-.34,.846),(-.46,-.34,.846),(-.43,-.25,.85)],.0025,'black')
        plate('Cookie snack saucer',.66,-.18,.837,.115)
        for j in range(2):
            x=.63+j*.055;y=-.18+j*.034;z=.878+j*.018
            cyl('Chocolate chip cookie',(x,y,z),.060,.018,'cookie',32)
            for k in range(5):
                a=k*2*pi/5+.4
                uv('Cookie chocolate chip',(x+.033*cos(a),y+.033*sin(a),z+.010),(.006,.005,.003),'chocolate',10,6)
        mug('Music desk afternoon coffee',.65,.21,.837,.065,.105)
        box('Music desk closed pocket journal',(-.66,.28,.852),(.25,.17,.03),'plum',.007)
        box('Pocket journal elastic band',(-.59,.28,.869),(.012,.17,.003),'charcoal',.001)

collection('05 Three corporate workstations',start=38,end=78)
DESK_Y=[-2.44,-.54,1.36]
for i,y in enumerate(DESK_Y):local_group('Workstation '+str(i+1),(-3.79,y,0),pi/2,lambda i=i:workstation(i))

# Detailed five-star task chair with actual open mesh and attached lumbar bar.
def task_chair():
    cyl('Chair gas lift',(0,0,.32),.039,.39,'steelDark',24)
    cyl('Chair lift sleeve',(0,0,.22),.055,.22,'charcoal',24)
    for i in range(5):
        a=i*2*pi/5+.25;x=.33*cos(a);y=.33*sin(a)
        rod('Chair five star foot',(0,0,.20),(x,y,.12),.026,'charcoal',16)
        cyl('Chair caster stem',(x,y,.105),.017,.065,'steelDark',16)
        for side in [-1,1]:
            o=cyl('Twin chair caster',(x+side*.027,y,.095),.043,.035,'charcoal',20);o.rotation_euler.y=pi/2
    box('Chair underseat tilt mechanism',(0,0,.43),(.28,.28,.09),'charcoal',.025)
    pill('Task chair upholstered seat',(0,-.015,.52),(.56,.53,.13),'charcoal')
    # Continuous perimeter curves keep the woven fabric visibly inside a frame.
    outline=[(-.23,.20,.61),(-.25,.24,.82),(-.23,.28,1.13),(-.17,.29,1.19),(.17,.29,1.19),(.23,.28,1.13),(.25,.24,.82),(.23,.20,.61)]
    curve('Task chair continuous back frame',outline,.024,'steelDark',True)
    for i in range(31):
        z=.63+i*.0175;t=(z-.63)/.56;y=.20+.09*t
        curve('Task chair woven horizontal mesh',[(-.21,y,z),(0,y+.016,z),(.21,y,z)],.0025,'taskMesh')
    for i in range(20):
        x=-.20+i*.021
        curve('Task chair woven vertical mesh',[(x,.20,.63),(x,.25,.91),(x,.29,1.16)],.002,'taskMesh')
    curve('Task chair attached rear spine',[(0,.18,.46),(0,.29,.60),(0,.30,.76)],.029,'charcoal')
    curve('Task chair lumbar support attachment',[(-.23,.235,.75),(0,.31,.76),(.23,.235,.75)],.018,'steelDark')
    for x in [-.30,.30]:
        rod('Chair attached arm support',(x*.66,0,.46),(x,.04,.71),.022,'charcoal',16)
        pill('Chair arm pad',(x,-.03,.74),(.095,.31,.045),'charcoal')
    rod('Chair height lever',(.12,-.01,.43),(.29,-.12,.45),.012,'charcoal',12)
collection('06 Mesh task chairs',start=48,end=88)
for i,y in enumerate(DESK_Y):local_group('Task chair '+str(i+1),(-2.68,y,0),-pi/2,task_chair)

collection('07 Pantry lower cabinets and working sink',start=34,end=75)
# Separate doors, toe kick and hardware. Countertop is built around the real sink opening.
box('Pantry recessed toe kick',(.45,3.27,.15),(4.65,.58,.18),'walnut2',.009)
box('Pantry cabinet carcass',(.45,3.28,.43),(4.65,.70,.52),'oak2',.016)
for i in range(7):
    x=-1.53+i*.66
    box('Pantry cabinet individual door',(x,2.913,.53),(.633,.030,.67),'oak3',.012)
    rod('Pantry cupboard metal pull',(x+.20,2.886,.59),(x+.20,2.886,.76),.009,'steelDark',12)
# Slabs abut at edges and leave a 0.70 x 0.46m basin opening.
box('Stone counter left of basin',(-.575,3.22,.918),(2.65,.82,.064),'stone',.006)
box('Stone counter right of basin',(2.125,3.22,.918),(1.35,.82,.064),'stone',.006)
box('Stone counter front sink rim',(1.10,2.915,.918),(.70,.21,.064),'stone',.004)
box('Stone counter rear sink rim',(1.10,3.555,.918),(.70,.15,.064),'stone',.004)
box('Sink basin floor',(1.10,3.25,.735),(.64,.40,.018),'steelSink',.014)
for x in [.762,1.438]:box('Sink basin side',(x,3.25,.823),(.024,.46,.18),'steelSink',.008)
for y in [3.032,3.468]:box('Sink basin front rear',(1.10,y,.823),(.68,.024,.18),'steelSink',.008)
curve('Sink metal upper rim',rounded_points(1.1,3.25,.954,.72,.48,.035),.009,'steel',True,True)
cyl('Sink drain',(1.1,3.25,.747),.032,.004,'steelDark',24)
curve('Swan neck pantry faucet',[(1.1,3.53,.95),(1.1,3.53,1.26),(1.1,3.46,1.35),(1.1,3.22,1.35),(1.1,3.20,1.27)],.018,'steel')
rod('Faucet mixer lever',(1.22,3.52,.95),(1.22,3.50,1.07),.012,'steelDark',16)
bottle('Hand soap',1.54,3.49,.95,'sageLight',.18,.043,'SOAP')
# A tiled splashback at the rear, separate from counter and upper cabinets.
for i in range(15):
    for j in range(3):box('Pantry splashback tile',(-1.74+i*.31,3.695,1.04+j*.18),(.302,.025,.173),'ivory',.003)

collection('08 Upper cabinets and microwave',start=41,end=82)
for x,w in [(-1.15,1.45),(1.61,2.20)]:
    box('Upper pantry cabinet body',(x,3.43,2.49),(w,.51,.95),'oak2',.012)
    for xx in [x-w/4,x+w/4]:
        box('Upper pantry cabinet door',(xx,3.159,2.49),(w/2-.025,.028,.91),'oak3',.010)
        rod('Upper cabinet slim pull',(xx+w/4-.10,3.13,2.30),(xx+w/4-.10,3.13,2.50),.009,'steelDark',12)
box('Microwave shelf',(.04,3.40,2.04),(1.05,.59,.055),'oak3',.009)
box('Microwave upper cubby',(.04,3.44,2.95),(1.05,.51,.065),'oak3',.009)
for x in [-.49,.57]:box('Microwave cubby side',(x,3.44,2.49),(.06,.51,.94),'oak2',.009)
box('Microwave body',(.04,3.39,2.34),(.92,.48,.54),'steel',.025)
box('Microwave dark door',(-.055,3.138,2.34),(.66,.025,.43),'charcoal',.015)
box('Microwave smoked window',(-.10,3.12,2.34),(.49,.008,.30),'screen',.008)
rod('Microwave door handle',(.23,3.094,2.20),(.23,3.094,2.48),.012,'steel')
box('Microwave control display',(.40,3.134,2.46),(.12,.008,.07),'screen',.005)
text('Microwave time','12:00',(.40,3.125,2.45),.025,'codeGreen')
for z in [2.34,2.23]:cyl('Microwave control knob',(.40,3.12,z),.029,.018,'steelDark',24).rotation_euler.x=pi/2

collection('09 Pantry fridge and water cooler',start=46,end=86)
box('Office refrigerator body',(3.35,3.31,1.13),(.82,.83,2.08),'steel',.035)
for z,h in [(1.67,.89),(.635,1.10)]:
    box('Office refrigerator door',(3.35,2.874,z),(.79,.045,h),'ivory',.022)
    rod('Refrigerator vertical pull',(3.02,2.822,z-.24),(3.02,2.822,z+.24),.015,'steelDark')
for i in range(3):
    box('Fridge pinned lunch note',(3.44+(i%2)*.14,2.847,1.84-i*.19),(.17,.008,.15),['paper','honey','sageLight'][i],.002)
    uv('Fridge magnet',(3.44+(i%2)*.14,2.838,1.90-i*.19),(.018,.007,.018),'clay',12,8)
CX,CY=-2.30,3.18
box('Water cooler body',(CX,CY,.63),(.50,.52,1.12),'ivory',.035)
box('Water cooler front recess',(CX,CY-.273,.89),(.37,.015,.34),'charcoal',.014)
box('Water cooler drip tray',(CX,CY-.31,.68),(.39,.21,.035),'steelDark',.009)
for x,ma in [(CX-.09,'blueCan'),(CX+.09,'redCan')]:
    rod('Water cooler tap',(x,CY-.275,.96),(x,CY-.34,.96),.018,'steel',16)
    box('Cooler tap lever',(x,CY-.31,1.0),(.052,.06,.035),ma,.005)
lathe('Inverted water cooler bottle',(CX,CY,1.16),[(.07,0),(.07,.09),(.18,.18),(.21,.24),(.21,.61),(.17,.67),(0,.67)],'waterJug',48)
for z in [1.45,1.60,1.74]:torus('Cooler bottle strengthening ring',(CX,CY,z),.21,.012,'jugBlue',steps=48)
cyl('Cooler visible water',(CX,CY,1.47),.194,.35,'waterJug',48)

collection('10 Coffee machine mugs and pantry supplies',start=74,end=111)
MX,MY=-1.25,3.24
box('Coffee machine body',(MX,MY,1.26),(.51,.44,.62),'charcoal',.025)
box('Coffee machine steel fascia',(MX,MY-.232,1.29),(.46,.026,.51),'steelDark',.015)
box('Coffee machine control screen',(MX,MY-.25,1.46),(.19,.006,.10),'screen',.006)
text('Coffee machine display','READY',(MX,MY-.256,1.45),.029,'codeGreen')
for x in [MX-.17,MX+.17]:cyl('Coffee machine selector',(x,MY-.25,1.44),.026,.018,'steel',24).rotation_euler.x=pi/2
cyl('Coffee group head',(MX,MY-.255,1.30),.065,.07,'steel',32)
rod('Attached portafilter grip',(MX,MY-.27,1.275),(MX+.18,MY-.36,1.275),.019,'charcoal',20)
for x in [MX-.023,MX+.023]:rod('Coffee twin spout',(x,MY-.26,1.27),(x,MY-.26,1.21),.006,'steel',12)
box('Coffee machine attached drip tray',(MX,MY-.16,.98),(.48,.48,.045),'steelDark',.008)
for i in range(11):rod('Coffee drip grate',(MX-.20+i*.04,MY-.37,1.008),(MX-.20+i*.04,MY+.03,1.008),.003,'steel',8)
mug('Coffee cup under spout',MX,MY-.24,1.012,.048,.075)
for i in range(3):mug('Pantry clean mug',-.55+i*.23,3.23,.95,.063,.10)
jar('Office coffee beans',-.30,3.51,.95,'beefCrust',.078,.22)
jar('Office sugar',.04,3.51,.95,'paper',.070,.18)
pack('Office tea box',.38,3.40,.95,.21,.18,.26,'sageDark','TEA')
box('Pantry dish towel',(2.22,3.15,.96),(.35,.40,.016),'linen',.006)
for i in range(5):box('Dish towel woven stripe',(2.10+i*.06,3.15,.970),(.012,.38,.002),'sage',.001)

collection('11 Shared lunch table',start=50,end=89)
TX,TY=.80,-.90
TABLEZ=.87
# The table is separated from desk-chair backs and the kitchenette working aisle.
table('Rounded oak communal lunch table',TX,TY,TABLEZ,3.85,1.52,'oak3')
collection('12 Six upholstered lunch chairs',start=59,end=96)
def lunch_chair():
    for x in [-.21,.21]:
        for y in [-.20,.20]:rod('Lunch chair tapered oak leg',(x*1.15,y*1.12,.065),(x,y,.48),.027,'walnut2',16)
    pill('Lunch chair padded seat',(0,0,.51),(.57,.56,.12),'clay')
    o=pill('Lunch chair upholstered back',(0,.245,.79),(.57,.115,.52),'clay')
    o.rotation_euler.x=-.07
    for x in [-.24,.24]:rod('Lunch chair back support',(x,.18,.45),(x,.27,.77),.022,'walnut2',16)
for i,x in enumerate([TX-1.25,TX,TX+1.25]):
    local_group('Lunch chair north '+str(i+1),(x,TY+1.12,0),0,lunch_chair)
    local_group('Lunch chair south '+str(i+1),(x,TY-1.12,0),pi,lunch_chair)

# Actual layered triangular sandwich wedges with bread, filling and crust edges.
def triangle_layer(n,z,h,ma):
    pts=[(-.13,-.115),(.13,-.115),(-.13,.145)]
    v=[(x,y,zz) for zz in [z,z+h] for x,y in pts]
    o=mesh(n,v,[(2,1,0),(3,4,5),(0,1,4,3),(1,2,5,4),(2,0,3,5)],ma);bevel(o,.006,2)
def sandwich():
    for z in [0,.105]:
        triangle_layer('Sandwich bread crust',z,.034,'crustLight')
        triangle_layer('Sandwich soft crumb',z+.006,.024,'dough')
    triangle_layer('Sandwich cheese slice',.041,.011,'cheese')
    triangle_layer('Sandwich ham filling',.066,.021,'ham')
    for i in range(6):
        x=-.10+(i%3)*.055;y=-.085+(i//3)*.075
        uv('Sandwich folded lettuce',(x,y,.060),(.061,.052,.015),'leaf',12,8)
    for i in range(3):
        o=cyl('Sandwich tomato slice',(-.08+i*.055,-.06,.095),.045,.012,'tomato',20)
collection('13 Sandwich platter and shared fruit',start=89,end=127)
FOODZ=TABLEZ+.052
rounded_slab('Large sandwich platter',(TX-.62,TY,FOODZ+.016),1.35,.70,.033,.09,'ceramic')
curve('Platter raised lip',rounded_points(TX-.62,TY,FOODZ+.035,1.29,.64,.08),.009,'ceramic',True,True)
for i in range(4):
    for j in range(2):local_group('Layered sandwich wedge',(TX-1.08+i*.30,TY-.17+j*.31,FOODZ+.034),.06 if j else -.05,sandwich)
bowl('Communal fruit bowl',TX+.87,TY+.16,FOODZ,.32,'ceramic')
for i,(dx,dy,kind) in enumerate([(-.13,-.09,'apple'),(.07,-.12,'orange'),(.15,.06,'pear'),(-.08,.11,'apple')]):
    fruit('Lunch table fresh '+kind,TX+.87+dx,TY+.16+dy,FOODZ+.06,kind,.09)
for j in range(2):
    curve('Fruit bowl banana',[(TX+.68,TY+.25+j*.05,FOODZ+.19),(TX+.73,TY+.08+j*.05,FOODZ+.25),(TX+.94,TY+.02+j*.05,FOODZ+.26),(TX+1.07,TY+.11+j*.05,FOODZ+.20)],.026,'lemon')

collection('14 Lunch drinks napkins and table plant',start=94,end=132)
# Transparent pitcher with a continuous handle that meets the vessel at both ends.
JX,JY=TX+.12,TY+.16
lathe('Lunch water pitcher',(JX,JY,FOODZ),[(0,0),(.09,0),(.11,.03),(.11,.30),(.10,.35),(.09,.35),(.10,.29),(.10,.03),(0,.015)],'glass',48)
cyl('Pitcher water',(JX,JY,FOODZ+.12),.095,.20,'water',40)
curve('Pitcher attached handle',[(JX+.10,JY,FOODZ+.28),(JX+.20,JY,FOODZ+.28),(JX+.21,JY,FOODZ+.10),(JX+.10,JY,FOODZ+.09)],.012,'glass')
for x,y in [(TX+.07,TY-.30),(TX+.43,TY-.26)]:glass('Lunch drinking glass',x,y,FOODZ,.048,.13,'water')
box('Lunch napkin stack',(TX+.88,TY-.37,FOODZ+.016),(.30,.24,.028),'paper',.006)
for i in range(2):
    x=TX+.80+i*.13
    rod('Lunch cutlery handle',(x,TY-.44,FOODZ+.036),(x,TY-.28,FOODZ+.036),.006,'steel',12)
    for k in range(3):rod('Fork tine',(x+(k-1)*.011,TY-.28,FOODZ+.036),(x+(k-1)*.011,TY-.24,FOODZ+.036),.002,'steel',8)
pot('Lunch table small herb',TX+1.44,TY+.08,FOODZ,.10)

collection('15 Office finishing details',start=80,end=119)
# A bin sits at the end of the kitchenette, not in the path of its cupboard doors.
BX,BY=-2.95,3.18
rounded_slab('Office recycling bin',(BX,BY,.32),.42,.43,.51,.065,'sageDark')
rounded_slab('Recycling bin lid',(BX,BY,.59),.45,.46,.055,.06,'charcoal')
box('Recycling bin opening',(BX,BY,.62),(.28,.16,.012),'black',.022)
text('Recycling bin label','RECYCLE',(BX,BY-.223,.36),.048,'paper')
# Two wall-mounted task lights, with attached brackets rather than floating pendants.
for x in [-.95,1.5]:
    box('Pantry undercabinet light',(x,3.24,2.007),(.60,.055,.02),'led',.004)
for key,m in M.items():
    p=m.node_tree.nodes.get('Principled BSDF')
    if p and key not in ['glass','water','waterJug','tea','cola','led']:
        p.inputs['Roughness'].default_value=max(p.inputs['Roughness'].default_value,.48 if p.inputs['Metallic'].default_value else .78)
scene=bpy.context.scene
scene['design_reference']='Selected corporate office concept 05 — Shared Workspace & Pantry'
scene['workstations']=3;scene['task_chairs']=3;scene['lunch_seats']=6
scene['purpose']='Design review for A project to support my job search'
finish('corporate-workspace-pantry','Noms — Shared Corporate Workspace and Pantry',12.8,(0,0,1.12))
