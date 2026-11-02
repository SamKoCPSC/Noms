"""Prepare local approved potluck textures and the reading/reduced-motion still."""
from pathlib import Path
import sys
from PIL import Image

source, exported, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
for i, name in enumerate(('window', 'tv', 'pastry', 'chips-salt', 'chips-bbq'), start=1):
    image = Image.open(exported / f'potluck-texture-{i}.png').convert('RGB')
    image.thumbnail((1024, 1024), Image.Resampling.LANCZOS)
    image.save(output / f'potluck-{name}.webp', quality=90, method=6)
image = Image.open(source / 'room-hero-clean.png').convert('RGB')
image.thumbnail((960, 960), Image.Resampling.LANCZOS)
image.save(output / 'sams-potluck.webp', quality=90, method=6)
