def croissant(n,loc,width=.3,angle=0):
 global PASTRY
 if 'PASTRY' not in globals():
  with bpy.data.libraries.load(str(R/'assets/croissant-library.blend'),link=False) as (a,b):b.objects=[n for n in a.objects if n.startswith('Croissant 1')]
  src=b.objects[0];d=src.data.copy();d.transform(src.matrix_world);pts=[v.co for v in d.vertices];lo=Vector([min(p[i]for p in pts)for i in range(3)]);hi=Vector([max(p[i]for p in pts)for i in range(3)]);center=Vector(((lo.x+hi.x)/2,(lo.y+hi.y)/2,lo.z))
  d.transform(Matrix.Scale(1/max(hi.x-lo.x,hi.y-lo.y),4)@Matrix.Translation(-center));PASTRY=d;bpy.data.objects.remove(src,do_unlink=True)
  for m in d.materials:
   if m and m.use_nodes:
    p=m.node_tree.nodes.get('Principled BSDF')
    if p:p.inputs['Roughness'].default_value=.78
 o=bpy.data.objects.new(n,PASTRY);COL.objects.link(o);o.parent=ROOT;o.location=loc;o.scale=(width,)*3;o.rotation_euler.z=angle;return o
