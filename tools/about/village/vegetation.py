"""Deterministic stylized meadow and folded foliage, Python + Shapely 2.2.

The original landscape snapshot stays immutable. Replacements reference its
indices; existing room assets, paths, tree positions and camera poses do not move.
All foliage fits inside the original component bounds. New groundcover is clipped
away from paving, rooms, water, foundations and low landscape props.
"""
import gzip, hashlib, json, math, random
from pathlib import Path
import numpy as np
from shapely import contains_xy
from shapely.geometry import Point, Polygon, box
from shapely.ops import unary_union
from shapely.prepared import prep

ROOT=Path(__file__).resolve().parents[3]
ASSET=ROOT/'assets/models/terraced-village'; DAY=ASSET/'daylight'
source=ASSET/'source/world-environment.json'
items=json.loads(source.read_text()); layout=json.loads((ASSET/'layout.json').read_text())
poses=json.loads((ASSET/'source/camera-poses.json').read_text())
paths=json.loads(gzip.decompress((DAY/'source/stone-paths.json.gz').read_bytes()))
rng=random.Random(912743)

def components(item):
    """Connect indexed faces through position seams, not only shared indices."""
    verts=np.asarray(item['vertices']);parent=list(range(len(verts)));seen={}
    def find(i):
        while parent[i]!=i:parent[i]=parent[parent[i]];i=parent[i]
        return i
    def union(a,b):parent[find(a)]=find(b)
    for i,v in enumerate(verts):
        key=tuple(np.round(v,5))
        if key in seen:union(i,seen[key])
        else:seen[key]=i
    for a,b,c in np.asarray(item['indices']).reshape(-1,3):union(a,b);union(a,c)
    groups={}
    for i in range(len(verts)):groups.setdefault(find(i),[]).append(i)
    return [verts[g] for g in groups.values()]

def footprint(item,top_only=False):
    v=np.asarray(item['vertices']);n=np.asarray(item['normals']);polys=[]
    for face in np.asarray(item['indices']).reshape(-1,3):
        if top_only and n[face,1].mean()<.1:continue
        p=Polygon(v[face][:,[0,2]])
        if p.area>1e-8:polys.append(p)
    return unary_union(polys)

network=footprint(paths['meshes'][0],True)
river=footprint(next(i for i in items if i['name']=='River banks'))
room_boxes=[box(p['bounds']['min'][0],p['bounds']['min'][2],p['bounds']['max'][0],p['bounds']['max'][2]).buffer(.45) for p in poses]
obstacles=[river.buffer(.35),*room_boxes]
prop_names={'Cottage foundation','Front porch deck','Weathered landscape rock','Broad entrance step','Doorway landing','Low stone planting border','Recessed herb bed soil','Inset corner planting pocket'}
for item in items:
    if item['name'] in prop_names:
        for v in components(item):
            lo=v.min(0);hi=v.max(0)
            obstacles.append(box(lo[0],lo[2],hi[0],hi[2]).buffer(.22))
excluded=unary_union([network.buffer(.32),*obstacles]);safe=prep(excluded)
near=prep(network.buffer(6));middle=prep(network.buffer(15))

def ground(x,z):
    value=.18-z*.014+math.sin(x*.045)*math.cos(z*.035)*.3
    for stop in layout['stops']:
        d=math.hypot(x-stop['position'][0],z-stop['position'][2]);t=min(1,max(0,(d-stop['width']*.76)/7));blend=1-t*t*(3-2*t)
        value+=(stop['position'][1]-.15-value)*blend
    return value

def linear(color):
    a=np.array([int(color[i:i+2],16)/255 for i in (1,3,5)])
    return np.where(a<=.04045,a/12.92,((a+.055)/1.055)**2.4)

meshes={}
def batch(name,color):
    key=(name,tuple(color))
    if key not in meshes:meshes[key]=dict(name=f'{name} {sum(k[0]==name for k in meshes)+1:02d}',vertices=[],normals=[],indices=[],colors=[],color=list(color),roughness=1,specular_intensity=.04)
    return meshes[key]
def tri(m,a,b,c,tints=None):
    v=np.array([a,b,c]);normal=np.cross(v[1]-v[0],v[2]-v[0]);length=np.linalg.norm(normal)
    if length<1e-10:return
    normal/=length;k=len(m['vertices']);m['vertices'].extend(np.round(v,6).tolist());m['normals'].extend([np.round(normal,6).tolist()]*3);m['indices'].extend([k,k+1,k+2])
    if tints is None:tints=[m['color']]*3
    m['colors'].extend([np.round(t,6).tolist() for t in tints])
def quad(m,a,b,c,d,tints=None):
    tri(m,a,b,c,None if tints is None else [tints[0],tints[1],tints[2]])
    tri(m,a,c,d,None if tints is None else [tints[0],tints[2],tints[3]])

remove=[];foliage_proof=[]
for index,item in enumerate(items):
    name=item['name'];v=np.array(item['vertices']);size=np.ptp(v,axis=0);color=np.array(item['color'])
    evergreen=name=='Folded evergreen canopy'
    old_cone=name=='Landscape detail' and index>0 and max(size)<15 and color[1]>color[0]*1.25 and color[1]>color[2]*1.4
    if name in {'Meadow grass blade','Small meadow flower'}:remove.append(index);continue
    if name not in {'Rounded leafy canopy','Leafy low shrub','Low culinary herb planting','Window herb leaves'} and not evergreen and not old_cone:continue
    remove.append(index)
    for original in components(item):
        lo=original.min(0);hi=original.max(0);center=(lo+hi)/2;size=hi-lo
        if min(size)<.01:continue
        cone=evergreen or old_cone;shrub=size[1]<2
        sectors=16 if cone or not shrub else 12;rings=6 if cone else (8 if shrub else 12)
        phase=rng.random()*math.tau;points=[]
        for j in range(rings+1):
            t=j/rings
            for k in range(sectors):
                angle=k*math.tau/sectors
                if cone:
                    radius=(1-t)*(.82+.18*(k%2))*(1+.08*math.sin(t*math.pi*3))
                else:
                    radius=math.sin(math.pi*t)*(.87+.09*math.cos(angle*6+phase))*(.90+.10*math.cos(t*math.pi*6))
                points.append([radius*math.cos(angle),t,radius*math.sin(angle)])
        points=np.array(points);pmin=points.min(0);span=np.ptp(points,axis=0)
        points=lo+(points-pmin)/span*size
        m=batch('Ridged evergreen foliage' if cone else 'Folded shrub foliage' if shrub else 'Layered broadleaf foliage',color)
        for j in range(rings):
            for k in range(sectors):
                a=j*sectors+k;b=j*sectors+(k+1)%sectors;c=(j+1)*sectors+(k+1)%sectors;d=(j+1)*sectors+k
                # Outward winding: azimuth followed by height in Y-up coordinates.
                t0=color*(.86+.20*j/rings);t1=color*(.86+.20*(j+1)/rings)
                quad(m,points[a],points[d],points[c],points[b],[t0,t1,t1,t0])
        if cone:
            cap=points[:sectors].mean(0)
            for k in range(sectors):tri(m,cap,points[k],points[(k+1)%sectors],[color*.86]*3)
        assert np.all(points>=lo-1e-7) and np.all(points<=hi+1e-7)
        foliage_proof.append(dict(source_index=index,min=lo.tolist(),max=hi.tolist()))

stem=batch('Meadow stems and leaves',linear('#568635'));white=batch('Ivory daisy petals',linear('#fff6d7'));yellow=batch('Golden flower centers',linear('#e9bf40'));purple=batch('Lavender flower spikes',linear('#a782cc'));gold=batch('Buttercup petals',linear('#e9ce66'))

def spike(m,base,tip,width,color=None,cap=True):
    base=np.array(base);tip=np.array(tip);angle=rng.random()*math.tau
    vertices=[base+np.array([math.cos(angle+i*math.tau/3)*width,0,math.sin(angle+i*math.tau/3)*width]) for i in range(3)]
    tint=np.array(m['color'] if color is None else color)
    for i in range(3):tri(m,vertices[i],tip,vertices[(i+1)%3],[tint*.76,tint*1.12,tint*.76])
    if cap:tri(m,*vertices,[tint*.76]*3)

def leaf(base,angle,length,width):
    base=np.array(base);direction=np.array([math.cos(angle),.2,math.sin(angle)]);side=np.array([-math.sin(angle),0,math.cos(angle)])
    mid=base+direction*length*.55;tip=base+direction*length
    ridge=mid+np.array([0,.035,0]);left=mid+side*width;right=mid-side*width
    for a,b,c in [(base,ridge,left),(left,ridge,tip),(base,right,ridge),(right,tip,ridge)]:tri(stem,a,c,b)

def bloom(m,center,radius,petals):
    center=np.array(center)
    for k in range(petals):
        angle=k*math.tau/petals;d=np.array([math.cos(angle),0,math.sin(angle)]);s=np.array([-math.sin(angle),0,math.cos(angle)])
        base=center+d*radius*.15;tip=center+d*radius;mid=center+d*radius*.65;ridge=mid+np.array([0,.025,0])
        l=mid+s*radius*.23;r=mid-s*radius*.23
        for a,b,c in [(base,ridge,l),(l,ridge,tip),(base,r,ridge),(r,tip,ridge)]:tri(m,a,c,b)
    spike(yellow,center+[0,-.006,0],center+[0,.045,0],radius*.22)

patches=[];flower_count=0
for attempt in range(14000):
    if len(patches)>=200:break
    x=rng.uniform(-86,86);z=rng.uniform(-138,65);p=Point(x,z)
    if not near.contains(p) or excluded.distance(p)<1.15 or safe.contains(p):continue
    if any((x-a)**2+(z-b)**2<9 for a,b,_ in patches):continue
    kind=len(patches)%5;patches.append((x,z,kind))
    for f in range(rng.randint(9,16)):
        angle=rng.random()*math.tau;r=.85*math.sqrt(rng.random());fx=x+math.cos(angle)*r;fz=z+math.sin(angle)*r;fy=ground(fx,fz)-.03
        height=rng.uniform(.25,.48) if kind<3 else rng.uniform(.45,.8)
        base=np.array([fx,fy,fz]);top=base+[rng.uniform(-.08,.08),height,rng.uniform(-.08,.08)]
        spike(stem,base,top,.017)
        for n in range(3):leaf(base+[0,.08,0],angle+n*2.1,rng.uniform(.22,.38),.07)
        if kind<3:bloom(white if kind<2 else gold,top,rng.uniform(.105,.16),6)
        else:
            for n in range(4):bloom(purple,top-[0,n*.08,0],.075*(1+n*.17),4)
        flower_count+=1
flower_region=prep(unary_union([Point(x,z).buffer(.95) for x,z,_ in patches]))
palette=[linear(c) for c in ['#78a83d','#87b848','#6e9e39','#9bc153']]
grass=[batch('Folded meadow grass',c) for c in palette];clumps=[];blades=0
# Fill the same meadow bounds with roughly twice as many smaller tufts.
# Three blades per tuft spread coverage evenly. Omit only the buried base caps;
# visible blade faces retain their folded shape and baked lighting.
spacing=.52
for row in range(math.ceil(-137.36/spacing),math.floor(68/spacing)+1):
    z=row*spacing
    for col in range(math.ceil(-89.08/spacing),math.floor(89.08/spacing)+1):
        x=col*spacing+rng.uniform(-.18,.18);zz=z+rng.uniform(-.18,.18);p=Point(x,zz)
        probability=.85 if near.contains(p) else .38 if middle.contains(p) else .16
        if rng.random()>probability or safe.contains(p) or flower_region.contains(p):continue
        y=ground(x,zz)-.06;m=grass[rng.randrange(len(grass))]
        for k in range(3):
            angle=rng.random()*math.tau;height=rng.uniform(.26,.62);base=[x+rng.uniform(-.08,.08),y,zz+rng.uniform(-.08,.08)]
            tip=[base[0]+math.cos(angle)*height*.23,y+height,base[2]+math.sin(angle)*height*.23]
            spike(m,base,tip,rng.uniform(.05,.085),cap=False);blades+=1
        clumps.append((x,zz))

# Conservative plant envelopes fit inside the existing padded exclusions.
# Flower centers are 1.15 beyond exclusions already padded by at least .22;
# their 1.28 radius therefore retains at least .09 clearance from actual props.
assert all(network.distance(Point(x,z))>.32 for x,z in clumps)
assert all(not safe.contains(Point(x,z)) for x,z in clumps)
assert all(excluded.distance(Point(x,z))>=1.15 for x,z,_ in patches)
checked_vertices=0
for mesh in meshes.values():
    v=np.array(mesh['vertices']);n=np.array(mesh['normals']);c=np.array(mesh['colors']);indices=np.array(mesh['indices'])
    assert len(v)==len(n)==len(c)
    assert np.isfinite(v).all() and np.isfinite(c).all() and np.isfinite(n).all()
    assert np.abs(np.linalg.norm(n,axis=1)-1).max()<2e-6
    assert indices.min()>=0 and indices.max()<len(v)
    if mesh['name'].startswith(('Ivory daisy','Lavender flower','Buttercup petals')):
        assert (n[:,1]>0).all(), 'Petal tops must face the sky for the daylight bake'
    if 'foliage' not in mesh['name']:
        assert not contains_xy(network,v[:,0],v[:,2]).any(),mesh['name']
        checked_vertices+=len(v)
result=dict(baseline_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),remove_indices=remove,meshes=list(meshes.values()))
data=json.dumps(result,separators=(',',':')).encode();out=DAY/'source/vegetation.json.gz';out.write_bytes(gzip.compress(data,compresslevel=9,mtime=0))
report=dict(groundcover_vertices_checked_against_paving=checked_vertices,groundcover_path_intrusions=0,finite_vertices_colors_and_unit_normals=True,seed=912743,baseline_sha256=result['baseline_sha256'],sha256=hashlib.sha256(out.read_bytes()).hexdigest(),bytes=out.stat().st_size,foliage_components=len(foliage_proof),foliage_within_original_bounds=True,evergreen_undersides_closed=True,petal_tops_face_sky=True,grass_spacing=spacing,grass_blades_per_clump=3,buried_grass_caps_omitted=True,grass_clumps=len(clumps),grass_blades=blades,flower_patches=len(patches),individual_flowers=flower_count,triangles=sum(len(m['indices'])//3 for m in meshes.values()),meshes=len(meshes),minimum_grass_center_path_clearance=min(network.distance(Point(x,z)) for x,z in clumps),grass_envelope_radius=.28,flower_envelope_radius=1.28,groundcover_max_height=.85,paths_rooms_river_props_excluded=True,stone_path_sha256=hashlib.sha256((DAY/'source/stone-paths.json.gz').read_bytes()).hexdigest(),original_foliage_bounds=foliage_proof)
(DAY/'source/vegetation-verification.json').write_text(json.dumps(report,indent=2)+'\n')
# Rebuilding an existing layer must not downgrade a newer world revision.
if layout.get('version',0)<=10:
    layout['version']=10;layout['stage']='Denser meadow with connected stone paths and shared bright daylight'
layout['vegetation']=dict(geometry=str(out.relative_to(ROOT)),verification=str((DAY/'source/vegetation-verification.json').relative_to(ROOT)),style='Matte folded grass, daisies, buttercups, lavender and ridged layered foliage')
(ASSET/'layout.json').write_text(json.dumps(layout,indent=2)+'\n')
print('VEGETATION_GENERATED',json.dumps({k:v for k,v in report.items() if k!='original_foliage_bounds'}),flush=True)
