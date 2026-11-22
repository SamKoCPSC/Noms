import bpy
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parent.parent
bpy.ops.wm.open_mainfile(filepath=str(root/'models/brick-loft-kitchen.blend'))
s=bpy.context.scene;s.camera.location=(-8,-11,15);s.camera.rotation_euler=(Vector((0,0,1))-s.camera.location).to_track_quat('-Z','Y').to_euler();s.camera.data.ortho_scale=9.2
s.render.resolution_x=1300;s.render.resolution_y=1200;s.cycles.samples=32;s.render.threads=6
s.render.filepath=str(root/'previews/brick-loft-layout.png');bpy.ops.render.render(write_still=True)
