"""Render a saved native master at final quality, preserving a portable output path."""
import bpy
from pathlib import Path
root=Path(__file__).resolve().parent.parent
path=Path(bpy.data.filepath)
scene=bpy.context.scene
scene.cycles.samples=64
scene.render.resolution_x=1600
scene.render.resolution_y=1400
scene.render.resolution_percentage=100
scene.render.filepath=f'//../previews/{path.stem}.png'
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(path),compress=True)
scene.render.filepath=str(root/'previews'/f'{path.stem}.png')
bpy.ops.render.render(write_still=True)
print('FINAL_RENDER_COMPLETE',path.stem,flush=True)
