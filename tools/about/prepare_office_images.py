"""Prepare the approved office prints and reading-mode poster without editing source images."""
from pathlib import Path
import sys
from PIL import Image

source, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
for name, filename, size in [
    ('office-editor-left', 'textures/editor-left.png', 1024),
    ('office-editor-right', 'textures/editor-right.png', 1024),
    ('office-rtx4090', 'textures/rtx4090-front.png', 1024),
    ('office-intel-core', 'textures/intel-core-front.png', 512),
    ('sams-office', 'office-hero-clean.png', 960),
]:
    image = Image.open(source / filename).convert('RGB').resize((size, size))
    image.save(output / f'{name}.webp', quality=88, method=6)
