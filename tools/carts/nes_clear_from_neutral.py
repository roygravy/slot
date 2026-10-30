"""Make the clear NES cart from the grey one: the grey's relief, faint, inside a clean dark rim
kept from the clear cart as it was. Run after the grey is final: nes_clear_from_neutral.py
NEUTRAL CLEAR (CLEAR is read for its rim and written over)."""
import sys
import numpy as np
from PIL import Image
from scipy import ndimage as nd

neutral, clear = sys.argv[1], sys.argv[2]
n = np.asarray(Image.open(neutral)).astype(float)
c = np.asarray(Image.open(clear)).astype(float).copy()
inside = n[..., 3] > 127
rim = inside & ~nd.binary_erosion(inside, iterations=5)
# the same as nes_render.py's clear, and the few levels lighter the clear asset always was
faint = 128 + (n[..., :3] - 128) * 0.45 + 3.9
body = inside & ~rim
c[body, :3] = faint[body]
# where the grey's outline grew (the channel's top squared off), the rim is the straight top
# edge's, from a column it never changed in
grew = rim & (c[..., 3] < n[..., 3])
rows = np.nonzero(grew)[0]
c[grew, :3] = c[rows, 200, :3]
c[..., 3] = n[..., 3]
Image.fromarray(np.clip(c, 0, 255).round().astype(np.uint8), 'RGBA').save(clear)
