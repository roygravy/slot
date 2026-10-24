"""The Super Game Boy's top for the band, from renders made larger than the shelf's carts
(Roy: the zoom made the SGB soft). Tints the neutral renders as tools/sgb/carts/colour2.py does
the finished faces, then hands each to sgb_band.py's layout, zooming down instead of up.

usage: sgb_band_hd.py RENDERS F
RENDERS holds sgb_render.py / sgb_pal_render.py output made with SGB_F=F (and SGB2's with its
extra argument). Writes assets/sgb_band_<model>_{back,front}.png."""
import sys
import numpy as np
from PIL import Image

R, F = sys.argv[1].rstrip('/') + '/', float(sys.argv[2])
W, TOP = 720, 6
H = 87
HOLE = (246, 474)

def overlay(b, c):
    b, c = b / 255, c / 255
    return np.where(b < 0.5, 2 * b * c, 1 - 2 * (1 - b) * (1 - c)) * 255

def face(neutral, mouth, colour, clear=False):
    n = np.asarray(Image.open(R + neutral)).astype(np.float32)
    a = n.copy()
    for k in range(3):
        a[..., k] = overlay(n[..., 0], colour[k])
    mo = np.load(R + mouth)
    rows = np.cumsum(mo > 0.5, axis=0)
    shade = np.clip(1 - 0.35 * np.exp(-rows / (4.0 * F)), 0, 1)
    depth = rows / np.maximum(rows.max(axis=0, keepdims=True), 1)
    sheen = 1 + 0.18 * np.clip(depth, 0, 1) ** 2
    for k, v in enumerate((44, 44, 48)):
        a[..., k] = a[..., k] * (1 - mo) + v * shade * sheen * mo
    above = (np.roll(mo, -1, axis=0) > 0.3) & (mo < 0.3)
    a[..., :3] = np.where(above[..., None], np.clip(a[..., :3] * 1.12, 0, 255), a[..., :3])
    if clear:
        a[..., 3] *= 0.72
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), 'RGBA'), mo > 0.5

def band(name, im, mouth):
    ys, xs = np.where(mouth)
    zoom = (HOLE[1] - HOLE[0]) / (xs.max() + 1 - xs.min())
    fw, fh = round(im.width * zoom), round(im.height * zoom)
    big = im.resize((fw, fh), Image.Resampling.LANCZOS)
    mbig = np.asarray(Image.fromarray(mouth.astype(np.uint8) * 255).resize((fw, fh), Image.Resampling.BILINEAR)) > 127
    cx = (xs.min() + xs.max() + 1) / 2 * zoom
    ox = round(W / 2 - cx)
    back = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    back.alpha_composite(big, (ox, TOP))
    front = np.asarray(back).copy()
    for x in np.where(mbig.any(axis=0))[0]:
        if 0 <= x + ox < W:
            front[:, x + ox, 3] = 0
    back.save(f'assets/sgb_band_{name}_back.png', optimize=True)
    Image.fromarray(front).save(f'assets/sgb_band_{name}_front.png', optimize=True)
    print(f'{name}: zoom {zoom:.5f} x {ox} y {TOP} w {fw} h {fh}')

band('ntsc', *face('sgb_neutral.png', 'sgb_mouth.npy', (134, 134, 138)))
pal = face('sgb_pal_neutral.png', 'sgb_pal_mouth.npy', (198, 198, 201))
band('pal', *pal)
band('sfc', *pal)
band('sgb2', *face('sgb2_clear.png', 'sgb2_mouth.npy', (40, 150, 225), clear=True))
