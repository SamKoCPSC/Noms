"""Render an alternate view from the editable master."""
import bpy
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parent.parent
bpy.ops.wm.open_mainfile(filepath=str(root/'models/modern-baking-lab.blend'))
s=bpy.context.scene
s.camera.location=(9,-12,14);s.camera.rotation_euler=(Vector((.65,-.55,1.0))-s.camera.location).to_track_quat('-Z','Y').to_euler();s.camera.data.ortho_scale=13.8
s.render.resolution_x=1500;s.render.resolution_y=1400;s.cycles.samples=32;s.render.threads=6
s.render.filepath=str(root/'previews/modern-baking-lab-layout.png');bpy.ops.render.render(write_still=True)
