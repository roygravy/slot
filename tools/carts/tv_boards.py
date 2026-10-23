"""Redraw the NES and SNES boards in the Game Boy / GBA boards' style (gb_board.png, board.png):
mottled solder mask with a lighter rim, dark traces, gold-ringed vias, a via above every gold
finger, black-blue chips on silver gull-wing pins inside a white silkscreen box, a few small parts
and the big '2026 Slot.' mark. The outlines are the boards' current ones (the assets' alpha), so
they still fit their shells. Usage: tv_boards.py FONT_REGULAR FONT_BOLD (run from the repo root)."""
import sys, numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

REG, BOLD = sys.argv[1], sys.argv[2]
S = 4  # drawn at 4x, boxed down
GREEN = np.array([1, 72, 36.])
POUR = np.array([0, 52, 24.])
TRACE = np.array([0, 42, 18.])
RIM = np.array([20, 104, 62.])
RING = np.array([176, 166, 18.])
HOLE = np.array([6, 30, 16.])
NAVY = np.array([1, 24, 36.])
PIN, PIN_DARK = np.array([214, 214, 214.]), np.array([150, 150, 152.])
SILK = np.array([214, 220, 214.])
GOLD, GOLD_DARK = np.array([196, 128, 40.]), np.array([150, 92, 30.])

def board(name, layout):
    src = Image.open(f'assets/{name}.png').convert('RGBA')
    W, H = src.size
    w, h = W * S, H * S
    alpha = np.asarray(src)[..., 3]
    m = np.asarray(Image.fromarray(alpha).resize((w, h), Image.Resampling.BILINEAR)) > 127
    rng = np.random.default_rng(len(name))
    yy, xx = np.mgrid[0:h, 0:w]

    def layer():
        im = Image.new('L', (w, h), 0)
        return im, ImageDraw.Draw(im)
    def on(im): return np.asarray(im) > 127

    # mottled mask: soft blotches and a few dark specks, as the GBA board has
    blot = nd.gaussian_filter(rng.normal(0, 1, (h, w)), 6 * S)
    blot /= blot.std()
    rgb = GREEN[None, None, :] * (1 + 0.035 * blot[..., None])
    speck = nd.gaussian_filter(rng.normal(0, 1, (h, w)), 0.6 * S)
    rgb *= 1 - 0.18 * np.clip(speck / speck.std() - 2.2, 0, 1)[..., None]

    L = layout
    # copper pours under the mask
    pim, pd = layer()
    for box in L.get('pours', []):
        pd.rounded_rectangle([c * S for c in box], radius=3 * S, fill=255)
    rgb[on(pim)] = rgb[on(pim)] * 0.25 + POUR * 0.75

    # traces: from each finger's via straight up, then a 45 degree jog towards the chips
    tim, td = layer()
    fx0, fx1, fy0, fy1 = L['fingers']
    n = L['count']
    pitch = (fx1 - fx0) / n
    vias = []
    for k in range(n):
        x = fx0 + (k + 0.5) * pitch
        vy = fy0 - 3.2
        vias.append((x, vy))
        if k % 2 == 0 or k % 7 == 3:
            top = L['fan_top'] + (k * 37 % 11) * 1.3
            jog = (x - (fx0 + fx1) / 2) * -0.18
            td.line([(x * S, vy * S), (x * S, (top + 8) * S), ((x + jog) * S, top * S),
                     ((x + jog) * S, (top - 6) * S)], fill=255, width=int(1.1 * S))
            vias.append((x + jog, top - 6))
    for path in L.get('traces', []):
        td.line([(px * S, py * S) for px, py in path], fill=255, width=int(1.1 * S), joint='curve')
    rgb[on(tim)] = TRACE

    # gold fingers, lighter at the top where the plating is fresh
    gim, gd = layer()
    for k in range(n):
        x = fx0 + k * pitch
        gd.rectangle([(x + 0.12 * pitch) * S, fy0 * S, (x + 0.88 * pitch) * S, fy1 * S], fill=255)
    g = on(gim)
    t = np.clip((yy - fy0 * S) / max((fy1 - fy0) * S, 1), 0, 1)
    gold = GOLD[None, None, :] * (1 - 0.25 * t[..., None]) * (1 + 0.06 * blot[..., None])
    rgb[g] = gold[g]

    # chips: their pins' traces first, so the pins sit on top
    t2im, td2 = layer()
    pin_vias = []
    for (x0, y0, x1, y1, pins, side) in L['chips']:
        if side == 'tb':
            for k in range(0, pins, 2):
                u = x0 + (k + 0.5) / pins * (x1 - x0)
                run = 6 + (k * 5 % 4) * 2.5
                td2.line([(u * S, (y1 + 3) * S), (u * S, (y1 + 3 + run) * S)], fill=255, width=int(1.1 * S))
                pin_vias.append((u, y1 + 3 + run + 1.2))
    rgb[on(t2im)] = TRACE
    # chips: body, gull-wing pins top and bottom (or left and right), silkscreen box, pin-1 dot
    for (x0, y0, x1, y1, pins, side) in L['chips']:
        sim, sd = layer()
        pad = 4.4
        sd.rounded_rectangle([(x0 - pad) * S, (y0 - pad) * S, (x1 + pad) * S, (y1 + pad) * S],
                             radius=1.5 * S, outline=255, width=int(0.8 * S))
        rgb[on(sim)] = SILK
        for k in range(pins):
            if side == 'tb':
                u = x0 + (k + 0.5) / pins * (x1 - x0)
                for a, b in ((y0 - 3.4, y0), (y1, y1 + 3.4)):
                    rgb[(np.abs(xx - u * S) < 0.9 * S) & (yy >= a * S) & (yy < b * S)] = PIN
                    rgb[(np.abs(xx - u * S - 0.7 * S) < 0.25 * S) & (yy >= a * S) & (yy < b * S)] = PIN_DARK
            else:
                v = y0 + (k + 0.5) / pins * (y1 - y0)
                for a, b in ((x0 - 3.4, x0), (x1, x1 + 3.4)):
                    rgb[(np.abs(yy - v * S) < 0.9 * S) & (xx >= a * S) & (xx < b * S)] = PIN
                    rgb[(np.abs(yy - v * S - 0.7 * S) < 0.25 * S) & (xx >= a * S) & (xx < b * S)] = PIN_DARK
        body = (xx >= x0 * S) & (xx < x1 * S) & (yy >= y0 * S) & (yy < y1 * S)
        sheen = 1 + 0.35 * np.clip(1 - (yy - y0 * S) / ((y1 - y0) * S), 0, 1) * np.clip(1 - (xx - x0 * S) / ((x1 - x0) * S), 0, 1)
        rgb[body] = (NAVY[None, :] * sheen[body][:, None]) * (1 + 0.05 * blot[body][:, None])
        dot = (xx - (x0 + 2.2) * S) ** 2 + (yy - (y1 - 2.2) * S) ** 2 < (0.9 * S) ** 2
        rgb[dot] = NAVY * 0.55

    # small parts: a two-pad part with a silver end each side and its name above
    fim, fd = layer()
    f_small = ImageFont.truetype(REG, int(5.2 * S))
    for (x, y, label) in L.get('parts', []):
        fd.rectangle([(x - 3.6) * S, (y - 2.6) * S, (x + 3.6) * S, (y + 2.6) * S], outline=255, width=int(0.6 * S))
        fd.text((x * S, (y - 4.2) * S), label, font=f_small, fill=255, anchor='md')
        rgb[(np.abs(xx - x * S) < 2.6 * S) & (np.abs(yy - y * S) < 1.3 * S)] = [10, 70, 40]
        for e in (-1, 1):
            rgb[(np.abs(xx - (x + e * 2.2) * S) < 0.7 * S) & (np.abs(yy - y * S) < 1.3 * S)] = PIN
    rgb[on(fim)] = SILK

    # vias: a dark drill in a gold ring
    for (x, y) in vias + pin_vias + [tuple(v) for v in L.get('vias', [])]:
        r = np.hypot(xx - x * S, yy - y * S) / S
        rgb[(r < 1.9) & m] = RING
        rgb[(r < 1.0) & m] = HOLE

    # silkscreen: the board's code, and '2026 Slot.' as on the GBA board
    xim, xd = layer()
    cx, cy, size = L['mark']
    f_reg = ImageFont.truetype(REG, int(size * 0.62 * S))
    f_bold = ImageFont.truetype(BOLD, int(size * S))
    xd.text((cx * S, cy * S), '2026 ', font=f_reg, fill=255, anchor='ls')
    adv = xd.textlength('2026 ', font=f_reg)
    xd.text((cx * S + adv, cy * S), 'Slot.', font=f_bold, fill=255, anchor='ls')
    code_x, code_y, code = L['code']
    xd.text((code_x * S, code_y * S), code, font=ImageFont.truetype(REG, int(5.6 * S)), fill=255, anchor='ls')
    rgb[on(xim)] = SILK

    # the mask's lighter rim along the board edge
    d = nd.distance_transform_edt(np.pad(m, 1))[1:-1, 1:-1] / S
    rim = (d < 1.6) & ~g
    rgb[rim] = rgb[rim] * 0.4 + RIM * 0.6

    rgb = np.clip(rgb, 0, 255)
    out = np.zeros((h, w, 4), np.float32)
    out[..., :3] = rgb * m[..., None]
    out[..., 3] = m * 255.
    small = Image.fromarray(out.astype(np.uint8), 'RGBA').convert('RGBa').resize((W, H), Image.Resampling.BOX).convert('RGBA')
    a = np.asarray(small).copy()
    a[..., 3] = alpha  # keep the outline exactly as it was
    Image.fromarray(a, 'RGBA').save(f'assets/{name}.png', optimize=True)

# NES: wide body 20..284 x 214..302, a narrower top piece 158..275 from y 182; 36 fingers
board('nes_board', dict(
    fingers=(24, 280, 307, 335), count=36, fan_top=288,
    chips=[(32, 234, 118, 258, 12, 'tb'), (132, 234, 218, 258, 12, 'tb'), (236, 236, 270, 254, 6, 'tb')],
    parts=[(262, 198, 'C1'), (228, 274, 'C2')],
    traces=[[(118, 244), (132, 244)], [(118, 249), (132, 249)], [(218, 245), (236, 245)],
            [(253, 236), (253, 222), (240, 212), (240, 200)], [(26, 230), (26, 290)]],
    vias=[(240, 198), (276, 222), (276, 290), (26, 226)],
    mark=(166, 206, 14), code=(32, 226, 'NES-SLOT-01')))

# SNES (NTSC): body 49..312 x 65..197, tab 106..260 down to 227; 31 fingers
board('snes_board', dict(
    fingers=(110, 256, 203, 226), count=31, fan_top=176,
    chips=[(64, 92, 180, 124, 16, 'tb'), (206, 90, 268, 108, 8, 'tb'), (206, 136, 252, 152, 6, 'tb')],
    parts=[(290, 98, 'C1'), (278, 150, 'R1')],
    traces=[[(180, 100), (206, 99)], [(180, 116), (206, 144)], [(268, 99), (290, 120), (290, 170)],
            [(56, 100), (56, 170), (70, 182)]],
    vias=[(60, 76), (300, 76), (300, 186), (290, 172), (190, 80)],
    mark=(64, 168, 15), code=(206, 82, 'SHVC-SLOT-01')))

# SNES (PAL / Super Famicom): body 45..291 x 82..198, tab 91..244 down to 224; 31 fingers
board('snes_pal_board', dict(
    fingers=(95, 240, 201, 223), count=31, fan_top=180,
    chips=[(58, 108, 168, 138, 16, 'tb'), (190, 104, 250, 122, 8, 'tb'), (190, 140, 234, 154, 6, 'tb')],
    parts=[(272, 112, 'C1'), (264, 156, 'R1')],
    traces=[[(168, 114), (190, 113)], [(168, 130), (190, 147)], [(250, 113), (272, 132), (272, 176)],
            [(52, 116), (52, 176), (64, 186)]],
    vias=[(55, 92), (281, 92), (281, 188), (272, 178), (176, 96)],
    mark=(60, 171, 14), code=(190, 98, 'SNSP-SLOT-01')))
print('ok')
