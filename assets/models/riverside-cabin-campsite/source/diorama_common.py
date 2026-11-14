"""Noms collection: shared modeling primitives, adapted from the approved potluck master.
Loaded by build_scenes.py; run that entry point to rebuild an individual scene.
"""
import bpy, math, random, json, sys
from pathlib import Path
from mathutils import Vector, Matrix
from math import sin,cos,pi
R=Path(__file__).resolve().parent; random.seed(28)
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
for c in list(bpy.data.collections):
 if c.name!='Collection':bpy.data.collections.remove(c)
COL=bpy.data.collections['Collection']; COL.name='00 Studio'; ROOT=None; GROUPS=[]
def collection(n,offset=(0,-1,0),start=1,end=30,assembly=True):
 global COL,ROOT
 COL=bpy.data.collections.new(n);bpy.context.scene.collection.children.link(COL);ROOT=None
 if assembly:
  ROOT=bpy.data.objects.new(n+' ROOT',None);COL.objects.link(ROOT);ROOT['assembly']=n;ROOT['incoming_offset']=offset;ROOT['start_frame']=start;ROOT['end_frame']=end;GROUPS.append(ROOT)
def link(o):
 for c in list(o.users_collection):c.objects.unlink(o)
 COL.objects.link(o)
 if ROOT:o.parent=ROOT
 return o
def linear(v):return v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4
def rgb(h):return (*[linear(int(h[i:i+2],16)/255) for i in (1,3,5)],1)
def material(n,h,rough=.8,metal=0,trans=0,emit=0):
 m=bpy.data.materials.new(n);m.use_nodes=True;p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=rgb(h);p.inputs['Roughness'].default_value=rough;p.inputs['Metallic'].default_value=metal;p.inputs['Transmission Weight'].default_value=trans;p.inputs['IOR'].default_value=1.46
 if emit:p.inputs['Emission Color'].default_value=rgb(h);p.inputs['Emission Strength'].default_value=emit
 m.diffuse_color=rgb(h);return m
M={k:material(k,*v) for k,v in {
'ivory':('#EDE5D6',.65),'wall':('#E4D8C4',.8),'walnut':('#4E3324',.4),'walnut2':('#63412A',.42),'oak':('#B78954',.48),'oak2':('#AD7E4B',.48),'oak3':('#C19663',.5),'gap':('#442F25',.7),'floor':('#BC915C',.45),'cream':('#DAD3C2',.85),'charcoal':('#3E4141',.85),'piping':('#292D2D',.85),'creamPiping':('#BEB5A4',.85),'rust':('#A95F35',.85),'black':('#171C1E',.3),'steel':('#A7ABAD',.27,.8),'steelDark':('#636B6E',.3,.7),'glass':('#EDF7F8',.08,0,1),'water':('#DBF3F3',.06,0,.95),'tea':('#A14C13',.1,0,.72),'cola':('#321B0D',.12,0,.35),'stone':('#E3D8C5',.36),'rug':('#B6A58A',.95),'rugline':('#98866C',.95),'paper':('#F3EDDB',.8),'bookCream':('#D5C9AD',.72),'bookRust':('#A27652',.7),'bookInk':('#57584B',.7),'ceramic':('#EEE8D7',.3),'darkPlate':('#293130',.32),'bamboo':('#B98D50',.55),'bambooDark':('#98703A',.55),'dough':('#EEDDC1',.63),'rice':('#F1EEE0',.58),'nori':('#253728',.66),'salmon':('#DB7952',.36),'salmonFat':('#F4D5AF',.45),'cucumber':('#789A48',.5),'avocado':('#C3CA77',.5),'beefCrust':('#5B3525',.65),'beef':('#BD796E',.62),'fat':('#D5B1A0',.55),'gravy':('#68432C',.35),'pasta':('#DCAD51',.42),'tomato':('#A84525',.43),'basil':('#48713B',.6),'fry':('#DCAF4C',.5),'chip':('#DCBA6A',.52),'cookie':('#C89C62',.66),'chocolate':('#493023',.6),'orange':('#D99127',.48),'grape':('#5D3545',.34),'lemon':('#E3BF45',.44),'redCan':('#B84537',.25,.6),'blueCan':('#3B6C87',.28,.6),'led':('#FFE0A1',.3,0,0,3.0),'sage':('#727966',.7),
}.items()}
for key in ['walnut','walnut2','oak','oak2','oak3','floor']:
 m=M[key];ns=m.node_tree.nodes;l=m.node_tree.links;p=ns.get('Principled BSDF');tex=ns.new('ShaderNodeTexNoise');tex.inputs['Scale'].default_value=3;tex.inputs['Detail'].default_value=2
 co=ns.new('ShaderNodeTexCoord');mul=ns.new('ShaderNodeVectorMath');mul.operation='MULTIPLY';mul.inputs[1].default_value=(2,65,5);l.new(co.outputs['Generated'],mul.inputs[0]);l.new(mul.outputs[0],tex.inputs['Vector'])
 ramp=ns.new('ShaderNodeValToRGB');c=p.inputs['Base Color'].default_value[:];ramp.color_ramp.elements[0].position=.15;ramp.color_ramp.elements[0].color=tuple(v*.83 for v in c[:3])+(1,);ramp.color_ramp.elements[1].position=.85;ramp.color_ramp.elements[1].color=tuple(min(1,v*1.12) for v in c[:3])+(1,);l.new(tex.outputs['Fac'],ramp.inputs[0]);l.new(ramp.outputs[0],p.inputs['Base Color'])
for key in ['cream','charcoal','rust','rug','beef','beefCrust','dough','cookie']:
 m=M[key];ns=m.node_tree.nodes;l=m.node_tree.links;tex=ns.new('ShaderNodeTexNoise');tex.inputs['Scale'].default_value=155 if key in ['cream','charcoal','rust'] else 75;tex.inputs['Detail'].default_value=2;bump=ns.new('ShaderNodeBump');bump.inputs['Strength'].default_value=.16;bump.inputs['Distance'].default_value=.0018;l.new(tex.outputs['Fac'],bump.inputs['Height']);l.new(bump.outputs[0],ns.get('Principled BSDF').inputs['Normal'])
def assign(o,m):o.data.materials.append(M[m] if isinstance(m,str) else m);return o
def smooth(o):
 for p in o.data.polygons:p.use_smooth=True
 return o
def bevel(o,b=.02,s=4):
 md=o.modifiers.new('Rounded edges','BEVEL');md.width=b;md.segments=s;o.modifiers.new('Weighted normals','WEIGHTED_NORMAL');return o
def box(n,loc,dims,m,b=.02):
 bpy.ops.mesh.primitive_cube_add(size=1,location=loc);o=link(bpy.context.object);o.name=n;o.dimensions=dims;bpy.ops.object.transform_apply(location=False,rotation=False,scale=True);assign(o,m)
 if b:bevel(o,b)
 return o
def mesh(n,v,f,m):
 d=bpy.data.meshes.new(n);d.from_pydata(v,[],f);d.update();o=bpy.data.objects.new(n,d);COL.objects.link(o)
 if ROOT:o.parent=ROOT
 assign(o,m);return o
def uv(n,loc,sc,m,seg=24,rings=12):
 bpy.ops.mesh.primitive_uv_sphere_add(segments=seg,ring_count=rings,location=loc);o=link(bpy.context.object);o.name=n;o.scale=sc;bpy.ops.object.transform_apply(location=False,rotation=False,scale=True);assign(o,m);return smooth(o)
def cyl(n,loc,r,h,m,vertices=64):
 bpy.ops.mesh.primitive_cylinder_add(vertices=vertices,radius=r,depth=h,location=loc);o=link(bpy.context.object);o.name=n;assign(o,m);bevel(o,min(.007,h*.15),3);return smooth(o)
def rod(n,a,b,r,m,vertices=20):
 a,b=Vector(a),Vector(b);o=cyl(n,(a+b)/2,r,(b-a).length,m,vertices);o.rotation_euler=(b-a).to_track_quat('Z','Y').to_euler();return o
def curve(n,pts,r,m,cyclic=False,poly=False):
 d=bpy.data.curves.new(n,'CURVE');d.dimensions='3D';d.resolution_u=5;d.bevel_depth=r;d.bevel_resolution=2;s=d.splines.new('POLY' if poly else 'BEZIER')
 if poly:
  s.points.add(len(pts)-1)
  for p,co in zip(s.points,pts):p.co=(*co,1)
 else:
  s.bezier_points.add(len(pts)-1)
  for p,co in zip(s.bezier_points,pts):p.co=co;p.handle_left_type='AUTO';p.handle_right_type='AUTO'
 s.use_cyclic_u=cyclic;o=bpy.data.objects.new(n,d);COL.objects.link(o)
 if ROOT:o.parent=ROOT
 assign(o,m);return o
def torus(n,loc,r,minor,m,rot=(0,0,0),steps=64):
 bpy.ops.mesh.primitive_torus_add(major_radius=r,minor_radius=minor,major_segments=steps,minor_segments=8,location=loc,rotation=rot);o=link(bpy.context.object);o.name=n;assign(o,m);return smooth(o)
def lathe(n,loc,profile,m,steps=64,scale=(1,1,1)):
 v=[(loc[0]+r*cos(i*2*pi/steps)*scale[0],loc[1]+r*sin(i*2*pi/steps)*scale[1],loc[2]+z*scale[2]) for r,z in profile for i in range(steps)];f=[(j*steps+i,j*steps+(i+1)%steps,(j+1)*steps+(i+1)%steps,(j+1)*steps+i)for j in range(len(profile)-1)for i in range(steps)];return smooth(mesh(n,v,f,m))
def local_group(n,loc,angle,fn):
 before=set(bpy.data.objects);parent=ROOT;fn();new=set(bpy.data.objects)-before;o=bpy.data.objects.new(n,None);COL.objects.link(o)
 if parent:o.parent=parent
 o.location=loc;o.rotation_euler.z=angle
 for child in new:
  if child.parent==parent:child.parent=o
 return o
def pill(n,loc,dims,m):
 power=.32;nu=40;nv=20;v=[]
 def sp(x):return math.copysign(abs(x)**power,x)
 for j in range(nv+1):
  b=-pi/2+pi*j/nv
  for i in range(nu):
   a=2*pi*i/nu;v.append((loc[0]+dims[0]/2*sp(cos(b))*sp(cos(a)),loc[1]+dims[1]/2*sp(cos(b))*sp(sin(a)),loc[2]+dims[2]/2*sp(sin(b))))
 f=[(j*nu+i,j*nu+(i+1)%nu,(j+1)*nu+(i+1)%nu,(j+1)*nu+i)for j in range(nv)for i in range(nu)];return smooth(mesh(n,v,f,m))
def rounded_points(cx,cy,z,w,d,r=.06):
 pts=[]
 for x,y,a in [(cx+w/2-r,cy+d/2-r,0),(cx-w/2+r,cy+d/2-r,pi/2),(cx-w/2+r,cy-d/2+r,pi),(cx+w/2-r,cy-d/2+r,3*pi/2)]:
  for j in range(9):
   t=a+j*pi/16;pts.append((x+r*cos(t),y+r*sin(t),z))
 return pts
def rounded_slab(n,loc,w,d,h,r,m):
 top=rounded_points(loc[0],loc[1],loc[2]+h/2,w,d,r);bottom=[(x,y,loc[2]-h/2)for x,y,z in top];N=len(top);f=[tuple(reversed(range(N))),tuple(range(N,2*N))]+[(i,(i+1)%N,(i+1)%N+N,i+N)for i in range(N)];o=mesh(n,bottom+top,f,m);bevel(o,min(.015,h*.2),3);return o

def plate(n,x,y,z,r=.15,m='ceramic',sx=1,sy=1):
 return lathe(n,(x,y,z),[(0,0),(.72*r,0),(.95*r,.012),(r,.020),(r,.028),(.96*r,.032),(.77*r,.018),(0,.018)],m,64,(sx,sy,1))
def bowl(n,x,y,z,r=.2,m='ceramic'):
 return lathe(n,(x,y,z),[(0,0),(.58*r,0),(.66*r,.02),(.90*r,.065),(r,.105),(r-.007,.112),(r-.015,.104),(.85*r,.065),(.58*r,.025),(0,.025)],m,64)
def glass(n,x,y,z,r=.035,h=.15,drink=None):
 lathe(n,(x,y,z),[(0,0),(r*.83,0),(r,.012),(r,h),(r-.0025,h),(r-.003,h*.1),(0,h*.1)],'glass',48)
 if drink:lathe(n+' liquid',(x,y,z),[(0,.018),(r-.004,.018),(r-.004,h*.68),(0,h*.68)],drink,48)
def mug(n,x,y,z,r=.045,h=.09):
 lathe(n,(x,y,z),[(0,0),(r*.85,0),(r,.01),(r,h),(r-.006,h),(r-.006,.012),(0,.012)],'ceramic',48)
 curve(n+' handle',[(x+r*.9,y,z+h*.77),(x+r*1.65,y,z+h*.82),(x+r*1.75,y,z+h*.40),(x+r*.9,y,z+h*.28)],.007,'ceramic')
 lathe(n+' coffee',(x,y,z),[(0,h*.78),(r-.008,h*.78)],'cola',48)
def book(n,x,y,z,w=.10,h=.38,d=.25,idx=0):
 m=['bookCream','bookRust','bookInk'][idx%3];box(n+' pages',(x,y,z+h/2),(w-.01,d-.01,h-.012),'paper',.003)
 for xx in [x-w/2,x+w/2]:box(n+' cover',(xx,y,z+h/2),(.006,d,h),m,.002)
 box(n+' spine',(x,y-d/2,z+h/2),(w,.018,h),m,.003)
 for zz in [z+.04,z+h-.04]:box(n+' spine stripe',(x,y-d/2-.01,zz),(w*.7,.002,.003),'paper',.001)


# Shared matte finish for the new collection.
for key,m in M.items():
 p=m.node_tree.nodes.get("Principled BSDF")
 if key not in ["glass","water","tea","cola","led"]:
  p.inputs["Roughness"].default_value=max(.48 if p.inputs["Metallic"].default_value else .75,p.inputs["Roughness"].default_value)
