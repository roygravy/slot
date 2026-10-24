"""The Super Game Boy faces for the shelf: the neutral renders tinted, the black mouth, the sticker.
The SGB2 is built up like a clear game cart: back shell, board, then the clear front over them.

usage: colour2.py RENDERS [OUT]
RENDERS holds sgb_render.py / sgb_pal_render.py output (SGB_F=1, SGB2's with its extra argument).
Writes OUT/sgb_face_{ntsc,pal,sfc,sgb2}.png (OUT defaults to assets) and a side-by-side preview."""
import numpy as np, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image, ImageFont, ImageDraw, ImageFilter
from scipy import ndimage as nd
from stickers import sticker
R = sys.argv[1].rstrip('/') + '/'
OUT = sys.argv[2] if len(sys.argv) > 2 else 'assets'
def overlay(b, c):
    b = b / 255; c = c / 255
    return np.where(b < 0.5, 2 * b * c, 1 - 2 * (1 - b) * (1 - c)) * 255
def tint(path, rgb, alpha=1.0):
    n = np.asarray(Image.open(R + path)).astype(np.float32)
    o = n.copy()
    for k in range(3):
        o[..., k] = overlay(n[..., 0], rgb[k])
    o[..., 3] *= alpha
    return o
def over(dst, src):
    """src over dst, both float RGBA 0..255."""
    sa, da = src[..., 3:] / 255, dst[..., 3:] / 255
    a = sa + da * (1 - sa)
    rgb = (src[..., :3] * sa + dst[..., :3] * da * (1 - sa)) / np.maximum(a, 1e-6)
    return np.concatenate([rgb, a * 255], axis=2)
BAY = {'sgb': (0.29, 0.71), 'sgb_pal': (0.262, 0.744), 'sgb2': (0.262, 0.744)}  # the renders' bay walls
def mouth(a, stem):
    # the mouth is black plastic whatever the shell's colour: a little lighter than black, its
    # top edge in shadow under the lip of the bay
    mo = np.load(R + stem + '_mouth.npy')
    rows = np.cumsum(mo > 0.5, axis=0)
    shade = np.clip(1 - 0.35 * np.exp(-rows / 4.0), 0, 1)
    # a soft sheen lower down the black moulding, and the grey piece's edge catching the light
    # just above the seam where the two mouldings meet (Roy)
    depth = rows / np.maximum(rows.max(axis=0, keepdims=True), 1)
    sheen = 1 + 0.18 * np.clip(depth, 0, 1) ** 2
    for k, v in enumerate((44, 44, 48)):
        a[..., k] = a[..., k] * (1 - mo) + v * shade * sheen * mo
    # the bay's two ridges run on down the black moulding (Roy's photo of the inside)
    u0, u1 = BAY[stem]
    xs = np.arange(a.shape[1]) / a.shape[1]
    for f in (0.25, 0.75):
        uc = u0 + f * (u1 - u0)
        on = (np.abs(xs - uc) < 0.004)[None, :] * mo
        beside = ((xs >= uc + 0.004) & (xs < uc + 0.009))[None, :] * mo
        a[..., :3] += on[..., None] * 24 - beside[..., None] * 10
    a[..., 3] = np.maximum(a[..., 3], mo * 255)
    above = (np.roll(mo, -1, axis=0) > 0.3) & (mo < 0.3)
    a[..., :3] = np.where(above[..., None], np.clip(a[..., :3] * 1.12, 0, 255), a[..., :3])
    return a
def label(im, stem, style, rect):
    if rect is not None:
        x, y, w, h = rect
        im.alpha_composite(sticker((w, h), style), (x, y))
        return im
    # the sticker fills the recess moulded for it, a hair inside its edge (Roy)
    lm = np.load(R + stem + '_label.npy')
    lm = nd.gaussian_filter(nd.binary_erosion(lm > 0.5, iterations=1).astype(np.float32), 0.5)
    ys, xs = np.where(lm > 0.5); x, y, x1, y1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
    st = sticker((x1 - x, y1 - y), style)
    base = Image.new('RGBA', st.size, st.getpixel((st.width // 2, st.height - 3))[:3] + (255,))
    base.alpha_composite(Image.fromarray(np.where(np.asarray(st)[..., 3:] > 0, np.asarray(st), 0).astype(np.uint8)))
    a = np.asarray(base).copy(); a[..., 3] = (lm[y:y1, x:x1] * 255).astype(np.uint8)
    im.alpha_composite(Image.fromarray(a), (x, y))
    return im
def make(stem, neutral, colour, style, rect, size):
    a = mouth(tint(neutral, colour), stem)
    im = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), 'RGBA')
    return label(im, stem, style, rect).resize(size, Image.Resampling.LANCZOS)
def sgb2(colour, size):
    """Clear plastic over the board, as on Roy's photo: the back shell in the same colour, the
    green board showing through the front's lower half, so the front lets most of it through."""
    back = tint('sgb2_interior.png', [c * 0.85 for c in colour], 0.92)
    # the plastic blurs what lies behind it a little and the board sits in its shade
    board = np.asarray(Image.open(R + 'sgb2_board.png').filter(ImageFilter.GaussianBlur(0.8))).astype(np.float32)
    board[..., :3] *= 0.75
    front = tint('sgb2_clear.png', colour, 0.55)
    a = over(over(back, board), front)
    # clear plastic reads by its edges: the opaque render's light and shade laid over it, so the
    # moulding stays as crisp as on the grey carts
    n = np.asarray(Image.open(R + 'sgb2_neutral.png')).astype(np.float32)[..., 0]
    lift = np.clip((n - 128) / 127, 0, 1)[..., None]
    dark = np.clip((128 - n) / 128, 0, 1)[..., None]
    a[..., :3] = a[..., :3] * (1 - 0.75 * dark) + (255 - a[..., :3]) * 0.7 * lift
    a = mouth(a, 'sgb2')
    im = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), 'RGBA')
    return label(im, 'sgb2', 'sgb2', None).resize(size, Image.Resampling.LANCZOS)
US = make('sgb', 'sgb_neutral.png', (134, 134, 138), 'us', (82, 145, 236, 86), (360, 315))
PAL = make('sgb_pal', 'sgb_pal_neutral.png', (198, 198, 201), 'pal', None, (336, 319))
SFC = make('sgb_pal', 'sgb_pal_neutral.png', (198, 198, 201), 'sfc', None, (336, 319))
B2 = sgb2((28, 146, 162), (336, 319))   # the teal of Roy's photo
for n, im in [('ntsc', US), ('pal', PAL), ('sfc', SFC), ('sgb2', B2)]:
    im.save(f'{OUT}/sgb_face_{n}.png', optimize=True)
F = ImageFont.truetype('/usr/share/fonts/opentype/inter/Inter-SemiBold.otf', 20)
m = Image.new('RGBA', (4 * 370 + 30, 400), (21, 21, 24, 255)); d = ImageDraw.Draw(m)
for i, (t, im) in enumerate([('NTSC (VS)', US), ('PAL', PAL), ('Japans (SFC)', SFC), ('SGB2', B2)]):
    x = 15 + i * 370; d.text((x, 14), t, font=F, fill=(235, 235, 240)); m.alpha_composite(im, (x + (360 - im.width) // 2, 60))
m.save(f'{OUT}/sgb_faces_preview.png')
