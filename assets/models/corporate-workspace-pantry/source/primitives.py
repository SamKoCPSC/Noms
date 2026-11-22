"""Direct-data primitives shared with the supermarket revision; loaded after scene helpers."""
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

def text(n,words,loc,size=.12,mat='ivory',rot=(pi/2,0,0),align='CENTER'):
    d=bpy.data.curves.new(n,'FONT');d.body=words;d.align_x=align;d.size=size;d.resolution_u=2
    o=bpy.data.objects.new(n,d);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.rotation_euler=rot;assign(o,mat);return o
