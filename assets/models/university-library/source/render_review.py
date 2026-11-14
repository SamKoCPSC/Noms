"""Render the saved master and validate source geometry (no scene edits)."""
import bpy,json,math,sys
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parent.parent
bpy.ops.wm.open_mainfile(filepath=str(root/'models/university-library.blend'))
s=bpy.context.scene
objects=[o for o in s.objects if o.type in {'MESH','CURVE','FONT'} and o.name!='Studio ground']
roots=[o for o in s.objects if 'assembly' in o]
dg=bpy.context.evaluated_depsgraph_get();triangles=0
for o in objects:
 assert all(math.isfinite(v) for row in o.matrix_world for v in row),o.name
 ancestor=o.parent
 while ancestor and ancestor not in roots:ancestor=ancestor.parent
 assert ancestor in roots,o.name
 e=o.evaluated_get(dg);m=e.to_mesh();m.calc_loop_triangles();triangles+=len(m.loop_triangles);e.to_mesh_clear()
report={'model':'university-library','reconstruction':True,'revision':int(s.get('revision',0)),'geometry_objects':len(objects),'assembly_groups':len(roots),'evaluated_triangles':triangles,'external_images':[i.filepath for i in bpy.data.images if i.source=='FILE' and not i.packed_file],'finite_transforms':True,'all_geometry_grouped':True,'baked_shadows':False}
assert len(roots)==13
assert s.get('revision')==4
assert s.get('window_chairs')==4 and s.get('stair_treads')==24
assert s.get('stair_rotation_degrees')==360
assert not any(any(term in o.name.lower() for term in ['campus tree','campus garden','foliage','doorway','exterior garden']) for o in objects)
panels=[o for o in objects if o.name.startswith('Blue simulated window panel')]
assert len(panels)==18
for o in panels:assert o.data.materials[0].node_tree.nodes.get('Principled BSDF').inputs['Transmission Weight'].default_value==0
# Measure the built wedge geometry, not only its declared metadata.
steps=sorted([o for o in objects if o.name.startswith('Curved stair tread')],key=lambda o:max(v.co.z for v in o.data.vertices))
assert len(steps)==24
sweeps=[]
for step in steps:
 a=step.data.vertices[0].co;b=step.data.vertices[8].co
 start_angle=math.atan2(a.y-2.56,a.x+3.65);end_angle=math.atan2(b.y-2.56,b.x+3.65)
 sweeps.append((end_angle-start_angle)%(2*math.pi))
measured_degrees=math.degrees(sum(sweeps))
assert abs(measured_degrees-360)<.001,measured_degrees
for before,after in zip(steps,steps[1:]):
 a=before.data.vertices[8].co;b=after.data.vertices[0].co
 assert math.hypot(a.x-b.x,a.y-b.y)<1e-5
assert math.hypot(steps[-1].data.vertices[8].co.x-steps[0].data.vertices[0].co.x,steps[-1].data.vertices[8].co.y-steps[0].data.vertices[0].co.y)<1e-5
report['stair_rotation_degrees']=round(measured_degrees,6)
report['opaque_blue_panels']=len(panels)
report['features']={k:int(s[k]) for k in ['reading_chairs','window_chairs','portraits','art_glass_ribbons','stair_treads']}
assert len([o for o in objects if o.name.startswith('Laptop beveled base')])==4
assert len([o for o in objects if 'center binding' in o.name and o.name.startswith('Open study textbook')])==5
report['features']['study_laptops']=4
report['features']['open_study_books']=5
assert not report['external_images']
(root/'native-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('VALIDATED',json.dumps(report),flush=True)
s.render.resolution_percentage=100;s.render.resolution_x=1600;s.render.resolution_y=1400;s.cycles.samples=64;s.cycles.use_denoising=False;s.render.threads=6
s.render.filepath=str(root/'previews/university-library.png');bpy.ops.render.render(write_still=True)
s.camera.location=(-5,-8,8);s.camera.rotation_euler=(Vector((-.15,-.45,.85))-s.camera.location).to_track_quat('-Z','Y').to_euler();s.camera.data.ortho_scale=6.4
s.render.resolution_x=1500;s.render.resolution_y=1200;s.render.filepath=str(root/'previews/university-library-detail.png');bpy.ops.render.render(write_still=True)
