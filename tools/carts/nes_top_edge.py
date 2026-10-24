"""Square off the NES cart's top edge: the notches along it filled level with the rest, each
column's top rows given the straight edge's light and shade over its own colour beneath."""
import sys
import numpy as np
from PIL import Image
BASE = 9
for path in sys.argv[1:]:
    a = np.asarray(Image.open(path)).astype(float).copy()
    al = a[..., 3]
    top = np.array([int(np.argmax(al[:, x] > 128)) if al[:, x].max() > 128 else -1 for x in range(a.shape[1])])
    notch = (top > 1)
    near = np.convolve(notch.astype(int), np.ones(7, int), 'same') > 0
    ref = 200
    for x in range(8, 304):
        if not near[x]:
            continue
        # in the channel, from the first ridge, which runs up to the edge
        base = 5 if 50 <= x <= 120 else BASE
        for r in range(0, base):
            a[r, x, :3] = a[base, x, :3] + (a[r, ref, :3] - a[base, ref, :3])
            a[r, x, 3] = a[r, ref, 3]
    Image.fromarray(np.clip(a, 0, 255).round().astype(np.uint8), 'RGBA').save(path)
