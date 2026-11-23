"""Render screenshots from the saved master without modifying its cameras on disk."""
import bpy,sys
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[1];name=root.name
bpy.ops.wm.open_mainfile(filepath=str(root/'models'/f'{name}.blend'))
s=bpy.context.scene;s.render.threads=6;s.cycles.samples=48;s.cycles.use_denoising=False
views=[(name,None,None,None,1440,1260)]
if 'kitchen' in name:
 views += [('prime-rib-detail',(2.1,-3.0,3.6),(-.15,.10,1.23),2.45,1400,1050),('plated-dinner-detail',(.5,-3.2,3.4),(.1,-2.16,.93),2.0,1400,1050)]
else:
 views += [('campsite-details',(7,-10,7),(-.7,-.4,1.4),8.0,1400,1050)]
if '--hero' in sys.argv:views=views[:1]
if '--details' in sys.argv:views=views[1:]
for label,eye,target,scale,w,h in views:
 if eye:s.camera.location=eye;s.camera.rotation_euler=(Vector(target)-s.camera.location).to_track_quat('-Z','Y').to_euler();s.camera.data.ortho_scale=scale
 s.render.resolution_x=w;s.render.resolution_y=h;s.render.resolution_percentage=100
 s.render.filepath=str(root/'previews'/f'{label}.png');bpy.ops.render.render(write_still=True)
