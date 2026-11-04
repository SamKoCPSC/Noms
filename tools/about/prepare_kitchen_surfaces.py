"""Pack Blender's unlit kitchen surface bakes for WebGL, without display grading.
Usage: python tools/about/prepare_kitchen_surfaces.py EXPORT_DIR ASSET_DIR
Normal RG stores octahedral object normals, B stores roughness. Opaque RGB avoids
browser alpha premultiplication reducing precision in smooth glossy normals.
Color RGB stores sRGB albedo and A stores assembly-local cavity occlusion.
"""
from pathlib import Path
import sys
import numpy as np
from PIL import Image

source, output = map(Path, sys.argv[1:3])
output.mkdir(parents=True, exist_ok=True)
Image.open(source/'kitchen-surface-color.png').convert('RGBA').save(
    output/'kitchen-surface-color.webp', lossless=True, method=6)
raw = np.array(Image.open(source/'kitchen-surface-normal.png').convert('RGBA'))
n = raw[:, :, :3].astype(np.float32)/127.5-1.
n /= np.maximum(np.abs(n).sum(axis=2, keepdims=True), 1e-6)
xy = n[:, :, :2]
folded = (1.-np.abs(xy[:, :, ::-1]))*np.where(xy >= 0., 1., -1.)
xy = np.where(n[:, :, 2:] < 0., folded, xy)
encoded = np.empty(raw.shape[:2]+(3,), dtype=np.uint8)
encoded[:, :, :2] = np.round((xy*.5+.5)*255.).clip(0,255).astype(np.uint8)
encoded[:, :, 2] = raw[:, :, 3]
# Verify the lossy direction encoding, including negative-Z normals and poles.
f = encoded[:, :, :2].astype(np.float32)/127.5-1.
decoded = np.concatenate([f, 1.-np.abs(f).sum(axis=2, keepdims=True)], axis=2)
decoded[:, :, :2] = np.where(decoded[:, :, 2:] < 0.,
    (1.-np.abs(f[:, :, ::-1]))*np.where(f >= 0., 1., -1.), f)
decoded /= np.maximum(np.linalg.norm(decoded, axis=2, keepdims=True), 1e-6)
original = raw[:, :, :3].astype(np.float32)/127.5-1.
original /= np.maximum(np.linalg.norm(original, axis=2, keepdims=True), 1e-6)
error = np.arccos(np.clip((decoded*original).sum(axis=2),-1.,1.))
assert np.isfinite(error).all() and error.max() < .017, float(error.max())
assert np.array_equal(encoded[:, :, 2], raw[:, :, 3])
Image.fromarray(encoded).save(output/'kitchen-surface-normal.webp',lossless=True,method=6)
print('Surface atlases packed; maximum normal error in degrees:',float(np.degrees(error.max())))
