"""Draw the original Game Boy's power switch for the Game Boy slot, dark and light housing.

The switch is one piece of plastic: a knob on top of the console (a small plate with a low
dome on it, centred on the body's depth, so the band's front rim hides its foot) and the lock
tab that slides out of the slot's right wall over the cart's notch. Both are the same grey,
a few shades darker than the light housing, a dark grey on the dark one (Roy).

Writes assets/power_knob_{dark,light}.png (KNOB_W x KNOB_H, its plate's left edge KNOB_PAD px
in, its bottom row on the band's top edge) and assets/power_tab_{dark,light}.png (TAB_W x
TAB_H, with a soft shadow row under it).
"""
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
K = 8                       # supersampling
LOCK_T = 17                 # the tab's travel, and the knob's and the tab's length
KNOB_PAD = 3                # room left and right of the knob for its shadow
KNOB_H = 11                 # dome top to the band's top edge
TAB_W, TAB_H = LOCK_T, 7    # six rows of tab, one row of shadow under it

THEMES = {
    "light": dict(dome=((198, 198, 196), (158, 158, 156)), plate=((150, 150, 148), (126, 126, 124)),
                  tab=((160, 160, 158), (134, 134, 132)), hi=(206, 206, 204), seam=(104, 104, 102),
                  haze=(230, 230, 224), shadow=0.2),
    "dark": dict(dome=((108, 108, 110), (80, 80, 82)), plate=((68, 68, 70), (54, 54, 56)),
                 tab=((82, 82, 84), (62, 62, 64)), hi=(112, 112, 114), seam=(30, 30, 32),
                 haze=(17, 17, 20), shadow=0.35),
}


def gradient(size, top, bottom, y0, y1):
    g = Image.new("RGB", size)
    d = ImageDraw.Draw(g)
    for y in range(size[1]):
        u = min(1.0, max(0.0, (y / K - y0) / max(1e-6, y1 - y0)))
        d.line([0, y, size[0], y], fill=tuple(int(a + (b - a) * u) for a, b in zip(top, bottom)))
    return g


def knob(t):
    c = THEMES[t]
    w, h = LOCK_T + 2 * KNOB_PAD, KNOB_H
    # Drawn on a taller canvas: the plate goes on 2 px below the band's top edge, where the
    # band's front rim hides it; the bottom of the canvas is cut off at the end.
    full_h = h + 2
    X = lambda v: int(round(v * K))
    size = (w * K, full_h * K)
    base = full_h - 1        # the plate's foot: three rows of plate show over the rim
    bh, sg = 4.0, 5.0        # plate and dome heights
    kl, kr = KNOB_PAD, KNOB_PAD + LOCK_T
    kx = (kl + kr) / 2
    plate = Image.new("L", size, 0)
    ImageDraw.Draw(plate).rounded_rectangle([X(kl), X(base - bh), X(kr), X(base)], radius=int(1.5 * K), fill=255)
    ch = round(LOCK_T * 0.78)
    radius = (ch * ch / 4 + sg * sg) / (2 * sg)
    cy = base - bh + radius - sg
    dome = Image.new("L", size, 0)
    ImageDraw.Draw(dome).ellipse([X(kx - radius), X(cy - radius), X(kx + radius), X(cy + radius)], fill=255)
    ImageDraw.Draw(dome).rectangle([0, X(base - bh) + 1, size[0], size[1]], fill=0)
    shape = ImageChops.lighter(plate, dome)

    rgb = gradient(size, *c["dome"], base - bh - sg, base - bh)
    rgb.paste(gradient(size, *c["plate"], base - bh, base), (0, 0), plate)
    ov = Image.new("RGBA", size, (0, 0, 0, 0))
    d = ImageDraw.Draw(ov)
    r = ch / 2
    d.ellipse([X(kx - r * 0.55), X(base - bh - sg + 0.7), X(kx - r * 0.05), X(base - bh - sg + 2.0)], fill=c["hi"] + (60,))
    d.line([X(kl + 0.5), X(base - bh + 0.5), X(kx - r), X(base - bh + 0.5)], fill=c["hi"] + (150,), width=K)
    d.line([X(kx + r), X(base - bh + 0.5), X(kr - 0.5), X(base - bh + 0.5)], fill=c["hi"] + (150,), width=K)
    ov.putalpha(ImageChops.multiply(ov.split()[3], shape))
    body = Image.new("RGBA", size, (0, 0, 0, 0))
    body.paste(rgb, (0, 0), shape)
    body = Image.alpha_composite(body, ov)
    # A little haze of the wall behind it: it stands back from the band's front.
    hz = Image.blend(body.convert("RGB"), Image.new("RGB", size, c["haze"]), 0.04)
    body = Image.merge("RGBA", (*hz.split(), body.split()[3]))

    # Its faint shadow on the top surface around the foot.
    sh = shape.filter(ImageFilter.GaussianBlur(1.1 * K)).point(lambda v: int(v * c["shadow"]))
    shadow = Image.new("RGBA", size, (0, 0, 0, 0))
    shadow.putalpha(sh)
    out = Image.alpha_composite(shadow, body).resize((w, full_h), Image.LANCZOS)
    return out.crop((0, 0, w, h))


def tab(t):
    c = THEMES[t]
    X = lambda v: int(round(v * K))
    size = (TAB_W * K, TAB_H * K)
    shape = Image.new("L", size, 0)
    ImageDraw.Draw(shape).rounded_rectangle([0, 0, size[0] + K, X(6)], radius=K, fill=255)
    rgb = gradient(size, *c["tab"], 0, 6)
    body = Image.new("RGBA", size, (0, 0, 0, 0))
    body.paste(rgb, (0, 0), shape)
    ov = Image.new("RGBA", size, (0, 0, 0, 0))
    d = ImageDraw.Draw(ov)
    d.line([0, K // 2, size[0], K // 2], fill=c["hi"] + (220,), width=K)
    d.line([K // 2, 0, K // 2, X(6)], fill=c["hi"] + (110,), width=K)
    d.line([0, X(6) - K // 2, size[0], X(6) - K // 2], fill=c["seam"] + (160,), width=K)
    ov.putalpha(ImageChops.multiply(ov.split()[3], shape))
    body = Image.alpha_composite(body, ov)
    sh = Image.new("L", size, 0)
    ImageDraw.Draw(sh).rectangle([X(0.6), X(6), size[0], size[1]], fill=int(255 * 0.35))
    sh = sh.filter(ImageFilter.GaussianBlur(0.6 * K))
    shadow = Image.new("RGBA", size, (0, 0, 0, 0))
    shadow.putalpha(ImageChops.multiply(sh, ImageChops.invert(shape)))
    return Image.alpha_composite(shadow, body).resize((TAB_W, TAB_H), Image.LANCZOS)


if __name__ == "__main__":
    for t in THEMES:
        knob(t).save(ROOT / f"assets/power_knob_{t}.png")
        tab(t).save(ROOT / f"assets/power_tab_{t}.png")
