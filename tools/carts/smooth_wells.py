"""Smooth plastic where the label sits (Roy, st6): a sticker sticks to the mould's polished
label well, not to its grain. Takes the grain out inside each well, edges left as they are, so
bevels stay. On the NES (Roy 2026-11-12) only the label well, the arrow and the grip's ribs down
to the last rib (not the bottom block): the ribs smoothed along their length, so they stay crisp.
Usage: smooth_wells.py ASSET_DIR [NAME...] (rewrites the neutral shells in place; run once)."""
import sys
import numpy as np
import cv2
from PIL import Image, ImageDraw
from scipy import ndimage as nd

# (x0, y0, x1, y1) in asset pixels; the label boxes as in crates/slot-ui/src/cart.rs, a few px
# wider so the well's own bevel is taken along. The Game Boy and Color wells are drawn flat.
WELLS = {
    'gba_neutral.png': [(46, 48, 314, 188)],
    'nes_neutral.png': [],  # its own pass: nes_extra
    'snes_neutral.png': [(66, 0, 294, 108)],
    'snes_pal_neutral.png': [(27, 11, 308, 112)],
}

def smooth(grey):
    """Grain out, shape kept: flat plastic takes a wide blur; where the mould has an edge (a
    bevel, a rib, the arrow) the pixels stay as they are, so its lines stay crisp."""
    g = grey.astype(np.float32)
    calm = nd.gaussian_filter(g, 1.5)
    edge = np.hypot(nd.sobel(calm, 0), nd.sobel(calm, 1))
    keep = nd.maximum_filter(np.clip((edge - 6.0) / 10.0, 0, 1), 3)
    flat = cv2.bilateralFilter(nd.gaussian_filter(g, 2.5), 9, 6.0, 4.0)
    return g * keep + flat * (1 - keep)

# The NES arrow (a triangle under the label) and the grip's ribs, in asset pixels.
NES_ARROW_FACE = [(144, 251.5), (169, 251.5), (156.5, 269.5)]
NES_ARROW_TOP = (144, 248, 170, 252)
NES_RIBS = (60, 0, 111, 300)
# The label well: flat inside x 127..265, y 0..226; its rim lines at x 125..126 and 266..267
# and y 227..229, the moulded screw mark at (157, 193).
NES_WELL = (127, 0, 266, 227)
NES_DOT = (157, 193, 8)

def masked_blur(g, inside, sigma):
    """A blur that only reads pixels inside the mask, so nothing from outside bleeds in."""
    num = nd.gaussian_filter(g * inside, sigma)
    den = nd.gaussian_filter(inside, sigma)
    return np.where(den > 1e-3, num / np.maximum(den, 1e-3), g)

def along(g, band, axis, sigma=4.0):
    num = nd.gaussian_filter1d(g * band, sigma, axis=axis)
    den = nd.gaussian_filter1d(band, sigma, axis=axis)
    return np.where(den > 1e-3, num / np.maximum(den, 1e-3), g)

def nes_well(rgba):
    """The label well smooth right up to its rim, and the rim lines themselves smoothed along
    their run, so the flat reaches the edge cleanly (Roy 2026-11-12: 'niet tot de rand')."""
    h, w = rgba.shape[:2]
    x0, y0, x1, y1 = NES_WELL
    inside = np.zeros((h, w), np.float32)
    inside[y0 + 8:y1, x0:x1] = 1  # the top 8 rows hold the edge's shading: smoothed across
    top = np.zeros((h, w), np.float32)
    top[y0:y0 + 8, x0:x1] = 1
    yy, xx = np.mgrid[0:h, 0:w]
    cx, cy, r = NES_DOT
    dot = (np.hypot(xx - cx, yy - cy) < r).astype(np.float32)
    dot = nd.gaussian_filter(dot, 1.0)
    side = np.zeros((h, w), np.float32)
    side[y0:y1 - 1, x0 - 3:x0] = 1
    side[y0:y1 - 1, x1:x1 + 3] = 1
    bottom = np.zeros((h, w), np.float32)
    bottom[y1 - 1:y1 + 4, x0 + 2:x1 - 2] = 1
    out = rgba.copy()
    for c in range(3):
        g = rgba[..., c]
        flat = masked_blur(g, inside * (1 - (dot > 0.01)), 3.0)
        flat = flat * (1 - dot) + smooth(g) * dot
        v = out[..., c] * (1 - inside) + flat * inside
        v = np.where(top > 0, along(g, top, 1), v)
        v = np.where(side > 0, along(g, side, 0), v)
        v = np.where(bottom > 0, along(g, bottom, 1), v)
        out[..., c] = v
    return out

def along_ribs(grey, x0, x1):
    """The ribs run across: a blur along each row (inside the grip only) takes the grain out and
    leaves every rib's top and bottom edge exactly where it was."""
    g = grey.astype(np.float32)
    inside = np.zeros_like(g)
    inside[:, x0:x1] = 1
    blur = nd.gaussian_filter1d(g * inside, 3.0, axis=1)
    weight = nd.gaussian_filter1d(inside, 3.0, axis=1)
    return np.where(weight > 1e-3, blur / np.maximum(weight, 1e-3), g)

def nes_extra(rgba):
    """Arrow and ribs. The arrow's face is flat right up to its edges: its inside blurred
    within itself, its top edge smoothed along its run, its slanted sides kept crisp."""
    h, w = rgba.shape[:2]
    def poly(pts):
        m = Image.new('L', (w, h), 0)
        ImageDraw.Draw(m).polygon(pts, fill=255)
        return np.asarray(m, np.float32) / 255
    face = poly(NES_ARROW_FACE)
    soft = nd.gaussian_filter(face, 0.7)
    top = np.zeros((h, w), np.float32)
    tx0, ty0, tx1, ty1 = NES_ARROW_TOP
    top[ty0:ty1, tx0:tx1] = 1
    x0, y0, x1, y1 = NES_RIBS
    ribs = np.zeros((h, w), np.float32)
    ribs[y0:y1, x0:x1] = 1
    ribs = nd.gaussian_filter(ribs, 1.5)
    out = rgba.copy()
    for c in range(3):
        g = rgba[..., c]
        flat = masked_blur(g, face, 3.0)
        v = g * (1 - soft) + flat * soft
        v = np.where(top > 0, along(g, top, 1, 3.0), v)
        r = along_ribs(g, x0, x1)
        out[..., c] = v * (1 - ribs) + r * ribs
    return out

only = set(sys.argv[2:])
for name, boxes in WELLS.items():
    if only and name not in only:
        continue
    path = f'{sys.argv[1]}/{name}'
    rgba = np.asarray(Image.open(path).convert('RGBA')).astype(np.float32)
    mask = np.zeros(rgba.shape[:2], np.float32)
    for x0, y0, x1, y1 in boxes:
        mask[y0:y1, x0:x1] = 1
    mask = nd.gaussian_filter(mask, 1.0) * (rgba[..., 3] > 0)
    out = rgba.copy()
    for c in range(3):
        out[..., c] = rgba[..., c] * (1 - mask) + smooth(rgba[..., c]) * mask
    if name == 'nes_neutral.png':
        out[..., :3] = nes_extra(nes_well(out))[..., :3]
    Image.fromarray(np.clip(out + 0.5, 0, 255).astype(np.uint8), 'RGBA').save(path)
    print(name)
