"""Render the three editable office posters from the saved Blender master."""
from pathlib import Path
import bpy
from mathutils import Vector

scene=bpy.context.scene
camera=scene.camera
camera.data.type='ORTHO'
scene.render.resolution_x=900
scene.render.resolution_y=650
scene.render.resolution_percentage=100
scene.cycles.samples=16
scene.render.threads=2
shots=[
    ('offline-poster',(1,-2.62,2.40),(-4.25,-2.62,2.40),2.08),
    ('circle-back-poster',(1,-.56,2.40),(-4.25,-.56,2.40),2.08),
    ('bandwidth-poster',(-2.98,-2,2.58),(-2.98,3.65,2.58),1.52),
]
for name,position,target,scale in shots:
    camera.location=position
    camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale=scale
    scene.render.filepath=str(Path(__file__).resolve().parent.parent/'previews'/f'{name}.png')
    bpy.ops.render.render(write_still=True)
