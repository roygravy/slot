"""The NES and SNES console bands, cut from the Game Boy slot's own housing (same bevels, grain).

NES: a 256 wide lid between the two housing pieces, 2px seams; the closed lid is baked into the back
strip and also saved on its own, so the firmware can swing it open. SNES: one plain band with a
groove cut down its front each side, just outside the seated NTSC cart.
Usage: tv_slots.py ASSETS_DIR
"""
import sys
import os
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


def nes_blocks(im, x0=MX - 82, x1=MX - GAP - 1, mono=False):
    """The real NES's top beside its lid (Roy's photo, 2026-11-12): two blocks of glossy
    black plastic without grain (on the dark and light themes the housing's own colour), set
    in a well whose left wall is a fine dark line, running right up to the lid. Each one's
    top edge catches the light, a dark gap between them; their corners a little round, but
    for the upper one's top."""
    a = np.asarray(im, np.float32).copy()
    rgb = a[..., :3]
    ys, xs = np.mgrid[0:H * SS, 0:W * SS].astype(np.float32)
    y, x = (ys + 0.5) / SS, (xs + 0.5) / SS
    r = 3.0

    def inside(bx0, by0, bx1, by1, top_round):
        # a rectangle, its lower corners round, its upper ones too if `top_round`
        m = (x >= bx0) & (x < bx1) & (y >= by0) & (y < by1)
        rows = [(by1 - r, y > by1 - r)] + ([(by0 + r, y < by0 + r)] if top_round else [])
        for cy, side in rows:
            for cx, xside in ((bx0 + r, x < bx0 + r), (bx1 - r, x > bx1 - r)):
                m &= ~(side & xside & (np.hypot(x - cx, y - cy) > r))
        return m

    def down(v):
        return v.reshape(H, SS, W, SS).mean((1, 3))

    base = np.array((36, 36, 38), np.float32) if not mono else np.median(rgb[20:50, x0:x1].reshape(-1, 3), 0)
    dark = base.sum() < 300
    lift = 34.0 if dark else 14.0
    housing = rgb.copy()
    # the two blocks equally tall, the gap halfway (Roy 2026-11-12)
    # the well: the gap between the blocks and round their corners is its dark floor
    floor = base * 0.45 if dark else base * 0.55
    out = housing.copy()
    out[:, x0:x1 + 1] = floor
    for k, (by0, by1) in enumerate(((0.0, H / 2 - 1), (H / 2 + 1, H + 4.0))):
        # the upper one square at the top, round below; the lower one round all round (Roy)
        m = down(inside(x0, by0, x1 + 1, by1, k == 1).astype(np.float32))[..., None]
        yy = np.clip(np.arange(H, dtype=np.float32) + 0.5 - by0, 0, None)[:, None, None]
        face = base + lift * np.exp(-yy / 7.0) * (0.7 if k else 1.0)
        # each block's top edge catches the light (Roy 2026-11-12: at the top, not below)
        rows = np.arange(H)[:, None, None]
        y0 = int(np.ceil(by0))
        face = np.where(rows == y0, base + (52.0 if dark else 24.0), face)
        face = np.where(rows == y0 + 1, base + (20.0 if dark else 9.0), face)
        out = out * (1 - m) + np.minimum(255, face) * m
    # the well's left wall: a fine dark line, the housing beside it a shade darker
    out[:, x0 - 1] = housing[:, x0 - 1] * 0.5
    out[:, x0 - 2] = housing[:, x0 - 2] * 0.88
    rgb[:, x0 - 2:x1 + 1] = out[:, x0 - 2:x1 + 1].clip(0, 255)
    a[..., :3] = rgb
    im.paste(Image.fromarray(a.astype(np.uint8), 'RGBA'))


def lid_seams(front, lid, back, lx, rx, r=2.0):
    """The NES lid in its opening, as on Roy's photo (2026-11-12): fine lines, no bevels.
    The gap each side is a dark line; the lid's own left edge catches the light; on its right
    the lid's wall shows a little lighter before the gap. Its top corners and the housing's
    at the gap are a little round, showing the dark behind."""
    f = np.asarray(front, np.float32).copy()
    l = np.asarray(lid, np.float32).copy()
    b = np.asarray(back, np.float32).copy()
    plastic = f[:, lx - 6, :3]
    gap = (plastic * 0.42)[:, None]
    b[:, lx:lx + 2, :3] = gap
    b[:, rx - 2:rx, :3] = gap
    l[:, 0, :3] = np.minimum(255, l[:, 0, :3] * 1.08 + 6)
    l[:, MW - 2:, :3] = np.minimum(255, l[:, MW - 2:, :3] * 1.05 + 4)
    f[:, rx, :3] *= 0.8

    def round_corner(img, col0, step):
        # the top corner whose outermost column is col0, the piece running on by `step`
        for j in range(int(r) + 1):
            for i in range(int(r) + 1):
                dx, dy = max(r - (i + 0.5), 0), max(r - (j + 0.5), 0)
                img[j, col0 + step * i, 3] *= np.clip(r - np.hypot(dx, dy) + 0.5, 0, 1)

    round_corner(f, lx - 1, -1)      # the left housing's top right
    round_corner(l, 0, 1)            # the lid's top left
    round_corner(l, MW - 1, -1)      # the lid's top right
    round_corner(f, rx, 1)           # the right housing's top left
    front.paste(Image.fromarray(f.clip(0, 255).astype(np.uint8), 'RGBA'))
    lid.paste(Image.fromarray(l.clip(0, 255).astype(np.uint8), 'RGBA'))
    back.paste(Image.fromarray(b.clip(0, 255).astype(np.uint8), 'RGBA'))


DECK = (150, 149, 153)              # the SNES PAL's darker top deck, around its cartridge slot
# 6px lower than it was (Roy 2026-11-12)
DECK_R, DECK_B, SS = 16, int(os.environ.get('DECK_B', 27)), 4
# the deck stands proud of the front by this much, its lip showing below it (Roy 2026-11-12:
# the SNES's plastic had no thickness); its shadow falls on the front below
LIP, SHADE = (int(v) for v in os.environ.get('SNES_LIP', '0,0').split(','))
# the band's rounded top edge, drawn this many rows tall instead of the Game Boy's 8: the
# SNES's plastic reads thicker (Roy 2026-11-12); the deck lies flush and rounds over with it
RIM = int(os.environ.get('SNES_RIM', 12))
# the corners' arcs start right where the grooves come out from under the deck
DECK_X = (C - 120 - 2 - 3 - DECK_R, C + 120 + 2 + 3 + DECK_R)


def pal_deck(band, mono=False):
    """The SNES PAL's top (Roy, Originals): the dark deck in the middle, rounded where it meets
    the light front. Both lie flush; the seam between the two plastic parts is drawn like the
    grooves. The grain stays; the grooves are cut afterwards, on the front only."""
    ys, xs = np.mgrid[0:H * SS, 0:W * SS].astype(np.float32)
    y, x = (ys + 0.5) / SS, (xs + 0.5) / SS
    x0, x1 = DECK_X
    m = (x >= x0) & (x < x1) & (y < DECK_B)
    for cx in (x0 + DECK_R, x1 - DECK_R):
        side = (x < cx) if cx == x0 + DECK_R else (x > cx)
        m &= ~(side & (y > DECK_B - DECK_R) & ((x - cx) ** 2 + (y - DECK_B + DECK_R) ** 2 > DECK_R ** 2))
    e = m.astype(np.float32)
    # the deck's lip: the deck pushed down LIP px, what shows below it is its edge
    lip = np.zeros_like(m)
    for k in range(1, LIP * SS + 1):
        lip[k:] |= m[:-k]
    lip &= ~m
    cy = DECK_B - DECK_R + LIP
    cx = np.clip(x, x0 + DECK_R, x1 - DECK_R)
    # how far outside the deck and its lip: the seam is the grooves' line all round
    d = np.where(y > cy, np.hypot(x - cx, y - cy) - DECK_R, np.maximum(x0 - x, x - x1))
    # 2px dark, 1px lit: one pixel finer than before (Roy 2026-11-14), a touch softer (2026-11-15)
    # cut as a real groove (Roy 2026-11-12): the wall nearest the deck in shadow, the floor
    # lighter away from it, then the far wall's lit edge
    dark = (np.where(d <= 1, 0.5, 0.68) * ((d > 0) & (d <= 2))).astype(np.float32)
    dark1 = ((d > 0) & (d <= 2)).astype(np.float32)
    lit = ((d > 2) & (d <= 3)).astype(np.float32)

    def down(a):
        return a.reshape(H, SS, W, SS).mean((1, 3))

    a = np.asarray(band, np.float32)
    rgb = a[..., :3]
    body = np.median(rgb[20:50, 40:120].reshape(-1, 3), 0)
    md = down(e)[..., None]
    # the deck takes the housing's grain, and only the top two rows of its lit edge
    row = np.median(rgb[:, 40:120], 1)[:, None]
    tone = rgb / row
    tone[:2] *= row[:2] / body
    if RIM > 8:
        tone[2:RIM] *= row[2:RIM] / body
    # dark and light themes: the deck takes the housing's own colour, only its seam shows
    deck = (body if mono else np.array(DECK, np.float32)) * tone
    rgb = rgb * (1 - md) + deck * md
    # the lip: the deck's own plastic turned away from the light, darkest at its foot
    ml = down(lip.astype(np.float32))[..., None]
    fall = np.linspace(0, 1, H)[:, None, None]
    yy = np.arange(H)[:, None, None].astype(np.float32)
    foot = np.clip((yy - (DECK_B - 1)) / max(LIP, 1), 0, 1)
    lipc = deck * (0.78 - 0.18 * foot)
    rgb = rgb * (1 - ml) + lipc * ml
    # its soft shadow on the front below: the deck and lip, blurred and dropped a little
    if SHADE:
        from scipy.ndimage import gaussian_filter
        both = down((m | lip).astype(np.float32))
        sh = gaussian_filter(np.roll(both, 1, 0), SHADE / 2.0)
        sh = np.clip(sh - both, 0, 1)[..., None]
        rgb = rgb * (1 - 0.45 * sh)
    dk, lt = down(dark)[..., None], down(lit)[..., None]
    cov = down(dark1)[..., None]
    rgb = rgb * (1 - cov) + rgb * np.where(cov > 0, dk / np.maximum(cov, 1e-6), 1) * cov
    rgb = rgb * (1 - lt) + np.minimum(255, rgb * 1.08 + 6) * lt
    a[..., :3] = rgb.clip(0, 255)
    return Image.fromarray(a.astype(np.uint8), 'RGBA')


def grooves(band, top=0, xs=None):
    """A groove cut down the band each side, just outside the 240 wide seated cart (2px play),
    shaded from the plastic beside it. Originals starts it below the PAL deck (`top`)."""
    a = np.asarray(band, np.float32)
    # 2px dark and 1px lit: one pixel finer than before (Roy 2026-11-14), the inner edges stay
    gx = C - 120 - 2 - 4
    for i, x in enumerate(xs or (gx + 1, W - gx - 4)):
        # the deck's side lines are shaded from the housing outside them, on both sides
        beside = a[top:, x + 5 if xs and i else x - 3, :3]
        light = beside.sum(1, keepdims=True) > 300
        # a real groove (Roy 2026-11-12): its shadowed wall, then its floor lightening
        # towards the lit far wall
        k = np.where(light, 1.0, 0.82)
        for i, f in enumerate((0.5, 0.68)):
            a[top:, x + i, :3] = beside * (f * k)
        a[top:, x + 2, :3] = np.minimum(255, beside * 1.08 + 6)
        if top == 0:
            a[0, x:x + 3, :3] = a[0, x:x + 3, :3] * 0.9
    return Image.fromarray(a.astype(np.uint8), 'RGBA')


def strips(name):
    return (Image.open(f'{A}slot_front_gb_{name}.png').convert('RGBA'),
            Image.open(f'{A}slot_back_gb_{name}.png').convert('RGBA'))


# Each theme's NES and SNES bands, cut from that theme's Game Boy housing; Originals has one
# housing for each console (tools/slot_art.py).
# Dark and light have the Originals' details (NES blocks, SNES deck), monochrome (Roy 2026-11-07).
import os
# DMG Grey: cut from the DMG's own grey housing, in the studio (Roy 2026-11-12, st5/st8).
for theme, nes_from, snes_from in [('dark', 'dark', 'dark'), ('light', 'light', 'light'),
                                   ('grey', 'orig', 'orig'), ('orig', 'orig_nes', 'orig_snes')]:
    if not os.path.exists(f'{A}slot_front_gb_{nes_from}.png'):
        continue                    # Originals' housings come from tools/slot_art.py first
    gb, back_gb = strips(nes_from)
    left_edge = gb.crop((L_END - EDGE, 0, L_END, H))       # a piece's right-hand edge
    right_edge = gb.crop((R_START, 0, R_START + EDGE, H))  # a piece's left-hand edge
    base = gb.getpixel((120, 30))[:3]

    # ---- NES ----
    front = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    lx = MX - GAP
    front.paste(gb.crop((0, 0, lx, H)), (0, 0))
    rx = MX + MW + GAP
    front.paste(gb.crop((R_START + EDGE, 0, R_START + EDGE + W - rx, H)), (rx, 0))
    lid = plain(front, MW)
    seam = back_gb.getpixel((C, 30))
    back = Image.new('RGBA', (W, H), seam)
    # The lid in its opening: no bevelled edges but the SNES's grooves (Roy 2026-11-12), the
    # housing's and the lid's top corners a little round.
    lid_seams(front, lid, back, lx, rx)
    back.alpha_composite(front)
    back.alpha_composite(lid, (MX, 0))
    # The real NES's top beside its lid: the two black blocks of the ribbed strip, as tall
    # together as the lid (Roy's top-view photo); the ribs run on below, out of the band.
    for part in (front, back):
        nes_blocks(part, mono=theme != 'orig')
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
    if RIM > 8:
        a = np.asarray(band, np.float32).copy()
        src = np.arange(RIM) * 8 / RIM
        lo = np.floor(src).astype(int)
        f = (src - lo)[:, None, None]
        a[:RIM] = a[lo] * (1 - f) + a[lo + 1] * f
        band = Image.fromarray(a.round().astype(np.uint8), 'RGBA')
    band = grooves(pal_deck(band, mono=theme != 'orig'), DECK_B + LIP + 1)
    # a line down each side of the deck over the band's full height, as the grooves, standing
    # off the deck twice as far as the grooves stand in from these lines (Roy 2026-11-12)
    gx = C - 120 - 2 - 4
    off = 2 * (gx - (DECK_X[0] - 4))
    band = grooves(band, 0, (gx - off + 1, W - gx - 4 + off))
    band.save(f'{A}slot_front_snes_{theme}.png', optimize=True)
    band.save(f'{A}slot_back_snes_{theme}.png', optimize=True)

# The NES and SNES greys' Game Boy housings were only the stuff the bands are cut from.
for name in ('orig_nes', 'orig_snes'):
    for part in ('front', 'back'):
        if os.path.exists(f'{A}slot_{part}_gb_{name}.png'):
            os.remove(f'{A}slot_{part}_gb_{name}.png')
