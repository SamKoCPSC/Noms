"""Publish Blender review renders and matching reading-mode WebP posters.

python assets/models/about-penguin-plushes/source/publish_previews.py /tmp/plush-review
Requires Pillow. Run render_review.py for every scene and library-gallery first.
"""
from pathlib import Path
import json
import shutil
import sys
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[2]
source = Path(sys.argv[1])
rows = json.loads((ROOT / 'placements.json').read_text())['models']
output = ROOT / 'previews'
output.mkdir(exist_ok=True)
font_path = Path('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf')
font = ImageFont.truetype(str(font_path), 20) if font_path.exists() else ImageFont.load_default()
scenes, details = [], []
for row in rows:
    name = row['model']
    image = Image.open(source / f'{name}.png').convert('RGB')
    assert image.size == (1080, 945), name
    shutil.copy2(source / f'{name}.png', output / f'{name}.png')
    image.save(REPO / f'assets/about/sams-{name}.webp', quality=88, method=6)
    scenes.append((name.replace('-', ' ').title(), image))
    for i, plush in enumerate(row['plushes']):
        filename = f'{name}-plush-{i + 1}.png'
        shutil.copy2(source / filename, output / filename)
        details.append((name.replace('-', ' ').title(), Image.open(source / filename).convert('RGB')))
shutil.copy2(source / 'library-gallery.png', output / 'library-gallery.png')

def sheet(items, columns, width, height, filename):
    canvas = Image.new('RGB', (columns * width, ((len(items) + columns - 1) // columns) * (height + 40)), '#f1ebe3')
    draw = ImageDraw.Draw(canvas)
    for i, (label, image) in enumerate(items):
        image = image.copy()
        image.thumbnail((width, height), Image.Resampling.LANCZOS)
        x, y = (i % columns) * width, (i // columns) * (height + 40)
        canvas.paste(image, (x + (width - image.width) // 2, y + (height - image.height) // 2))
        draw.text((x + 12, y + height + 8), label, fill='#413e38', font=font)
    canvas.save(output / filename, quality=92)

sheet(scenes, 3, 480, 420, 'collection.jpg')
sheet(details, 5, 300, 300, 'plush-details.jpg')
print(f'Published {len(scenes)} posters and {len(details)} plush close-ups')
