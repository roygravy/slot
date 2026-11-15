"""The Super Game Boy as the camera closes in on it: the shelf face at the band's size, its top
the band's own art, so the zoom stays as sharp as the slots and lands on the band without a
change (Roy, xh2).

usage: sgb_zoom_face.py
Reads assets/sgb_face_*.png and assets/sgb_band_*_{back,front}.png; writes
assets/sgb_zoom_{ntsc,pal,sfc,sgb2}.png."""
import numpy as np
from PIL import Image, ImageFilter

A = 'assets/'
W, H, TOP = 720, 87, 6
# as slot_chrome.rs draws the Super Game Boy's slot over the band: dim, and the rim's shade
MOUTH_W, DIM, SHADE, SHADE_A, STEPS = 244, 0.22, 14.0, 0.7, 8
# sgb_in_band in slot_chrome.rs: the face's left edge and size in the band's art
PLACE = {'ntsc': (89, 543, 475), 'pal': (123, 472, 448), 'sfc': (123, 472, 448), 'sgb2': (123, 472, 448)}
FEATHER = 3


def over(dst, src):
    sa, da = src[..., 3:] / 255, dst[..., 3:] / 255
    out = sa + da * (1 - sa)
    rgb = (src[..., :3] * sa + dst[..., :3] * da * (1 - sa)) / np.maximum(out, 1e-6)
    return np.concatenate([rgb, out * 255], axis=2)


def band_as_drawn(name):
    """The band's back, its slot dimmed and shaded, its front over it: as the screen shows it."""
    back = np.asarray(Image.open(f'{A}sgb_band_{name}_back.png').convert('RGBA')).astype(np.float64)
    front = np.asarray(Image.open(f'{A}sgb_band_{name}_front.png').convert('RGBA')).astype(np.float64)
    x0 = (W - MOUTH_W) // 2
    black = np.zeros((H, W, 4))
    black[TOP:, x0:x0 + MOUTH_W, 3] = DIM * 255
    a = over(back, black)
    step = SHADE / STEPS
    for k in range(STEPS):
        t = (STEPS - k) / STEPS
        s = np.zeros((H, W, 4))
        rows = TOP + step * (k + 1)
        full = int(rows)
        s[TOP:TOP + full, x0:x0 + MOUTH_W, 3] = SHADE_A * t * t / STEPS * 2.2 * 255
        if TOP + full < H:
            s[TOP + full, x0:x0 + MOUTH_W, 3] = SHADE_A * t * t / STEPS * 2.2 * 255 * (rows - int(rows))
        a = over(a, s)
    return over(a, front)


def zoom_face(name):
    x, w, h = PLACE[name]
    face = Image.open(f'{A}sgb_face_{name}.png').convert('RGBA')
    big = face.resize((w, h), Image.Resampling.LANCZOS).filter(ImageFilter.UnsharpMask(1.2, 60, 2))
    out = np.asarray(big).astype(np.float64)
    top = band_as_drawn(name)[TOP:, x:x + w]
    n = top.shape[0]
    # the band's art over the face's top, its last rows easing into the face below
    k = np.ones((n, 1, 1))
    k[n - FEATHER:, 0, 0] = np.linspace(1, 0, FEATHER + 2)[1:-1]
    out[:n] = top * k + out[:n] * (1 - k)
    Image.fromarray(np.clip(np.rint(out), 0, 255).astype(np.uint8), 'RGBA').save(f'{A}sgb_zoom_{name}.png', optimize=True)
    print(name, w, h)


for m in PLACE:
    zoom_face(m)
