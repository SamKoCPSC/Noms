"""Prepare the approved bedroom poster atlas and reduced-motion still."""
from pathlib import Path
import sys
from PIL import Image

source, exported, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
image = Image.open(exported / 'bedroom-texture-1.png').convert('RGB')
image.thumbnail((1536, 1536), Image.Resampling.LANCZOS)
image.save(output / 'bedroom-posters.webp', quality=90, method=6)
image = Image.open(source / 'bedroom-hero-clean.png').convert('RGB')
image.thumbnail((960, 960), Image.Resampling.LANCZOS)
image.save(output / 'sams-bedroom.webp', quality=90, method=6)
