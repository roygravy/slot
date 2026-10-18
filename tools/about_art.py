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

BIG = [
    "slot.  BY BRANDON T. KOWALSKI",
    "WITH EXTRA GRAVY BY ROY GRAVY",
    "MODEL NO.  DMG-01",
    "© 2026  MADE ALL OVER THE WORLD",
]
SMALL = (
    "THE EXQUISITE SKEUOMORPHIC ARTWORK MADE BY MARKUS. EMULATION POWERED BY MGBA, GPSP "
    "AND GAMBATTE. UNDERLYING OS IS BASEOS BY PVAIBHAV. TYPE IS OPEN SANS, PERMANENT MARKER "
    "BY FONT DINER AND NERD FONTS SYMBOLS BY RYAN L MCINTYRE."
)
SMALL_PX, SMALL_W = 12, 378

LOOKS = {
    "light": np.array([184, 181, 175], np.float32),
    "dark": np.array([62, 62, 67], np.float32),
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


def render(plastic):
    panel = mask(lambda d: d.rounded_rectangle(sc(PANEL), radius=6 * SS, fill=255))
    label = mask(lambda d: d.rounded_rectangle(sc(LABEL), radius=3 * SS, fill=255))

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
            d.text((x, y), s, font=font(20), fill=255)
            y += int(31 * SS)
        y += 4 * SS
        small = font(SMALL_PX)
        for s in wrap(SMALL, small, SMALL_W):
            d.text((x, y), s, font=small, fill=255)
            y += int(16 * SS)
        # The slot. badge, bottom right in the panel.
        bx1, by1 = PANEL[2] - 20, PANEL[3] - 20
        bx0, by0 = bx1 - 90, by1 - 86
        d.rounded_rectangle(sc((bx0, by0, bx1, by1)), radius=8 * SS, outline=255, width=4 * SS)
        d.rounded_rectangle(sc((bx0 + 10, by0 + 10, bx1 - 10, by1 - 10)), radius=4 * SS,
                            outline=255, width=3 * SS)
        d.text(((bx0 + bx1) * SS // 2, (by0 + by1) * SS // 2), "slot.", font=font(23),
               fill=255, anchor="mm")

    text = mask(letters)
    height = -blur(panel, 1.2 * SS) - 1.4 * blur(label, 1.0 * SS) + 1.3 * blur(text, 0.9 * SS)
    gy, gx = np.gradient(height)
    n = np.dstack([-gx * 2.6, -gy * 2.6, np.ones_like(height)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    light = np.array([-0.45, -0.7, 0.55])
    light /= np.linalg.norm(light)
    lam = np.clip((n * light).sum(2), 0, 1)
    shade = (0.35 + 0.65 * lam) / (0.35 + 0.65 * light[2])
    yy, xx = np.mgrid[0 : H * SS, 0 : W * SS]
    vignette = 1.04 - 0.10 * (((xx - W * SS / 2) / (W * SS / 2)) ** 2
                              + ((yy - H * SS * 0.45) / (H * SS / 2)) ** 2)
    c = plastic[None, None, :] * (shade * vignette)[..., None]
    lx0, ly0, lx1, ly1 = sc(LABEL)
    paper = (xx > lx0 + 3 * SS) & (xx < lx1 - 3 * SS) & (yy > ly0 + 3 * SS) & (yy < ly1 - 3 * SS)
    c[paper] = np.array([236, 234, 226], np.float32)
    c = c.reshape(H, SS, W, SS, 3).mean((1, 3))
    c += np.random.default_rng(3).normal(0, 1.0, (H, W, 1))
    return Image.fromarray(np.clip(c, 0, 255).astype(np.uint8), "RGB")


for look, plastic in LOOKS.items():
    render(plastic).save(ROOT / "assets" / f"about_{look}.png", optimize=True)
    print("wrote", f"about_{look}.png")
