"""World-space footprints shared by the cottage models, walks and meadow."""

import json
import math
from pathlib import Path

COTTAGES = json.loads(
    (
        Path(__file__).resolve().parents[4]
        / "assets/models/terraced-village/source/wayside-cottages.json"
    ).read_text()
)
LANE_RADIUS = 1.08


def facing(plot):
    x, z = plot["center"]
    bx, bz = plot["bend"]
    length = math.hypot(bx - x, bz - z)
    return (bx - x) / length, (bz - z) / length


def on_plot(plot, x, z):
    nx, nz = facing(plot)
    cx, cz = plot["center"]
    return cx + nz * x + nx * z, cz - nx * x + nz * z


def walk_points(plot):
    # The walk runs a little under the doorstep so it meets the threshold.
    start, control = plot["join"], plot["bend"]
    end = on_plot(plot, 0, plot["depth"] / 2 + 0.12)
    return [
        tuple(
            (1 - t) ** 2 * a + 2 * (1 - t) * t * b + t * t * c for a, b, c in zip(start, control, end)
        )
        for t in (i / 48 for i in range(49))
    ]
