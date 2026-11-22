"""Split the existing ground at grid-aligned boundaries without gaps or overlap."""
import bpy,json
from common import ASSET,CONFIG,xyz
def split_ground():
    collection=bpy.data.collections['World terrain — blockout']
    original=max(collection.objects,key=lambda o:o.dimensions.x*o.dimensions.y)
    material=original.active_material
    item=json.loads((ASSET/'source/world-environment.json').read_text())[0]
    vertices,normals,indices=item['vertices'],item['normals'],item['indices']
    x0,x1,z0,z1=CONFIG['ground_bounds_xz'];xm=(x0+x1)/2;zm=(z0+z1)/2
    bounds=[(x0,xm,z0,zm),(xm,x1,z0,zm),(x0,xm,zm,z1),(xm,x1,zm,z1)]
    groups=[[] for _ in range(5)]
    for i in range(0,len(indices),3):
        face=indices[i:i+3];x=sum(vertices[j][0] for j in face)/3;z=sum(vertices[j][2] for j in face)/3
        slot=next((j for j,b in enumerate(bounds) if b[0]<=x<b[1] and b[2]<=z<b[3]),4);groups[slot].append(face)
    assert sum(map(len,groups))*3==len(indices)
    bpy.data.objects.remove(original,do_unlink=True);result=[]
    for i,faces in enumerate(groups):
        used=sorted({j for f in faces for j in f});remap={j:k for k,j in enumerate(used)}
        mesh=bpy.data.meshes.new('Daylight ground '+str(i));mesh.from_pydata([xyz(vertices[j]) for j in used],[],[[remap[j] for j in f] for f in faces]);mesh.update()
        mesh.polygons.foreach_set('use_smooth',[True]*len(mesh.polygons));mesh.normals_split_custom_set_from_vertices([xyz(normals[j]) for j in used])
        mesh.materials.append(material.copy());obj=bpy.data.objects.new(mesh.name,mesh);collection.objects.link(obj);obj['ground_tile']=i
        if i<4:obj['tile_bounds']=bounds[i]
        result.append(obj)
    return result
