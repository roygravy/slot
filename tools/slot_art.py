"""Draw every cartridge slot strip: the Game Boy's and the GBA's, each in a dark and a light
housing, in one moulded-plastic style.

Each housing is a height field built from its outline (the strip under the top of the screen
with the slot's mouth cut out of it, convex corners rounded), lit from above. The Game Boy's
mouth is a snug straight-sided slot running past the bottom of the screen; the GBA's is the
shape of the GBA SP's slot, with a smooth finger scoop and the metal plate showing at the bottom
of its back. Inside the mouth the
back strip is the same plastic in shadow, and the front strip lays the walls' shadow on the cart.

Writes assets/slot_{back,front}_{dark,light}.png (GBA) and
assets/slot_{back,front}_gb_{dark,light}.png (Game Boy) at 720x58, and the Originals ones.
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from scipy.ndimage import distance_transform_edt as edt

ROOT = Path(__file__).resolve().parents[1]
W, H, SS = 720, 58, 4
PAD = 12 * SS                 # rows above the strip, outside the housing: its top edge is an edge
GB_MOUTH = (244, 476)         # a seated Game Boy pak (225 px of art) with a hair to spare each side
CORNER = 3.0                  # convex corners of the outline: crisp, barely rounded
BEVEL = 2.5                   # the slot's moulded edge: a tight radius, as on thin hard plastic
TOP_BEVEL = 7.0               # the housing's top edge rounds over softly
WALL = 2.2                    # the wall's own thickness, seen going down into the slot
INNER = 5.5                   # how deep the inner walls read on the back strip
RIB_W = 6.0                   # the raised ribs on the slot's back wall
RIB_TOP = 9.0
SCOOP = (263.0, 44.5)           # the scoop: where its arc leaves the floor, and its deepest y
METAL_TOP = 33.0                # below this the slot's back is the metal plate, not plastic

# Dark: the charcoal of the original GBA slot. Light: a warm grey toward the DMG's (Roy).
LOOKS = {
    "dark": np.array([42, 42, 45], np.float32),
    "light": np.array([178, 173, 163], np.float32),
}
# Originals: each console's own housing. The GBA's Indigo in its slot; in the Game Boy's, the
# DMG's grey, the Game Boy Color's Grape and, for the NES and SNES bands cut from it
# (tools/carts/tv_slots.py), the NES's and the Super Famicom's light greys.
ORIGINALS = [
    ("", "orig", np.array([84, 74, 152], np.float32)),
    ("_gb", "orig", np.array([196, 192, 183], np.float32)),
    ("_gb", "orig_gbc", np.array([104, 78, 146], np.float32)),
    ("_gb", "orig_nes", np.array([188, 186, 180], np.float32)),
    ("_gb", "orig_snes", np.array([200, 199, 202], np.float32)),
]


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
    """The GBA slot's mouth: a straight-walled band with tight corners where it steps in to
    the finger scoop, whose curve follows the original slot art."""
    ys, xs = np.mgrid[0 : H * SS, 0 : W * SS].astype(np.float32)
    x, y = (xs + 0.5) / SS, (ys + 0.5) / SS
    left, right, band_bottom, r = 233.0, 487.0, 28.5, 3.0
    band = (x >= left) & (x < right) & (y < band_bottom)
    for cx in (left + r, right - r):
        corner = (y > band_bottom - r) & (np.abs(x - cx) > 0) & (
            ((x < left + r) & (cx == left + r)) | ((x > right - r) & (cx == right - r))
        )
        band &= ~(corner & (np.hypot(x - cx, y - (band_bottom - r)) > r))
    # The finger scoop: a bite out of the straight floor, taken by one huge perfect circle
    # whose arc leaves the floor at x=SCOOP[0] (and its mirror) and dips to SCOOP[1].
    xa, y1 = SCOOP
    half, sag = 360.0 - xa, y1 - band_bottom
    radius = (half * half + sag * sag) / (2 * sag)
    scoop = (np.hypot(x - 360.0, y - (y1 - radius)) < radius) & (y >= band_bottom - 0.5)
    return band | scoop


def draw(mouth: np.ndarray, plastic: np.ndarray, front: bool, metal: bool) -> np.ndarray:
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
    # The slot's walls alone (the space above the strip counted as housing), for a crisp
    # edge there, and the top edge on its own, for a soft round-over.
    walls = housing.copy()
    walls[:PAD] = True
    d_wall = edt(walls)[PAD:] / SS
    d_top = np.broadcast_to((np.arange(hs)[:, None] + 0.5) / SS, (hs, ws))

    def quarter(d, r):
        t = np.clip(d / r, 0, 1)
        return np.sqrt(1 - (1 - t) ** 2) * r
    height = np.minimum(quarter(d_wall, BEVEL) + TOP_BEVEL, quarter(d_top, TOP_BEVEL) + BEVEL)
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

    # A thin bright line where the top face meets the edge, as moulded plastic catches light.
    lip = (d_in > 0.2) & (d_in < 1.1)
    shade = np.where(lip, shade * 1.10, shade)
    c = plastic[None, None, :] * shade[..., None]
    # A catch-light is added, not multiplied, so it shows on dark plastic as well as light.
    c += (spec * 16.0)[..., None]
    a = inside.astype(np.float32)

    cols = np.where(mouth.any(axis=0))[0]
    x = (np.arange(ws)[None, :] + 0.5)
    near = (x > cols.min() - (CORNER + 1) * SS) & (x < cols.max() + (CORNER + 1) * SS)
    near = np.repeat(near, hs, axis=0)
    gap = near & ~inside
    # The wall's face: the plastic's thickness going down into the slot, in shade, with the
    # left wall darker than the right one under the light from the left.
    d_out_px = edt(~inside) / SS
    wall = gap & (d_out_px < WALL)
    # How much a point reads as the left side (lit from the left), blended across the middle
    # so the shading has no seam along the slot's floor.
    mid = (cols.min() + cols.max()) / 2
    left_w = np.clip(0.5 - (x - mid) / (80.0 * SS), 0, 1)
    left_w = np.repeat(left_w, hs, axis=0)
    wall_shade = 0.74 - 0.16 * left_w
    wall_shade = np.where(y.repeat(ws, axis=1) < WALL + 1, 0.80, wall_shade)
    if front:
        # The walls' shadow on the cart standing in the slot.
        d_out = edt(~inside) / SS
        cast = 0.50 * np.exp(-(d_out - WALL).clip(0) / 3.5)
        c[gap] = np.array([18, 17, 16], np.float32)
        a[gap] = np.clip(cast[gap], 0, 0.65)
        c[wall] = plastic[None, :] * wall_shade[wall][..., None]
        a[wall] = 1.0
    else:
        # Inside, a real recess: thick inner walls lit from the left, the far wall's top edge
        # catching light, and shadow that deepens toward the walls and down into the slot.
        def smooth(e0, e1, v):
            t = np.clip((v - e0) / (e1 - e0), 0, 1)
            return t * t * (3 - 2 * t)
        yy = np.broadcast_to(y, (hs, ws))
        deep = 0.60 + 0.12 * smooth(3.0, 30.0, yy)
        deep = deep * (0.80 + 0.20 * smooth(INNER, INNER + 8.0, d_out_px))
        rim = yy < 2.5
        deep = np.where(rim, 0.92 - 0.10 * (yy / 2.5), deep)
        side = d_out_px < INNER
        t = d_out_px / INNER
        side_shade = left_w * (0.46 + 0.12 * t) + (1 - left_w) * (0.70 - 0.06 * t)
        side_shade = np.where(rim, np.maximum(side_shade, 0.85), side_shade)
        deep = np.where(side, side_shade, deep)
        c[gap] = plastic[None, :] * deep[gap][..., None]
        a[gap] = 1.0
        # The wall's lit top edge, a hair inside the housing's line.
        edge = gap & (d_out_px < 0.9)
        c[edge] = plastic[None, :] * (0.92 - 0.26 * left_w)[edge][..., None]
        # Two raised ribs moulded into the slot's back wall, as on the real hardware: long
        # pills with a lit left flank, a shaded right one and a soft shadow beside them.
        xs_ = np.broadcast_to(x, (hs, ws)) / SS
        lo, hi = cols.min() / SS, cols.max() / SS
        # On the GBA SP the ribs stand a little closer together than on the Game Boy.
        at = 0.30 if metal else 0.24
        for cx in (lo + (hi - lo) * at, lo + (hi - lo) * (1 - at)):
            r = RIB_W / 2
            top = RIB_TOP + r
            dy = np.maximum(top - yy, 0)
            d = np.hypot(xs_ - cx, dy)
            rib = gap & (d < r)
            t = np.clip((xs_ - cx) / r, -1, 1)
            rib_shade = np.where(dy > 0, 1.0 - 0.25 * np.clip(-(yy - top) / r, 0, 1), 1.0)
            rib_shade = rib_shade * (1.06 - 0.20 * (t + 1) / 2)
            shadow = gap & ~rib & (xs_ > cx) & (xs_ < cx + r + 2.5) & (yy > RIB_TOP + 1)
            c[shadow] *= 0.90
            c[rib] = plastic[None, :] * (deep[rib] * 1.12 * rib_shade[rib])[..., None]
        if metal:
            # Below the plastic back wall, the metal plate: brushed steel, its top edge catching
            # a line of light under a thin shadow from the plastic above it.
            below = yy - METAL_TOP
            plate = gap & (below >= 0)
            steel = np.array([158, 160, 164], np.float32) if plastic.mean() > 80 else np.array(
                [96, 98, 102], np.float32)
            grain = np.random.default_rng(5).normal(0, 1, (1, ws)).repeat(hs, axis=0) * 0.03
            tone = 0.86 + 0.10 * smooth(0.0, 9.0, below) + grain
            tone = np.where(below < 0.9, 1.18, tone)
            tone = tone * (0.80 + 0.20 * smooth(0.0, 4.0, d_out_px))
            c[plate] = steel[None, :] * tone[plate][..., None]
            seam = gap & (below < 0) & (below > -1.2)
            c[seam] *= 0.70

    c = c.reshape(H, SS, W, SS, 3).mean(axis=(1, 3))
    a = a.reshape(H, SS, W, SS).mean(axis=(1, 3))
    c += mottle(74)[..., None] * (0.6 if plastic.mean() < 80 else 1.0)
    return np.dstack([np.clip(c, 0, 255), a * 255]).round().astype(np.uint8)


MOUTHS = {"": (gba_mouth(), True), "_gb": (gb_mouth(), False)}
STRIPS = [(shape, look, plastic) for shape in MOUTHS for look, plastic in LOOKS.items()]
for shape, look, plastic in STRIPS + ORIGINALS:
    mouth, metal = MOUTHS[shape]
    for part, front in [("back", False), ("front", True)]:
        out = ROOT / "assets" / f"slot_{part}{shape}_{look}.png"
        Image.fromarray(draw(mouth, plastic, front, metal), "RGBA").save(out, optimize=True)
        print("wrote", out.name)
