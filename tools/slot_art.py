"""Draw every cartridge slot strip: the Game Boy's and the GBA's, each in a dark and a light
housing, in one moulded-plastic style.

Each housing is a height field built from its outline (the strip under the top of the screen
with the slot's mouth cut out of it, convex corners rounded), lit from above. The Game Boy's
mouth is a snug straight-sided slot running past the bottom of the screen; the GBA's is the
shape of its slot with the finger scoop, read from tools/gba_mouth_mask.png. Inside the mouth the
back strip is the same plastic in shadow, and the front strip lays the walls' shadow on the cart.

Writes assets/slot_{back,front}_{dark,light}.png (GBA) and
assets/slot_{back,front}_gb_{dark,light}.png (Game Boy) at 720x58.
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from scipy.ndimage import distance_transform_edt as edt

ROOT = Path(__file__).resolve().parents[1]
W, H, SS = 720, 58, 4
PAD = 12 * SS                 # rows above the strip, outside the housing: its top edge is an edge
GB_MOUTH = (246, 474)         # snug around a seated Game Boy pak, whose art is 225 px wide
CORNER = 7.0                  # convex corners of the outline are rounded this much
BEVEL = 6.0                   # the moulded edge: crisp, not pillowy

# Dark: the charcoal of the original GBA slot. Light: a tint lighter than a Game Boy cart.
LOOKS = {
    "dark": np.array([33, 33, 35], np.float32),
    "light": np.array([184, 181, 175], np.float32),
}


def mottle(seed: int) -> np.ndarray:
    rng = np.random.default_rng(seed)
    out = np.zeros((H, W), np.float32)
    for sigma, amp in [(0.0, 1.5), (0.5, 0.9)]:
        n = rng.normal(0, 1, (H, W)).astype(np.float32)
        if sigma:
            img = Image.fromarray(((n * 40) + 128).clip(0, 255).astype(np.uint8))
            n = (np.asarray(img.filter(ImageFilter.GaussianBlur(sigma)), np.float32) - 128) / 40
            n /= max(n.std(), 1e-3)
        out += n * amp
    return out


def gb_mouth() -> np.ndarray:
    xs = (np.arange(W * SS) + 0.5) / SS
    m = (xs >= GB_MOUTH[0]) & (xs < GB_MOUTH[1])
    return np.repeat(m[None, :], H * SS, axis=0)


def gba_mouth() -> np.ndarray:
    a = np.asarray(Image.open(ROOT / "tools" / "gba_mouth_mask.png").convert("RGBA"))[..., 3]
    big = Image.fromarray(a).resize((W * SS, H * SS), Image.BILINEAR)
    return np.asarray(big) < 128


def draw(mouth: np.ndarray, plastic: np.ndarray, front: bool) -> np.ndarray:
    hs, ws = H * SS, W * SS
    # The housing, with room above it so its top edge is an edge too.
    housing = np.zeros((hs + PAD, ws), bool)
    housing[PAD:] = ~mouth
    # Round its convex corners: shrink it by CORNER, then grow it back.
    r = CORNER * SS
    core = edt(housing) > r
    housing = edt(~core) <= r
    d_in = edt(housing)[PAD:] / SS          # depth into the housing, in px
    inside = housing[PAD:]

    t = np.clip(d_in / BEVEL, 0, 1)
    height = np.sqrt(1 - (1 - t) ** 2) * BEVEL
    gy, gx = np.gradient(height * SS)
    n = np.dstack([-gx, -gy, np.ones_like(height)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    light = np.array([-0.40, -0.70, 0.59], np.float32)
    light /= np.linalg.norm(light)
    lam = np.clip((n * light).sum(axis=2), 0, 1)
    shade = (0.38 + 0.62 * lam) / (0.38 + 0.62 * light[2])
    half = light + np.array([0, 0, 1], np.float32)
    half /= np.linalg.norm(half)
    spec = np.clip(np.clip((n * half).sum(axis=2), 0, 1) ** 24 - half[2] ** 24, 0, None)
    y = (np.arange(hs)[:, None] + 0.5) / SS
    shade = shade * (1.03 - 0.08 * np.clip(y / H, 0, 1))

    c = plastic[None, None, :] * shade[..., None]
    # A catch-light is added, not multiplied, so it shows on dark plastic as well as light.
    c += (spec * 38.0)[..., None]
    a = inside.astype(np.float32)

    cols = np.where(mouth.any(axis=0))[0]
    x = (np.arange(ws)[None, :] + 0.5)
    near = (x > cols.min() - (CORNER + 1) * SS) & (x < cols.max() + (CORNER + 1) * SS)
    near = np.repeat(near, hs, axis=0)
    gap = near & ~inside
    if front:
        # The walls' shadow on the cart standing in the slot.
        d_out = edt(~inside) / SS
        cast = 0.58 * np.exp(-d_out / 5.0)
        c[gap] = np.array([18, 17, 16], np.float32)
        a[gap] = np.clip(cast[gap], 0, 0.65)
    else:
        # Inside: the same plastic in the housing's shadow.
        deep = 0.76 + 0.10 * np.clip(y / 30.0, 0, 1)
        deep = np.broadcast_to(deep, (hs, ws))
        c[gap] = plastic[None, :] * deep[gap][..., None]
        a[gap] = 1.0

    c = c.reshape(H, SS, W, SS, 3).mean(axis=(1, 3))
    a = a.reshape(H, SS, W, SS).mean(axis=(1, 3))
    c += mottle(74)[..., None] * (0.6 if plastic.mean() < 80 else 1.0)
    return np.dstack([np.clip(c, 0, 255), a * 255]).round().astype(np.uint8)


for shape, mouth in [("", gba_mouth()), ("_gb", gb_mouth())]:
    for look, plastic in LOOKS.items():
        for part, front in [("back", False), ("front", True)]:
            out = ROOT / "assets" / f"slot_{part}{shape}_{look}.png"
            Image.fromarray(draw(mouth, plastic, front), "RGBA").save(out, optimize=True)
            print("wrote", out.name)
