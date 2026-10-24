"""Square off the top of the NES cart's channel, level with the body's top edge: each column's
top rows given the straight edge's light and shade over its own colour beneath. The notches
near the corners stay (Roy)."""
import sys
import numpy as np
from PIL import Image
for path in sys.argv[1:]:
    a = np.asarray(Image.open(path)).astype(float).copy()
    al = a[..., 3]
    top = np.array([int(np.argmax(al[:, x] > 128)) if al[:, x].max() > 128 else -1 for x in range(a.shape[1])])
    notch = (top > 1)
    near = np.convolve(notch.astype(int), np.ones(7, int), 'same') > 0
    ref, base = 200, 5
    cols = [x for x in range(54, 117) if near[x]]  # inside the channel walls
    # the first ridge's colour, which runs up to the edge, the same all along so the edge is even
    under = a[base, cols, :3].mean(axis=0)
    for x in cols:
        for r in range(0, base):
            a[r, x, :3] = under + (a[r, ref, :3] - a[base, ref, :3])
            a[r, x, 3] = a[r, ref, 3]
    Image.fromarray(np.clip(a, 0, 255).round().astype(np.uint8), 'RGBA').save(path)
