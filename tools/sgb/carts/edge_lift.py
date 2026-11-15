"""Brings the Super Game Boy faces' left, right and bottom edges up to the Game Boy pak's: the
darken_edges pass left a 5px dark fade there (Roy: much darker towards the sides than the paks).
Each line from the outline inward is lifted to the plateau just behind the edge, with a crisp
rim like the pak's (first pixel ~55%, then full). Only ever brightens.  Run on assets/sgb_face_*.png."""
import sys
import numpy as np
from PIL import Image

RIM = [0.55, 0.93]
REACH = 6
REACH_BOTTOM = 4

def lift(im):
    h, w = im.shape[:2]
    def run(line):
        op = np.where(line[:, 3] > 0.5 * line[:, 3].max())[0]
        if len(op) == 0:
            return
        s = op[0]
        if s + REACH + 4 >= len(line):
            return
        plateau = np.median(line[s + REACH:s + REACH + 4, :3], axis=0)
        for d in range(REACH):
            f = RIM[d] if d < len(RIM) else 1.0
            line[s + d, :3] = np.maximum(line[s + d, :3], plateau * f)
    for y in range(h):
        run(im[y])                    # left
        run(im[y, ::-1])              # right
    # The bottom rim differed from section to section (side pieces darker than the middle plate,
    # a step where they meet): every column gets the same rim, the lighter sections' (Roy: a hitch
    # in the bottom line).
    a = im[..., 3]
    cols = [x for x in range(w) if a[:, x].max() > 0]
    lo, hi = cols[0] + 8, cols[-1] - 8
    for x in range(lo, hi + 1):
        bot = int(np.where(a[:, x] > 0.5 * a[:, x].max())[0].max())
        for d in range(REACH_BOTTOM):
            ref = np.percentile(im[[int(np.where(a[:, c] > 0.5 * a[:, c].max())[0].max()) - d for c in range(lo, hi + 1)], range(lo, hi + 1), :3], 75, axis=0)
            im[bot - d, x, :3] = ref
    return im

for p in sys.argv[1:]:
    a = np.asarray(Image.open(p).convert('RGBA')).astype(np.float32)
    Image.fromarray(np.clip(lift(a), 0, 255).astype(np.uint8)).save(p)
    print(p)
