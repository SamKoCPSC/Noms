"""Small display-only GLB; keeps full-detail deliverables unchanged."""
import bpy,json
from pathlib import Path
r=Path(__file__).parent
bpy.ops.wm.read_factory_settings(use_empty=True);s=bpy.context.scene;s.render.fps=30;s.frame_start=0;s.frame_end=1356;s.name='Penguini bakes cookies'
bpy.ops.import_scene.gltf(filepath=str(r/'penguini-cookie-kitchen-baking.glb'))
s.frame_set(0)
before=0;after=0;parts=[]
for o in s.objects:
 if o.type!='MESH':continue
 count=sum(len(p.vertices)-2 for p in o.data.polygons);before+=count
 # Preserve lettering and small components. Broad dense surfaces dominate cost.
 if count>1500 and not (o.name.startswith(('Static kitchen |','Reading book |')) or 'Eye' in o.name or 'spoon' in o.name.lower() or 'dough' in o.name.lower()):
  mod=o.modifiers.new('Lightweight preview only','DECIMATE');mod.ratio=max(.045,min(.16,4000/count))
  mod.use_collapse_triangulate=True
  o.modifiers.move(len(o.modifiers)-1,0)
  bpy.context.view_layer.objects.active=o
  bpy.ops.object.modifier_apply(modifier=mod.name)
 e=o.evaluated_get(bpy.context.evaluated_depsgraph_get());m=e.to_mesh();m.calc_loop_triangles();n=len(m.loop_triangles);after+=n;e.to_mesh_clear()
 parts.append({'object':o.name,'before':count,'after':n})
bpy.ops.object.select_all(action='SELECT')
bpy.ops.export_scene.gltf(filepath=str(r/'preview-lite.glb'),export_format='GLB',use_selection=True,export_apply=True,export_animations=True,export_force_sampling=True,export_frame_range=True,export_frame_step=1,export_cameras=False,export_lights=False,export_animation_mode='SCENE',export_anim_scene_split_object=False)
report={'purpose':'Browser/video preview only; full-detail Blender and GLB unchanged','triangles_before':before,'triangles_after':after,'parts':parts}
(r/'preview-lod.json').write_text(json.dumps(report,indent=2));print('PREVIEW_LOD',before,after,flush=True)
