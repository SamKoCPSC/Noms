#!/usr/bin/env python3
"""Check captured browser avatar bytes against the editor's transform-then-crop contract.

Requires the existing ImageMagick CLI, not Pillow or a browser driver. This only
reads explicit local image/metadata paths; capture the upload before server
execution and fail the held request so the saved profile is not modified.
"""

import argparse
import json
import math
import subprocess
from pathlib import Path


def decode_pixels(path):
    dimensions = subprocess.check_output(
        ["magick", "identify", "-format", "%w %h", str(path)], text=True
    )
    width, height = map(int, dimensions.split())
    pixels = subprocess.check_output(
        ["magick", str(path), "-depth", "8", "RGBA:-"]
    )
    assert len(pixels) == width * height * 4, "Unexpected decoded image size"
    return width, height, pixels


def pixel(image, x, y):
    width, _, pixels = image
    offset = (y * width + x) * 4
    return tuple(pixels[offset : offset + 4])


def source_position(edit, image_size, output_position):
    """Invert CSS's centred full-image transform in the fixed crop viewport."""
    width, height = image_size
    crop = edit["crop"]
    transform = edit["transform"]
    angle = math.radians(transform["rotation"])
    cosine, sine = math.cos(angle), math.sin(angle)
    stretch_x, stretch_y = transform["stretch_x"], transform["stretch_y"]
    bounds_width = width * abs(stretch_x * cosine) + height * abs(stretch_y * sine)
    bounds_height = width * abs(stretch_x * sine) + height * abs(stretch_y * cosine)
    fit = max(0.1, min(1.0, width / bounds_width, height / bounds_height))
    scale_x = stretch_x * fit * (-1 if transform["flip_x"] else 1)
    scale_y = stretch_y * fit * (-1 if transform["flip_y"] else 1)
    x = (crop["x"] + output_position[0] * crop["width"]) * width - width / 2
    y = (crop["y"] + output_position[1] * crop["height"]) * height - height / 2
    return (
        (cosine * x + sine * y) / scale_x + width / 2,
        (-sine * x + cosine * y) / scale_y + height / 2,
    )


def adjusted_color(rgb, color):
    channels = [
        max(0, min(255, math.floor(channel * color[key] + 0.5)))
        for channel, key in zip(rgb, ["contrast_r", "contrast_g", "contrast_b"])
    ]
    luminance = sum(a * b for a, b in zip(channels, [0.2126, 0.7152, 0.0722]))
    return tuple(
        max(0, min(255, math.floor(luminance + (channel - luminance) * color["saturation"] + 0.5)))
        for channel in channels
    )


def check_variant(source, output_path, edit, expected_size):
    output = decode_pixels(output_path)
    width, height, _ = output
    assert (width, height) == (expected_size, expected_size), "Wrong variant dimensions"
    for x, y in [(0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1)]:
        assert pixel(output, x, y)[3] == 0, "Circular variant corners must be transparent"
    checked, maximum_error = 0, 0
    # Four output pixels of separation avoid thumbnail chroma/resampling halos.
    margin = max(12, math.ceil(4 * max(
        source[0] * edit["crop"]["width"], source[1] * edit["crop"]["height"]
    ) / width))
    fractions = [0.20 + index * 0.03 for index in range(21)]
    for x_fraction in fractions:
        for y_fraction in fractions:
            x, y = int(x_fraction * width), int(y_fraction * height)
            if math.hypot((x + 0.5) / width - 0.5, (y + 0.5) / height - 0.5) > 0.38:
                continue
            source_x, source_y = source_position(
                edit, source[:2], ((x + 0.5) / width, (y + 0.5) / height)
            )
            source_x, source_y = math.floor(source_x), math.floor(source_y)
            if not (margin <= source_x < source[0] - margin and margin <= source_y < source[1] - margin):
                continue
            original = pixel(source, source_x, source_y)
            # Exclude resampled grid/colour boundaries; lossy WebP is not exact.
            if any(
                pixel(source, source_x + dx, source_y + dy) != original
                for dx in range(-margin, margin + 1) for dy in range(-margin, margin + 1)
            ):
                continue
            actual = pixel(output, x, y)
            expected = adjusted_color(original[:3], edit["color"])
            error = max(abs(a - b) for a, b in zip(actual[:3], expected))
            assert actual[3] == 255, f"Unexpected interior transparency at {x},{y}"
            assert error <= 15, f"Pixel {x},{y}: {actual[:3]} versus {expected}; error {error}"
            maximum_error = max(maximum_error, error)
            checked += 1
    assert checked >= 8, f"Insufficient unambiguous interior samples: {checked}"
    print(f"PASS {output_path}: {width}x{height}, {checked} samples, max RGB error {maximum_error}, transparent corners")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("edit", type=Path)
    parser.add_argument("full", type=Path)
    parser.add_argument("thumbnail", type=Path)
    arguments = parser.parse_args()
    source = decode_pixels(arguments.source)
    edit = json.loads(arguments.edit.read_text())
    assert edit["crop_shape"] == "ellipse", "This checker is for circular avatars"
    check_variant(source, arguments.full, edit, 400)
    check_variant(source, arguments.thumbnail, edit, 100)


if __name__ == "__main__":
    main()
