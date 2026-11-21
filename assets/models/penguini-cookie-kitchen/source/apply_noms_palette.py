"""Apply the Noms light-theme palette to the editable kitchen materials.

Run before baking, with Blender 4.3.2:
  blender -b -t 4 --python apply_noms_palette.py -- MASTER.blend assets/main.css

Only material colors and palette metadata change. Penguini, food, wood, metal,
the logo book, geometry, lighting and animation retain their existing design.
"""
import colorsys
import json
from pathlib import Path
import re
import sys

import bpy


MATERIAL_TOKENS = {
    "Kitchen | terracotta painted cabinetry": "accent",
    "Kitchen | inset terracotta": "accent-hover",
    "Kitchen | soft plaster": "surface",
    # Keep the historical material ID so older source tools can still find it.
    "Kitchen | seafoam glazed backsplash": "bg-gradient-3",
    "Kitchen | sage ceramic": "success",
    "Kitchen | warm ivory enamel": "bg-base",
    "Kitchen | ivory tile": "surface",
    "Kitchen | biscuit tile": "bg-gradient-4",
    "Kitchen | warm grout": "shadow-dark",
    "Kitchen | unbleached linen": "surface",
    "Kitchen | warm stove graphite": "text-primary",
    "Kitchen | book print": "text-secondary",
}


# Increase chroma in sRGB before converting the paint to Blender's linear color.
# Skin, paper and the cream worktop retain their original lightness and hue.
SATURATION_GAIN = {
    "accent": 1.38,
    "accent-hover": 1.38,
    "success": 1.5,
    "bg-gradient-3": 1.65,
    "bg-gradient-4": 1.25,
}


def paint_color(hex_color, token):
    gain = SATURATION_GAIN.get(token, 1)
    if gain == 1:
        return hex_color
    rgb = [int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    hue, lightness, saturation = colorsys.rgb_to_hls(*rgb)
    rgb = colorsys.hls_to_rgb(hue, lightness, min(1, saturation * gain))
    return "#" + "".join(f"{round(channel * 255):02X}" for channel in rgb)


def linear_rgba(hex_color, alpha):
    srgb = [int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    return (*[v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
              for v in srgb], alpha)


def apply(master, css):
    # Read just :root: dark-mode overrides must not overwrite these base colors.
    light = re.search(r":root\s*\{(.*?)\n\}", css.read_text(), re.S).group(1)
    tokens = dict(re.findall(r"--([\w-]+):\s*(#[\da-fA-F]{6})\s*;", light))
    bpy.ops.wm.open_mainfile(filepath=str(master))
    palette = {}
    for name, token in MATERIAL_TOKENS.items():
        material = bpy.data.materials[name]
        color = paint_color(tokens[token], token)
        material.diffuse_color = linear_rgba(color, material.diffuse_color[3])
        for node in material.node_tree.nodes:
            if node.type == 'BSDF_PRINCIPLED':
                base = node.inputs['Base Color']
                assert not base.is_linked, f"Unexpected texture on {name}"
                base.default_value = linear_rgba(color, base.default_value[3])
        palette[name] = {"token": f"--{token}", "srgb": color}
    scene = bpy.context.scene
    scene['palette'] = 'Noms | ivory, terracotta, sage and charcoal'
    scene['palette_materials'] = json.dumps(palette, sort_keys=True)
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(master))
    print('NOMS_PALETTE_APPLIED', json.dumps(palette, sort_keys=True), flush=True)


if __name__ == '__main__':
    args = sys.argv[sys.argv.index('--') + 1:]
    if len(args) != 2:
        raise SystemExit('Expected MASTER.blend and assets/main.css after --')
    apply(Path(args[0]).resolve(), Path(args[1]).resolve())
