"""Create the model's reading-mode still from its approved render.

python tools/about/prepare_model_images.py PREVIEW_PNG OUTPUT_DIR MODEL
"""
import sys
from pathlib import Path

from PIL import Image

source, output = map(Path, sys.argv[1:3])
model = sys.argv[3]
assert model in ("corporate-office", "library", "supermarket")
output.mkdir(parents=True, exist_ok=True)
with Image.open(source) as image:
    image = image.convert("RGB")
    image.thumbnail((1080, 960), Image.Resampling.LANCZOS)
    image.save(output / f"sams-{model}.webp", quality=90)
