"""The Super Game Boy's top for the band, from renders made larger than the shelf's carts
(Roy: the zoom made the SGB soft). Tints the neutral renders as tools/sgb/carts/colour2.py does
the finished faces, then hands each to sgb_band.py's layout, zooming down instead of up.

usage: sgb_band_hd.py RENDERS F
RENDERS holds sgb_render.py / sgb_pal_render.py output made with SGB_F=F (and SGB2's with its
extra argument). Writes assets/sgb_band_<model>_{back,front}.png."""
import sys
import numpy as np
from PIL import Image, ImageFilter
import os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), 'carts'))
from edges import darken_edges

R, F = sys.argv[1].rstrip('/') + '/', float(sys.argv[2])
W, TOP = 720, 6
H = 87
HOLE = (246, 474)

def overlay(b, c):
    b, c = b / 255, c / 255
    return np.where(b < 0.5, 2 * b * c, 1 - 2 * (1 - b) * (1 - c)) * 255

# the zoomed top close up wants more light and shade than the shelf's small carts (Roy: too flat,
# the shadows unclear): the render's relief pushed away from flat grey, shadows deepened most
CONTRAST, SHADOW = 1.45, 1.25

def punch(g):
    d = (g - 128) * CONTRAST
    return np.clip(128 + np.where(d < 0, d * SHADOW, d), 0, 255)

def tinted(path, colour, alpha=1.0):
    n = darken_edges(np.asarray(Image.open(R + path)).astype(np.float32), round(6 * F))
    a = n.copy()
    g = punch(n[..., 0])
    for k in range(3):
        a[..., k] = overlay(g, colour[k])
    a[..., 3] *= alpha
    return a

def over(dst, src):
    sa, da = src[..., 3:] / 255, dst[..., 3:] / 255
    out = sa + da * (1 - sa)
    rgb = (src[..., :3] * sa + dst[..., :3] * da * (1 - sa)) / np.maximum(out, 1e-6)
    return np.concatenate([rgb, out * 255], axis=2)

def face(neutral, mouth, colour, clear=False):
    if clear:
        # as tools/sgb/carts/colour2.py builds the SGB2's face: the back shell under the clear
        # front, the opaque render's light and shade over both
        a = over(tinted('sgb2_interior.png', [c * 0.85 for c in colour], 0.92), tinted(neutral, colour, 0.55))
        g = punch(darken_edges(np.asarray(Image.open(R + 'sgb2_neutral.png')).astype(np.float32), round(6 * F))[..., 0])
        lift = np.clip((g - 128) / 127, 0, 1)[..., None]
        dark = np.clip((128 - g) / 128, 0, 1)[..., None]
        a[..., :3] = a[..., :3] * (1 - 0.75 * dark) + (255 - a[..., :3]) * 0.7 * lift
    else:
        a = tinted(neutral, colour)
    mo = np.load(R + mouth)
    rows = np.cumsum(mo > 0.5, axis=0)
    shade = np.clip(1 - 0.35 * np.exp(-rows / (4.0 * F)), 0, 1)
    depth = rows / np.maximum(rows.max(axis=0, keepdims=True), 1)
    sheen = 1 + 0.18 * np.clip(depth, 0, 1) ** 2
    for k, v in enumerate((44, 44, 48)):
        a[..., k] = a[..., k] * (1 - mo) + v * shade * sheen * mo
    # the bay's two ridges run on down the black moulding, as on the faces
    u0, u1 = (0.29, 0.71) if mouth == 'sgb_mouth.npy' else (0.262, 0.744)
    xs = np.arange(a.shape[1]) / a.shape[1]
    for f in (0.25, 0.75):
        uc = u0 + f * (u1 - u0)
        on = (np.abs(xs - uc) < 0.004)[None, :] * mo
        beside = ((xs >= uc + 0.004) & (xs < uc + 0.009))[None, :] * mo
        a[..., :3] += on[..., None] * 24 - beside[..., None] * 10
    above = (np.roll(mo, -1, axis=0) > 0.3) & (mo < 0.3)
    a[..., :3] = np.where(above[..., None], np.clip(a[..., :3] * 1.12, 0, 255), a[..., :3])
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), 'RGBA'), mo > 0.5

def band(name, im, mouth):
    ys, xs = np.where(mouth)
    zoom = (HOLE[1] - HOLE[0]) / (xs.max() + 1 - xs.min())
    fw, fh = round(im.width * zoom), round(im.height * zoom)
    big = im.resize((fw, fh), Image.Resampling.LANCZOS).filter(ImageFilter.UnsharpMask(1.2, 60, 2))  # crisp after the zoom down
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
band('sgb2', *face('sgb2_clear.png', 'sgb2_mouth.npy', (28, 146, 162), clear=True))
