"""Create lightweight web prints/poster from the approved revision 13 review assets.
Usage: python tools/about/prepare_kitchen_images.py REVIEW_DIR OUTPUT_DIR
Requires Pillow; original review images are never modified.
"""
from pathlib import Path
import sys
from PIL import Image

source, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
for name, filename, size in [
    ("kitchen-laptop", "laptop-noms-screen.png", 1024),
    ("kitchen-cookbook", "recipe-book-spread.png", 2048),
    ("sams-kitchen", "kitchen-final-clean.png", 960),
]:
    # Power-of-two print sizes allow WebGL 1 mipmaps. Their UV aspect is defined
    # by the actual model surface, so square GPU textures preserve the artwork.
    image = Image.open(source / filename).convert("RGB").resize((size, size))
    image.save(output / f"{name}.webp", quality=88, method=6)
