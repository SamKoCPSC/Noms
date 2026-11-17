"""Build one unioned, terrain-following stone network (Python + Shapely 2.2).

Inputs are the approved walking routes and entrance measurements. No room or
camera moves. Voronoi cells partition the footprint before the stones are inset:
junctions therefore cannot contain overlapping strips or conflicting heights.
"""
import gzip, hashlib, json, math, random
from pathlib import Path
import numpy as np
from shapely import constrained_delaunay_triangles, voronoi_polygons
from shapely.geometry import LineString, MultiPoint, Point, Polygon
from shapely.ops import unary_union
from shapely.prepared import prep
from shapely.geometry.polygon import orient

ROOT = Path(__file__).resolve().parents[3]
ASSET = ROOT / 'assets/models/terraced-village'
layout = json.loads((ASSET/'layout.json').read_text())
routes = json.loads((ASSET/'source/paths.json').read_text())
entrances = json.loads((ASSET/'source/entrance-verification.json').read_text())['entrances']
REMOVED = ['Ground-level walking path', 'Path-to-entrance connection',
           'Inset entrance forecourt', 'Entrance paving stone', 'Outdoor timber plank']

def smooth(t):
    t = min(1, max(0, t))
    return t*t*(3-2*t)

def ground(x, z):
    value = .18-z*.014+math.sin(x*.045)*math.cos(z*.035)*.3
    for stop in layout['stops']:
        distance = math.hypot(x-stop['position'][0], z-stop['position'][2])
        blend = 1-smooth((distance-stop['width']*.76)/7)
        value += (stop['position'][1]-.15-value)*blend
    return value

footprints = [LineString([(p[0], p[2]) for p in r]).simplify(.035).buffer(
    layout['walk']['path_width']/2, quad_segs=8) for r in routes]
patios, branches = [], []
for stop, entry in zip(layout['stops'], entrances):
    assert stop['id']==entry['id'], 'Entrance measurements must follow the layout order'
    a = np.array(entry['approach_start'])[::2]
    b = np.array(entry['approach_end'])[::2]
    threshold = np.array(entry['threshold'])[::2]
    normal = b-threshold; normal /= np.linalg.norm(normal)
    tangent = np.array([normal[1], -normal[0]])
    settings = stop['entrance']; start = entry['steps']*.46+.55
    center = threshold+normal*(start+settings['depth']/2)
    half = np.array([settings['width']/2, settings['depth']/2])
    polygon = Polygon([center+tangent*u*half[0]+normal*v*half[1]
                       for u, v in [(-1,-1), (1,-1), (1,1), (-1,1)]])
    patios.append((center, tangent, normal, half, entry['approach_end'][1]-.015))
    control = b+normal*2.5
    points = [(1-t)**2*a+2*(1-t)*t*control+t*t*b for t in np.linspace(0,1,65)]
    branch = LineString(points); branches.append(branch)
    footprints.extend([branch.buffer(1.25, quad_segs=8), polygon])

# Round inward junction corners only after the strips are united. There is a
# single boundary and a single height function, including the entrance patios.
network = unary_union(footprints).buffer(.12, quad_segs=4).buffer(-.12, quad_segs=4)
network = orient(network, sign=1)
assert network.is_valid and network.geom_type == 'Polygon', 'Disconnected path network'
prepared = prep(network)

def height(x, z):
    value = ground(x,z)+.19
    for center, tangent, normal, half, level in patios:
        dx, dz = x-center[0], z-center[1]
        u, v = dx*tangent[0]+dz*tangent[1], dx*normal[0]+dz*normal[1]
        distance = math.hypot(max(0,abs(u)-half[0]),max(0,abs(v)-half[1]))
        value += (level-value)*(1-smooth(distance/2))
    return max(ground(x,z)+.07, value)

rng = random.Random(4821)
x0,z0,x1,z1 = network.bounds
near = prep(network.buffer(2))
sites = []
for row in range(math.floor(z0/.65)-4, math.ceil(z1/.65)+4):
    for col in range(math.floor(x0/.9)-4, math.ceil(x1/.9)+4):
        x = (col+.5*(row%2))*.9+rng.uniform(-.11,.11)
        z = row*.65+rng.uniform(-.09,.09)
        if near.contains(Point(x,z)): sites.append((x,z))
diagram = voronoi_polygons(MultiPoint(sites), extend_to=network.envelope.buffer(3))
cells = []
for cell in diagram.geoms:
    if not prepared.intersects(cell): continue
    clipped = cell.intersection(network)
    for polygon in ([clipped] if clipped.geom_type=='Polygon' else getattr(clipped,'geoms',[])):
        if polygon.geom_type=='Polygon' and polygon.area>1e-10: cells.append(polygon)
covered = unary_union(cells)
assert covered.symmetric_difference(network).area < 1e-7
assert abs(sum(p.area for p in cells)-network.area) < 1e-7, 'Overlapping cells'

def linear(hex_color):
    rgb = [int(hex_color[i:i+2],16)/255 for i in (1,3,5)]
    return [v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4 for v in rgb]

palette = ['#b8b5a7','#c6c1b1','#aaa99f','#c1b9a7','#b4b5ad','#cbc6b8']
meshes = [dict(name='Continuous stone-path grout bed',vertices=[],normals=[],indices=[],color=linear('#8f9085'),roughness=1,specular_intensity=.08)]
meshes += [dict(name=f'Beveled matte limestone {i+1}',vertices=[],normals=[],indices=[],color=linear(color),roughness=1,specular_intensity=.08) for i,color in enumerate(palette)]

def triangle(mesh, a,b,c, upward=None):
    a,b,c = np.array(a),np.array(b),np.array(c)
    normal = np.cross(b-a,c-a)
    if np.linalg.norm(normal)<1e-12:return
    if upward is not None and (normal[1]>0)!=upward:b,c=c,b;normal=-normal
    normal /= np.linalg.norm(normal)
    n=len(mesh['vertices']);mesh['vertices'].extend([[round(float(v),6) for v in p] for p in (a,b,c)])
    mesh['normals'].extend([normal.tolist()]*3);mesh['indices'].extend([n,n+1,n+2])

def surface(mesh, polygon, offset, upward=True):
    for tri in constrained_delaunay_triangles(polygon).geoms:
        pts = [(x,offset(x,z) if callable(offset) else height(x,z)+offset,z) for x,z in list(tri.exterior.coords)[:3]]
        triangle(mesh,*pts,upward=upward)

# A continuous grout bed remains underneath all deliberate stone joints. Its
# top/bottom use the exact same partition, with no interior vertical walls.
for cell in cells:
    surface(meshes[0],cell,-.04)
    surface(meshes[0],cell,lambda x,z:ground(x,z)-.035,False)
    # Use cell-split boundary edges so the skirt shares the top's vertices,
    # even where a long perimeter segment crosses several paving cells.
    cell=orient(cell,sign=1)
    for ring in [cell.exterior,*cell.interiors]:
        pts=list(ring.coords)
        for (x,z),(u,v) in zip(pts,pts[1:]):
            if network.boundary.distance(Point((x+u)/2,(z+v)/2))>1e-7:continue
            a=(x,height(x,z)-.04,z);b=(u,height(u,v)-.04,v)
            c=(u,ground(u,v)-.035,v);d=(x,ground(x,z)-.035,z)
            triangle(meshes[0],a,b,c);triangle(meshes[0],a,c,d)

stones=0;clearance=100
for i,cell in enumerate(cells):
    inset=cell.buffer(-.025,join_style='mitre')
    if inset.is_empty or inset.geom_type!='Polygon' or inset.area<.025:continue
    # Simplify near-collinear clipping vertices before the small edge bevel.
    lower=inset.simplify(.008,preserve_topology=True)
    if not cell.covers(lower):lower=inset
    lower=orient(lower,sign=1)
    cx,cz=lower.centroid.coords[0]
    lower_pts=list(lower.exterior.coords)[:-1]
    upper_pts=[]
    for x,z in lower_pts:
        length=math.hypot(x-cx,z-cz);scale=max(.8,1-.025/max(length,.001))
        upper_pts.append((cx+(x-cx)*scale,cz+(z-cz)*scale))
    upper=Polygon(upper_pts)
    if not upper.is_valid or not lower.buffer(1e-8).covers(upper):continue
    mesh=meshes[1+i%len(palette)];surface(mesh,upper,0)
    surface(mesh,lower,-.065,False)
    for j in range(len(lower_pts)):
        k=(j+1)%len(lower_pts);x,z=lower_pts[j];u,v=lower_pts[k]
        X,Z=upper_pts[j];U,V=upper_pts[k]
        a=(X,height(X,Z),Z);b=(U,height(U,V),V)
        c=(u,height(u,v)-.026,v);d=(x,height(x,z)-.026,z)
        triangle(mesh,a,b,c);triangle(mesh,a,c,d)
        e=(u,height(u,v)-.065,v);f=(x,height(x,z)-.065,z)
        triangle(mesh,d,c,e);triangle(mesh,d,e,f)
        clearance=min(clearance,height(x,z)-ground(x,z))
    stones+=1

assert clearance >= .069
for route in routes:
    assert all(network.covers(Point(p[0],p[2])) for p in route)
assert all(network.covers(b) for b in branches)
output=ASSET/'daylight/source/stone-paths.json.gz'
payload=json.dumps(dict(remove_names=REMOVED,meshes=meshes),separators=(',',':')).encode()
output.write_bytes(gzip.compress(payload,mtime=0))
report=dict(connected_components=1,valid_footprint=True,stone_count=stones,
    partition_overlap_area=abs(sum(p.area for p in cells)-network.area),
    uncovered_area=covered.symmetric_difference(network).area,minimum_terrain_clearance=clearance,
    entrances_connected=len(branches),walking_routes_covered=len(routes),
    one_height_field=True,continuous_grout_bed=True,beveled_edges=True,
    triangles=sum(len(m['indices'])//3 for m in meshes),sha256=hashlib.sha256(output.read_bytes()).hexdigest())
(ASSET/'daylight/source/stone-path-verification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
