"""Draw the DMG theme's cartridge slot: the original Game Boy's top edge.

The housing is modelled as a height field (a rounded top edge, and a snug slot with soft
rounded walls running down past the bottom of the screen) and lit from above, so its curves
shade the way the moulded plastic does. Its grey is a tint lighter than a Game Boy cart's, and
inside the slot it is the same plastic in shadow. No screws.

Writes assets/slot_back_dmg.png (behind the cart) and assets/slot_front_dmg.png (over it, with
the mouth left open) at 720x58, the size the other slot strips are authored at.
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
W, H, SS = 720, 58, 4
MOUTH = (246, 474)   # snug around a seated Game Boy pak, whose art is 225 px wide
EDGE_R = 10.0        # the radius of the housing's rounded top edge
WALL_R = 7.0         # the radius where the housing turns down into the slot
DEPTH = 7.0          # how far down the slot floor is, in the same units

# A tint lighter than a Game Boy cart's grey plastic.
PLASTIC = np.array([184, 181, 175], np.float32)


def rounded(d, r):
    """A quarter-round profile: 0 at the edge, 1 at r and beyond."""
    t = np.clip(d / r, 0, 1)
    return np.sqrt(1 - (1 - t) ** 2)


def mottle(seed: int) -> np.ndarray:
    """The cart art's speckled plastic: fine grain over soft blotches."""
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


def blur(a: np.ndarray, sigma: float) -> np.ndarray:
    lo, hi = a.min(), a.max()
    img = Image.fromarray(((a - lo) / max(hi - lo, 1e-6) * 255).astype(np.uint8))
    b = np.asarray(img.filter(ImageFilter.GaussianBlur(sigma)), np.float32) / 255
    return b * (hi - lo) + lo


CORNER = 7.0    # the round where the housing's top turns down into the slot
BEVEL = 6.0     # how wide the housing's moulded edge is: crisp, not pillowy


def housing_sdf(x, y):
    """Signed distance to the housing's outline, negative inside: the strip under the top of
    the screen with the slot cut out of it, every corner rounded."""
    def column(qx, qy):
        # A quarter-plane {qx < 0, qy > 0} with its corner rounded by CORNER.
        ax, ay = qx + CORNER, CORNER - qy
        out = np.hypot(np.maximum(ax, 0), np.maximum(ay, 0))
        return out + np.minimum(np.maximum(ax, ay), 0) - CORNER
    left = column(x - MOUTH[0], y)
    right = column(MOUTH[1] - x, y)
    return np.minimum(left, right)


def draw(front: bool, plastic: np.ndarray) -> np.ndarray:
    hs, ws = H * SS, W * SS
    ys, xs = np.mgrid[0:hs, 0:ws].astype(np.float32)
    x, y = (xs + 0.5) / SS, (ys + 0.5) / SS
    d = housing_sdf(x, y)
    inside = d < 0
    into = np.maximum(MOUTH[0] - x, x - MOUTH[1])
    mouth = (into < 0)

    # A rounded edge: height rises over BEVEL px from the outline, as a quarter round.
    depth = np.clip(-d / BEVEL, 0, 1)
    height = np.sqrt(1 - (1 - depth) ** 2) * BEVEL
    gy, gx = np.gradient(height * SS)
    n = np.dstack([-gx, -gy, np.ones_like(height)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    light = np.array([-0.40, -0.70, 0.59], np.float32)
    light /= np.linalg.norm(light)
    lam = np.clip((n * light).sum(axis=2), 0, 1)
    shade = (0.38 + 0.62 * lam) / (0.38 + 0.62 * light[2])
    half = light + np.array([0, 0, 1], np.float32)
    half /= np.linalg.norm(half)
    spec = np.clip((n * half).sum(axis=2), 0, 1) ** 24
    shade += 0.22 * np.clip(spec - half[2] ** 24, 0, None)
    # The moulding's broad face falls away a little toward the bottom.
    shade *= 1.03 - 0.08 * np.clip(y / H, 0, 1)

    c = plastic[None, None, :] * shade[..., None]
    a = inside.astype(np.float32)
    if front:
        # Shadow the housing throws onto the cart: from both walls, strongest in the gap.
        wall_d = np.clip(-into, 0, None)
        cast = 0.58 * np.exp(-wall_d / 5.0)
        near = (x > MOUTH[0] - CORNER - 1) & (x < MOUTH[1] + CORNER + 1)
        dark = near & ~inside
        c[dark] = np.array([24, 22, 20], np.float32)
        a[dark] = np.clip(cast[dark], 0, 0.65)
    else:
        # Behind the cart: the same plastic, deep in the housing's shadow.
        back = ~inside
        deep = 0.76 + 0.10 * np.clip(y / 30.0, 0, 1)
        c[back] = plastic[None, :] * deep[back][..., None]
        # Above the walls' rounded tops there is no slot yet, just what is behind the housing.
        below = np.clip((y - CORNER * 0.6) / (CORNER * 0.6), 0, 1)
        # The notch behind each rounded corner shows the same shadowed slot, fading in the
        # same way, so no sliver of the background shows between corner and slot.
        near = (x > MOUTH[0] - CORNER - 1) & (x < MOUTH[1] + CORNER + 1)
        a[back] = np.where(near[back], 1.0, 0.0)

    c = c.reshape(H, SS, W, SS, 3).mean(axis=(1, 3))
    a = a.reshape(H, SS, W, SS).mean(axis=(1, 3))
    c += mottle(74)[..., None]
    return np.dstack([np.clip(c, 0, 255), a * 255]).round().astype(np.uint8)


# Light: a tint lighter than a Game Boy cart's grey. Dark: the charcoal of the dark theme's
# GBA slot, so both platforms' slots belong to one housing.
LOOKS = {"light": np.array([184, 181, 175], np.float32), "dark": np.array([58, 58, 63], np.float32)}

for look, plastic in LOOKS.items():
    for part, front in [("back", False), ("front", True)]:
        out = ROOT / "assets" / f"slot_{part}_gb_{look}.png"
        Image.fromarray(draw(front, plastic), "RGBA").save(out, optimize=True)
