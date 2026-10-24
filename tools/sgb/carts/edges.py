"""No light rim round a cart's left, right and bottom ends, as tools/carts/snes_pal_edges.py does
for the PAL game cart: each row (and column) from the outline inward is pulled down to the darkest
of its last few pixels, so the edge and the vent notches don't glow on a dark shelf (Roy)."""
import numpy as np

def darken_edges(im, reach=6):
    """im: float RGBA (H, W, 4), changed in place and returned."""
    def run(lines):
        for line in lines:
            op = np.where(line[:, 3] > 0)[0]
            if len(op) == 0:
                continue
            end = op[-1]; lo = max(op[0], end - reach)
            k = lo + int(np.argmin(line[lo:end + 1, 0]))
            line[k + 1:end + 1, :3] = np.minimum(line[k + 1:end + 1, :3], line[k, :3])
    h, w = im.shape[:2]
    run(im[i, :, :] for i in range(h))        # right edge
    run(im[:, i, :] for i in range(w))        # bottom edge
    run(im[i, ::-1, :] for i in range(h))     # left edge, so it matches the right
    return im
