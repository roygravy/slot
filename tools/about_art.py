"""Draw the About page: the back of an original Game Boy, with the credits moulded into the
plastic as raised letters in the housing's own colour, and a blank paper serial label that slot
fills in at runtime.

Writes assets/about_dark.png and assets/about_light.png at 640x480.
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[1]
FONT = ROOT / "crates" / "slot-ui" / "assets" / "label.ttf"
W, H, SS = 640, 480, 3

# The paper serial label, in screen pixels; crates/slot-ui/src/about.rs prints on it.
LABEL = (200, 34, 440, 92)
PANEL = (60, 124, 580, 408)
# The silver serial sticker inside the recess: smaller, round cornered, stuck on a little
# crooked. crates/slot-ui/src/about.rs prints the serial with the same tilt.
STICKER_INSET = (9, 8)
STICKER_DEG = -0.9

BIG = [
    "slot.  BY BRANDON T. KOWALSKI",
    "WITH EXTRA GRAVY BY ROY GRAVY",
    "MODEL NO.  DMG-01",
    "© 2026  MADE ALL OVER THE WORLD",
]
BIG_PX, BIG_STEP = 19, 28
SMALL = (
    "THE EXQUISITE SKEUOMORPHIC ARTWORK MADE BY MARKUS. EMULATION POWERED BY MGBA, GPSP, "
    "GAMBATTE, FCEUMM AND SNES9X, NTSC FILTER BY BLARGG. UNDERLYING OS IS BASEOS, HOME WI-FI, "
    "LINK AND LABEL SCRAPING BY PVAIBHAV. LABEL SCANS SHARED BY THE COMMUNITY, CHEATS FROM THE "
    "LIBRETRO DATABASE, SCREEN SHADERS AFTER THE LIBRETRO COLLECTION. TYPE IS OPEN SANS, "
    "LIBERATION SANS, PERMANENT MARKER BY FONT DINER AND NERD FONTS SYMBOLS BY RYAN L MCINTYRE."
)
SMALL_PX, SMALL_STEP, SMALL_W = 12, 15, 372

# The plastic, how steep the moulding reads, and how brightly the edges turned to the light
# catch it: on the dark housing the raised letters need that sheen, or they sink into the grey.
LOOKS = {
    "light": (np.array([184, 181, 175], np.float32), 2.6, 0.0),
    "dark": (np.array([62, 62, 67], np.float32), 3.4, 105.0),
}


def font(px):
    return ImageFont.truetype(str(FONT), int(px * SS))


def blur(a, s):
    im = Image.fromarray(np.clip(a * 255, 0, 255).astype(np.uint8))
    return np.asarray(im.filter(ImageFilter.GaussianBlur(s)), np.float32) / 255


def mask(draw_fn):
    im = Image.new("L", (W * SS, H * SS), 0)
    draw_fn(ImageDraw.Draw(im))
    return np.asarray(im, np.float32) / 255


def sc(box):
    return tuple(v * SS for v in box)


def sticker_mask():
    x0, y0, x1, y1 = LABEL
    ix, iy = STICKER_INSET
    im = Image.new("L", (W * SS, H * SS), 0)
    ImageDraw.Draw(im).rounded_rectangle(sc((x0 + ix, y0 + iy, x1 - ix, y1 - iy)),
                                         radius=5 * SS, fill=255)
    cx, cy = (x0 + x1) / 2 * SS, (y0 + y1) / 2 * SS
    im = im.rotate(STICKER_DEG, resample=Image.BICUBIC, center=(cx, cy))
    return np.asarray(im, np.float32) / 255


def metal(yy, xx):
    """Brushed silver foil: a soft vertical falloff, a diagonal sheen and fine grain."""
    x0, y0, x1, y1 = sc(LABEL)
    v = (yy - y0) / (y1 - y0)
    u = (xx - x0) / (x1 - x0)
    base = 196 + 22 * (1 - v) - 10 * v
    sheen = 18 * np.exp(-(((u * 0.8 + v * 0.6) - 0.55) / 0.16) ** 2)
    rng = np.random.default_rng(11)
    grain = rng.normal(0, 1, (yy.shape[0], 1)) * 2.2 + rng.normal(0, 1, yy.shape) * 1.2
    g = base + sheen + grain
    return np.dstack([g * 0.985, g * 0.995, g * 1.01])


def render(plastic, relief, sheen):
    panel = mask(lambda d: d.rounded_rectangle(sc(PANEL), radius=6 * SS, fill=255))
    label = mask(lambda d: d.rounded_rectangle(sc(LABEL), radius=2 * SS, fill=255))
    sticker = sticker_mask()

    def wrap(text, f, width):
        lines, line = [], ""
        for w in text.split():
            t = f"{line} {w}".strip()
            if line and f.getlength(t) > width * SS:
                lines.append(line)
                t = w
            line = t
        return lines + [line]

    def letters(d):
        x = (PANEL[0] + 24) * SS
        y = (PANEL[1] + 20) * SS
        for s in BIG:
            d.text((x, y), s, font=font(BIG_PX), fill=255)
            y += int(BIG_STEP * SS)
        y += 6 * SS
        small = font(SMALL_PX)
        for s in wrap(SMALL, small, SMALL_W):
            d.text((x, y), s, font=small, fill=255)
            y += int(SMALL_STEP * SS)
        # The slot. badge, bottom right in the panel.
        bx1, by1 = PANEL[2] - 20, PANEL[3] - 20
        bx0, by0 = bx1 - 90, by1 - 86
        d.rounded_rectangle(sc((bx0, by0, bx1, by1)), radius=8 * SS, outline=255, width=4 * SS)
        d.rounded_rectangle(sc((bx0 + 10, by0 + 10, bx1 - 10, by1 - 10)), radius=4 * SS,
                            outline=255, width=3 * SS)
        d.text(((bx0 + bx1) * SS // 2, (by0 + by1) * SS // 2), "slot.", font=font(23),
               fill=255, anchor="mm")

    text = mask(letters)
    # The sticker sits on the recess floor, proud of it but not flush with the plastic.
    height = (-blur(panel, 1.2 * SS) - 1.4 * blur(label, 1.0 * SS)
              + 1.3 * blur(text, 0.9 * SS) + 0.9 * blur(sticker, 0.5 * SS))
    gy, gx = np.gradient(height)
    n = np.dstack([-gx * relief, -gy * relief, np.ones_like(height)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    light = np.array([-0.45, -0.7, 0.55])
    light /= np.linalg.norm(light)
    lam = np.clip((n * light).sum(2), 0, 1)
    shade = (0.35 + 0.65 * lam) / (0.35 + 0.65 * light[2])
    yy, xx = np.mgrid[0 : H * SS, 0 : W * SS]
    vignette = 1.04 - 0.10 * (((xx - W * SS / 2) / (W * SS / 2)) ** 2
                              + ((yy - H * SS * 0.45) / (H * SS / 2)) ** 2)
    c = plastic[None, None, :] * (shade * vignette)[..., None]
    c += (sheen * np.clip(lam - light[2], 0, 1) ** 0.8)[..., None]
    # The recess floor sits in shadow, deepest under the sticker's lower right edge.
    floor = blur(label, 0.4 * SS)
    cast = blur(np.roll(np.roll(sticker, int(1.5 * SS), 0), int(1.0 * SS), 1), 1.2 * SS)
    c *= (1 - 0.10 * floor - 0.28 * cast * (1 - sticker))[..., None]
    foil = metal(yy, xx) * np.clip(0.55 + 0.45 * shade, 0.6, 1.15)[..., None]
    c = c * (1 - sticker[..., None]) + foil * sticker[..., None]
    c = c.reshape(H, SS, W, SS, 3).mean((1, 3))
    c += np.random.default_rng(3).normal(0, 1.0, (H, W, 1))
    return Image.fromarray(np.clip(c, 0, 255).astype(np.uint8), "RGB")


for look, (plastic, relief, sheen) in LOOKS.items():
    render(plastic, relief, sheen).save(ROOT / "assets" / f"about_{look}.png", optimize=True)
    print("wrote", f"about_{look}.png")
