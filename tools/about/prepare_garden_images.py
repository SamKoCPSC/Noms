"""Prepare the approved garden poster atlas and reduced-motion still."""
from pathlib import Path
import sys
from PIL import Image

source, exported, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
image = Image.open(exported / 'garden-texture-1.png').convert('RGB')
image.thumbnail((1536, 1536), Image.Resampling.LANCZOS)
image.save(output / 'garden-pastry.webp', quality=90, method=6)
image = Image.open(source / 'garden-v6-clean.png').convert('RGB')
image.thumbnail((960, 960), Image.Resampling.LANCZOS)
image.save(output / 'sams-garden.webp', quality=90, method=6)
