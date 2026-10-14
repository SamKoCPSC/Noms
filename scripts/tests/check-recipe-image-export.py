#!/usr/bin/env python3
"""Read-only pixel checks for captured recipe WebP variants.

Requires ImageMagick. Recipes resize the original, retaining aspect ratio and
keeping crop/transform/colour edits as separate metadata rather than baking
them into upload bytes. Use a flat-colour synthetic source for sampling; capture
and fail the upload before server execution so no saved content is modified.
"""

import argparse
import math
import subprocess
from pathlib import Path


def decode_pixels(path):
    dimensions = subprocess.check_output(
        ["magick", "identify", "-format", "%w %h", str(path)], text=True
    )
    width, height = map(int, dimensions.split())
    data = subprocess.check_output(["magick", str(path), "-depth", "8", "RGBA:-"])
    assert len(data) == width * height * 4, "Unexpected decoded image size"
    return width, height, data


def pixel(image, x, y):
    offset = (y * image[0] + x) * 4
    return tuple(image[2][offset : offset + 4])


def check_variant(source, path, maximum_dimension):
    # Match integer truncation, preserve aspect ratio, and never upscale.
    factor = min(1, maximum_dimension / max(source[:2]))
    if factor == 1:
        expected_size = source[:2]
    else:
        ratio = source[0] / source[1]
        expected_size = (
            (maximum_dimension, int(maximum_dimension / ratio))
            if source[0] > source[1]
            else (int(maximum_dimension * ratio), maximum_dimension)
        )
    output = decode_pixels(path)
    assert output[:2] == expected_size, f"{output[:2]} versus {expected_size}"
    header = Path(path).read_bytes()[:12]
    assert header[:4] == b"RIFF", "Missing RIFF container"
    assert header[8:12] == b"WEBP", "Missing WebP signature"
    margin = max(12, math.ceil(4 / factor))
    checked, maximum_error, maximum_alpha_error = 0, 0, 0
    for x_fraction in [0.15 + index * 0.05 for index in range(15)]:
        for y_fraction in [0.15 + index * 0.05 for index in range(15)]:
            x, y = int(x_fraction * output[0]), int(y_fraction * output[1])
            source_x = int((x + 0.5) * source[0] / output[0])
            source_y = int((y + 0.5) * source[1] / output[1])
            if not (margin <= source_x < source[0] - margin and margin <= source_y < source[1] - margin):
                continue
            expected = pixel(source, source_x, source_y)
            # Flat interior samples avoid resampling/JPEG/WebP boundary halos.
            if any(
                pixel(source, source_x + dx, source_y + dy) != expected
                for dx in range(-margin, margin + 1) for dy in range(-margin, margin + 1)
            ):
                continue
            actual = pixel(output, x, y)
            alpha_error = abs(actual[3] - expected[3])
            assert alpha_error <= 1, f"Alpha mismatch at {x},{y}: {actual[3]} versus {expected[3]}"
            error = max(abs(a - b) for a, b in zip(actual[:3], expected[:3])) if expected[3] else 0
            assert error <= 15, f"Pixel {x},{y}: {actual} versus {expected}; RGB error {error}"
            maximum_error = max(maximum_error, error)
            maximum_alpha_error = max(maximum_alpha_error, alpha_error)
            checked += 1
    assert checked >= 8, f"Insufficient unambiguous samples: {checked}"
    print(f"PASS {path}: {output[0]}x{output[1]}, {checked} samples, max RGB error {maximum_error}, max alpha error {maximum_alpha_error}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("full", type=Path)
    parser.add_argument("thumbnail", type=Path)
    arguments = parser.parse_args()
    source = decode_pixels(arguments.source)
    check_variant(source, arguments.full, 1920)
    check_variant(source, arguments.thumbnail, 800)


if __name__ == "__main__":
    main()
