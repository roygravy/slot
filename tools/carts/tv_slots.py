"""The NES and SNES console bands, cut from the Game Boy slot's own housing (same bevels, grain).

NES: a 256 wide lid between the two housing pieces, 2px seams; the closed lid is baked into the back
strip and also saved on its own, so the firmware can swing it open. SNES: one plain band with a
groove cut down its front each side, just outside the seated NTSC cart.
Usage: tv_slots.py ASSETS_DIR
"""
import sys
import numpy as np
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


DECK = (150, 149, 153)              # the SNES PAL's darker top deck, around its cartridge slot
DECK_R, DECK_B, SS = 16, 33, 4
# the corners' arcs start right where the grooves come out from under the deck
DECK_X = (C - 120 - 2 - 3 - DECK_R, C + 120 + 2 + 3 + DECK_R)


def pal_deck(band):
    """The SNES PAL's top (Roy, Originals): the dark deck in the middle, rounded where it meets
    the light front, a shadow where the deck falls away under the front's edge and a lit rim on
    that edge. The grain stays; the grooves are cut afterwards, on the front only."""
    ys, xs = np.mgrid[0:H * SS, 0:W * SS].astype(np.float32)
    y, x = (ys + 0.5) / SS, (xs + 0.5) / SS
    x0, x1 = DECK_X
    m = (x >= x0) & (x < x1) & (y < DECK_B)
    for cx in (x0 + DECK_R, x1 - DECK_R):
        side = (x < cx) if cx == x0 + DECK_R else (x > cx)
        m &= ~(side & (y > DECK_B - DECK_R) & ((x - cx) ** 2 + (y - DECK_B + DECK_R) ** 2 > DECK_R ** 2))
    e = m.astype(np.float32)

    def shifted(dy, dx, edge=1.0):
        r = np.roll(e, (dy, dx), (0, 1))
        if dy > 0: r[:dy] = edge
        if dx > 0: r[:, :dx] = edge
        if dx < 0: r[:, dx:] = edge
        return r

    def down(a):
        return a.reshape(H, SS, W, SS).mean((1, 3))

    shadow = np.zeros_like(e)
    for k in range(1, 3 * SS):
        f = 1 - k / (3 * SS)
        shadow = np.maximum(shadow, e * (1 - shifted(k, 0)) * f)
        shadow = np.maximum(shadow, e * (1 - shifted(0, k)) * f * 0.6)
        shadow = np.maximum(shadow, e * (1 - shifted(0, -k)) * f * 0.6)
    rim = np.zeros_like(e)
    for k in range(SS, 2 * SS):
        rim = np.maximum(rim, (1 - e) * shifted(k, 0, 0.0))
        rim = np.maximum(rim, (1 - e) * shifted(0, k, 0.0) * 0.6)
        rim = np.maximum(rim, (1 - e) * shifted(0, -k, 0.0) * 0.6)
    a = np.asarray(band, np.float32)
    rgb = a[..., :3]
    body = np.median(rgb[20:50, 40:120].reshape(-1, 3), 0)
    md = down(e)[..., None]
    # the deck takes the housing's grain, and only the top two rows of its lit edge
    row = np.median(rgb[:, 40:120], 1)[:, None]
    tone = rgb / row
    tone[:2] *= row[:2] / body
    deck = np.array(DECK, np.float32) * tone
    rgb = rgb * (1 - md) + deck * md
    rgb = rgb * (1 - 0.45 * down(shadow)[..., None]) + down(rim)[..., None] * 28
    a[..., :3] = rgb.clip(0, 255)
    return Image.fromarray(a.astype(np.uint8), 'RGBA')


def grooves(band, top=0):
    """A groove cut down the band each side, just outside the 240 wide seated cart (2px play),
    shaded from the plastic beside it. Originals starts it below the PAL deck (`top`)."""
    a = np.asarray(band, np.float32)
    gx = C - 120 - 2 - 3
    for x in (gx, W - gx - 3):
        beside = a[top:, x - 3, :3]
        light = beside.sum(1, keepdims=True) > 300
        a[top:, x:x + 2, :3] = (beside * np.where(light, 0.55, 0.45))[:, None]
        a[top:, x + 2, :3] = np.minimum(255, beside * 1.12 + 10)
        if top == 0:
            a[0, x:x + 3, :3] = a[0, x - 3, :3] * np.where(light[0], 0.55, 0.45)
    return Image.fromarray(a.astype(np.uint8), 'RGBA')


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
    if theme == 'orig':
        band = pal_deck(band)
    band = grooves(band, DECK_B if theme == 'orig' else 0)
    band.save(f'{A}slot_front_snes_{theme}.png', optimize=True)
    band.save(f'{A}slot_back_snes_{theme}.png', optimize=True)

# The NES and SNES greys' Game Boy housings were only the stuff the bands are cut from.
import os
for name in ('orig_nes', 'orig_snes'):
    for part in ('front', 'back'):
        os.remove(f'{A}slot_{part}_gb_{name}.png')
