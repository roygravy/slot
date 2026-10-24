"""Build the NES and SNES boards out of the GBA and Game Boy boards (board.png, gb_board.png), so
they are the same make: the GBA board's mottled green is copied straight over (its own features
patched out), its edge (a lighter cut rim) and drop shadow are copied by profile, its gold fingers
and the via over each one are copied cell by cell, and the chips are the Game Boy board's own,
turned on their side. Only the traces are drawn. One SNES board serves the NTSC and the PAL cart
(as the real carts share one), at the same size in both.
Usage: tv_boards.py (from the repo root)."""
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as nd

gba = np.asarray(Image.open('assets/board.png').convert('RGBA')).astype(np.float32)
gb = np.asarray(Image.open('assets/gb_board.png').convert('RGBA')).astype(np.float32)
R, G, B = gba[..., 0], gba[..., 1], gba[..., 2]
opaque = gba[..., 3] > 250
inner = nd.distance_transform_edt(opaque)
TRACE = np.array([0, 30, 18.])

# ---- the GBA board's green, with its parts patched out of it ----
clean = opaque & (inner > 6) & (G > R + 30) & (G > B + 15) & (G > 52) & (G < 125)
clean = nd.binary_erosion(clean, iterations=1)
tex, ok = gba[..., :3].copy(), clean.copy()
rng = np.random.default_rng(3)
for _ in range(400):
    if ok[16:150, 24:334].all():
        break
    dy, dx = rng.integers(-60, 61, 2)
    src, sok = np.roll(tex, (dy, dx), (0, 1)), np.roll(ok, (dy, dx), (0, 1))
    fill = ~ok & sok
    tex[fill] = src[fill]
    ok |= fill
TILE = tex[16:150, 24:334]

def green(h, w):
    t = np.concatenate([TILE, TILE[:, ::-1]], 1)
    t = np.concatenate([t, t[::-1]], 0)
    reps = (h // t.shape[0] + 1, w // t.shape[1] + 1)
    return np.tile(t, (*reps, 1))[:h, :w].copy()

# the cut edge, by distance from it (measured down the GBA board's left side)
RIM = {1: np.array([17, 108, 66.]), 2: np.array([9, 96, 56.]), 3: np.array([1, 77, 39.])}
FLAT = np.array([1, 68, 32.])
# the drop shadow below (rows 1..4 under the edge) and the faint one on the right
SHADOW_DOWN, SHADOW_RIGHT = (84, 84, 63, 21), 20

def feature(px):
    """The non-green pixels of a copied part: gold, silver, silkscreen, chip, drill."""
    r, g, b = px[..., 0], px[..., 1], px[..., 2]
    return (px[..., 3] > 128) & ~((g > r + 25) & (g > b + 8) & (r < 40))

# ---- parts to copy ----
st = 55  # a middle finger of the GBA board: gold from x 55, 7 wide, pitch 9
# eight neighbouring fingers of the GBA board in turn, so the diagonal shading in the gold
# alternates as it does there; each cell is the via over a finger, the finger, the green under it
def finger_cell(x0):
    c = gba[156:201, x0 - 1:x0 + 8].copy()
    return c
FINGERS = [finger_cell(46 + 9 * i) for i in range(8)]
FINGER_BOTTOM = 200 - 156                       # the cell's last board row
VIA = gba[146:155, 300:309]                     # a lone via, drill and ring
CHIP_BIG = np.rot90(gb[168:270, 66:134])        # the Game Boy board's 32-pin chip, on its side
CHIP_SMALL = np.rot90(gb[45:92, 113:150])       # its 8-pin chip
PART = gba[63:93, 186:206]                      # R5: a two-pad part and its name

def paste(rgb, part, x, y, holes=None):
    """Copy a part's own pixels over the green; its drill holes (see-through on the source
    boards) are marked in `holes` so they stay see-through here too."""
    h, w = part.shape[:2]
    f = feature(part)
    region = rgb[y:y + h, x:x + w]
    region[f] = part[..., :3][f]
    if holes is not None:
        solid = part[..., 3] > 128
        holes[y:y + h, x:x + w] |= nd.binary_fill_holes(solid) & ~solid

def build(mask4, fingers, chips, parts, vias, traces):
    """mask4: the board's outline at 4x (bool), so its edge and corners come out smooth.
    fingers: (x0, count, bottom row)."""
    h, w = mask4.shape[0] // 4, mask4.shape[1] // 4
    def down(a):
        return a.reshape(h, 4, w, 4).mean((1, 3))
    cover = down(mask4.astype(np.float32))
    mask = cover > 0
    rgb = green(h, w)
    # the cut rim, by each pixel's mean distance from the edge, so it follows the curves
    d = down(nd.distance_transform_edt(np.pad(mask4, 4))[4:-4, 4:-4] / 4)
    for k, c in RIM.items():
        wgt = np.clip(1 - np.abs(d - (k - 0.5)), 0, 1)[..., None] * mask[..., None]
        rgb = rgb * (1 - wgt) + (c + (rgb - FLAT) * 0.5) * wgt
    # traces, drawn at 4x for soft edges, in the GBA board's trace green
    S = 4
    im = Image.new('L', (w * S, h * S), 0); dr = ImageDraw.Draw(im)
    for path in traces:
        dr.line([(x * S + S / 2, y * S + S / 2) for x, y in path], fill=255, width=int(1.6 * S), joint='curve')
    cov = np.asarray(im.resize((w, h), Image.Resampling.BOX)).astype(np.float32)[..., None] / 255 * mask[..., None]
    rgb = rgb * (1 - cov) + TRACE * cov
    holes = np.zeros_like(mask)
    for part, x, y in chips + parts:
        paste(rgb, part, x, y, holes)
    for x, y in vias:
        paste(rgb, VIA, x - 4, y - 4, holes)
    for x0, n, bottom in (fingers if isinstance(fingers, list) else [fingers]):  # one group or several
        for k in range(n):
            paste(rgb, FINGERS[k % len(FINGERS)], x0 + k * 9, bottom - FINGER_BOTTOM, holes)
    def shift(m, dy, dx):
        out = np.zeros_like(m)
        out[dy:, dx:] = m[:m.shape[0] - dy, :m.shape[1] - dx]
        return out
    # the drop shadow under the board (black), behind its partly covered edge pixels
    shadow = np.zeros((h, w), np.float32)
    for k, a in enumerate(SHADOW_DOWN, 1):
        shadow = np.maximum(shadow, shift(cover, k, 0) * a / 255)
    shadow = np.maximum(shadow, shift(cover, 0, 1) * SHADOW_RIGHT / 255)
    cover = np.where(holes & (cover > 0.99), 0, cover)
    alpha = cover + (1 - cover) * shadow
    rgb = np.where(alpha[..., None] > 0, rgb * cover[..., None] / np.maximum(alpha, 1e-6)[..., None], 0)
    out = np.dstack([np.clip(rgb, 0, 255), alpha * 255]).round().astype(np.uint8)
    return out

def rr(h, w, box, r):
    """A rounded rectangle at 4x; box is inclusive, in board pixels."""
    m = Image.new('L', (w * 4, h * 4), 0)
    x0, y0, x1, y1 = box
    ImageDraw.Draw(m).rounded_rectangle((x0 * 4, y0 * 4, x1 * 4 + 3, y1 * 4 + 3), radius=r * 4, fill=255)
    return np.asarray(m) > 127

# ---- SNES: one board, 247 wide, for both carts ----
BW, BH = 247, 147  # the last 4 rows hold the drop shadow
FOOT = BH - 5       # the board's last row
def snes_board():
    m = rr(BH, BW, (0, 0, BW - 1, 116), 7)  # round corners, as the GBA board's
    m |= rr(BH, BW, (44, 100, 202, FOOT), 6)
    for cx in (0, BW - 1):  # the half-round notches the shell's posts sit in
        m &= ~rr(BH, BW, (cx - 5, 53, cx + 5, 63), 5)
    x0, n = 48, 17
    traces = []
    for k in range(n):
        x = x0 + k * 9 + 4
        # into the big chip's pins on the left, the small chips' on the right, at 45 degrees
        tx = 14 + k * 9.2 if k < 10 else 146 + (k - 10) * 6.5
        up = abs(tx - x)
        traces.append([(x, FOOT - 46), (x, 98), (tx, 98 - up), (tx, 70)])
    traces += [[(110, 40), (132, 40), (140, 26)], [(196, 30), (214, 30), (222, 44), (222, 92)],
               [(10, 82), (10, 100), (40, 100)]]
    vias = [(140, 23), (222, 95), (10, 79), (234, 12), (12, 12), (234, 104), (130, 92)]
    chips = [(CHIP_BIG, 8, 10), (CHIP_SMALL, 142, 8), (CHIP_SMALL, 142, 52)]
    parts = [(PART, 200, 44)]
    return build(m, (x0, n, FOOT), chips, parts, vias, traces)

if __name__ == '__main__':  # tools/sgb/carts/sgb2_board.py borrows build() and the parts
    board = snes_board()
    for name, (W, H) in (('snes_board', (360, 231)), ('snes_pal_board', (336, 228))):
        c = np.zeros((H, W, 4), np.uint8)
        x, y = (W - BW) // 2, H - BH
        c[y:y + BH, x:x + BW] = board
        Image.fromarray(c, 'RGBA').save(f'assets/{name}.png', optimize=True)

    # ---- NES: its own outline (a wide body, a narrower piece on top), fingers across the bottom ----
    def nes_board():
        """Sized on Roy's photo of an open NES cart (cart 6..305 wide here, so 0.755 of the photo):
        a wide body, a narrower piece on top at the right, and a tab with the fingers that fits
        the cart's narrower foot."""
        h, w = 337, 312
        m = rr(h, w, (28, 219, 280, 288), 7)
        m |= rr(h, w, (155, 180, 279, 230), 7)
        m |= rr(h, w, (37, 270, 271, 331), 6)
        x0, n = 37 + (234 - 25 * 9) // 2, 25
        traces, vias = [], []
        for k in range(n):
            x = x0 + k * 9 + 4
            if k % 3 == 0:
                traces.append([(x, 292), (x, 284)])
        traces += [[(180, 199), (180, 214), (190, 222)], [(262, 200), (274, 212), (274, 262)], [(32, 226), (32, 280)]]
        vias += [(274, 265), (180, 196), (32, 223), (32, 283)]
        chips = [(CHIP_BIG, 37, 221), (CHIP_BIG, 147, 221), (CHIP_SMALL, 206, 184)]
        parts = [(PART, 257, 226)]
        return build(m, (x0, n, 331), chips, parts, vias, traces)

    Image.fromarray(nes_board(), 'RGBA').save('assets/nes_board.png', optimize=True)
    print('ok')
