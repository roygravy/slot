"""The NES and SNES console bands, cut from the Game Boy slot's own housing (same bevels, grain).

NES: a 256 wide lid between the two housing pieces, 2px seams; the closed lid is baked into the back
strip and also saved on its own, so the firmware can swing it open. SNES: one plain band with a
groove cut down its front each side, just outside the seated NTSC cart.
Usage: tv_slots.py ASSETS_DIR
"""
import sys
from PIL import Image, ImageDraw

A = sys.argv[1].rstrip('/') + '/'
W, H, C = 720, 58, 360
EDGE = 7
L_END, R_START = 246, 474          # the Game Boy mouth: the left piece ends, the right one starts
MW, GAP = 256, 2
MX = C - MW // 2


def plain(front, w):
    """A run of plain housing w wide, tiled from the left piece away from its edges."""
    src = front.crop((40, 0, 220, H))
    out = Image.new('RGBA', (w, H))
    x = 0
    while x < w:
        out.paste(src, (x, 0))
        x += src.width
    return out


def nes_blocks(im, x0=MX - 82, x1=MX - 6):
    d = ImageDraw.Draw(im)
    d.line((x0 - 2, 0, x0 - 2, H), fill=(108, 107, 104, 255))
    d.line((x0 - 1, 0, x0 - 1, H), fill=(140, 138, 134, 255))
    for y0, y1 in ((1, 28), (29, H - 2)):
        d.rectangle((x0, y0, x1, y1), fill=(38, 38, 40, 255))
        d.line((x0, y0, x1, y0), fill=(78, 78, 80, 255))
        d.line((x0, y0, x0, y1), fill=(60, 60, 62, 255))
        d.line((x0, y1, x1, y1), fill=(18, 18, 20, 255))
        d.line((x1, y0, x1, y1), fill=(24, 24, 26, 255))


def strips(name):
    return (Image.open(f'{A}slot_front_gb_{name}.png').convert('RGBA'),
            Image.open(f'{A}slot_back_gb_{name}.png').convert('RGBA'))


# Each theme's NES and SNES bands, cut from that theme's Game Boy housing; Originals has one
# housing for each console (tools/slot_art.py).
for theme, nes_from, snes_from in [('dark', 'dark', 'dark'), ('light', 'light', 'light'),
                                   ('orig', 'orig_nes', 'orig_snes')]:
    gb, back_gb = strips(nes_from)
    left_edge = gb.crop((L_END - EDGE, 0, L_END, H))       # a piece's right-hand edge
    right_edge = gb.crop((R_START, 0, R_START + EDGE, H))  # a piece's left-hand edge
    base = gb.getpixel((120, 30))[:3]

    # ---- NES ----
    front = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    lx = MX - GAP
    front.paste(gb.crop((0, 0, lx - EDGE, H)), (0, 0))
    front.paste(left_edge, (lx - EDGE, 0))
    rx = MX + MW + GAP
    front.paste(right_edge, (rx, 0))
    front.paste(gb.crop((R_START + EDGE, 0, R_START + EDGE + W - rx - EDGE, H)), (rx + EDGE, 0))
    lid = plain(front, MW)
    lid.paste(right_edge, (0, 0))
    lid.paste(left_edge, (MW - EDGE, 0))
    seam = back_gb.getpixel((C, 30))
    back = Image.new('RGBA', (W, H), seam)
    back.alpha_composite(front)
    back.alpha_composite(lid, (MX, 0))
    if theme == 'orig':
        # The real NES's top beside its lid: the two black blocks of the ribbed strip, as tall
        # together as the lid (Roy's top-view photo); the ribs run on below, out of the band.
        for part in (front, back):
            nes_blocks(part)
    front.save(f'{A}slot_front_nes_{theme}.png', optimize=True)
    back.save(f'{A}slot_back_nes_{theme}.png', optimize=True)
    lid.save(f'{A}nes_lid_{theme}.png', optimize=True)

    # ---- SNES ----
    gb, back_gb = strips(snes_from)
    left_edge = gb.crop((L_END - EDGE, 0, L_END, H))
    right_edge = gb.crop((R_START, 0, R_START + EDGE, H))
    base = gb.getpixel((120, 30))[:3]
    band = gb.copy()
    band.paste(plain(gb, R_START + EDGE - (L_END - EDGE) + 20), (L_END - EDGE - 10, 0))
    d = ImageDraw.Draw(band)
    light = sum(base) > 300
    dark = tuple(round(v * (0.55 if light else 0.45)) for v in base)
    lit = tuple(min(255, round(v * 1.12 + 10)) for v in base)
    gx = C - 120 - 2 - 3                                   # just outside the 240 wide cart, 2px play
    for x in (gx, W - gx - 3):
        d.rectangle((x, 0, x + 1, H), fill=dark)
        d.line((x + 2, 0, x + 2, H), fill=lit)
        d.line((x, 0, x + 2, 0), fill=dark)
    band.save(f'{A}slot_front_snes_{theme}.png', optimize=True)
    band.save(f'{A}slot_back_snes_{theme}.png', optimize=True)

# The NES and SNES greys' Game Boy housings were only the stuff the bands are cut from.
import os
for name in ('orig_nes', 'orig_snes'):
    for part in ('front', 'back'):
        os.remove(f'{A}slot_{part}_gb_{name}.png')
