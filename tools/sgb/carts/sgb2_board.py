"""The Super Game Boy 2's board, seen through its clear shell: the same make as slot.'s other
boards (tools/carts/tv_boards.py builds it: the GBA board's mottled green, cut rim and shadow,
its gold fingers and vias, the Game Boy board's chips), in the outline of Roy's photo of an
opened Super Game Boy: a wide body with narrower shoulders at the top, notches for the shell's
posts, and three groups of fingers. No chips (it shows its back), branding or light (Roy).

usage: sgb2_board.py OUT   (from the repo root) writes OUT/sgb2_pcb.png at the face's 336x319."""
import os, sys
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../carts'))
import tv_boards as tb

W, H = 336, 319
FOOT = 314                           # the fingers' last row, just above the cart's bottom edge
def board():
    m = tb.rr(H, W, (40, 140, 312, 286), 7)                 # the body
    m |= tb.rr(H, W, (55, 124, 297, 160), 7)                # its shoulders, either side of the slot
    m |= tb.rr(H, W, (42, 270, 294, FOOT), 6)               # the finger tab
    for cx in (40, 312):                                    # the half-round notches for the posts
        m &= ~tb.rr(H, W, (cx - 5, 205, cx + 5, 215), 5)
    for gx in (89, 245):                                    # the gaps between the finger groups
        m &= ~tb.rr(H, W, (gx, 296, gx + 4, FOOT + 4), 1)
    groups = [(44, 5, FOOT), (96, 17, FOOT), (250, 5, FOOT)]
    # the back of the board, as on Roy's photo: no chips, only traces running up from the fingers
    # and wandering off between rows of vias
    traces, vias = [], []
    rng = np.random.default_rng(7)
    for x0, n, _ in groups:
        for k in range(n):
            x = x0 + k * 9 + 4
            top = 262 - (k % 5) * 9
            if k % 2 == 0:
                side = 1 if x < W / 2 else -1
                bend = x + side * (8 + (k % 4) * 5)
                traces.append([(x, FOOT - 46), (x, top), (bend, top - abs(bend - x))])
                if k % 4 == 0:
                    vias.append((bend, top - abs(bend - x) - 3))
            else:
                traces.append([(x, FOOT - 46), (x, top + 4)])
    for _ in range(6):
        x, y = rng.integers(70, 280), rng.integers(170, 225)
        dx = int(rng.choice([-1, 1]) * rng.integers(16, 40))
        traces.append([(x, y), (x + dx, y), (x + dx + 8 * np.sign(dx), y + 8)])
        vias.append((x + dx + 8 * np.sign(dx), y + 11))
    vias += [(52, 150), (288, 150), (290, 262), (50, 262)]
    chips, parts = [], []
    return tb.build(m, groups, chips, parts, vias, traces)

Image.fromarray(board(), 'RGBA').save(os.path.join(sys.argv[1], 'sgb2_pcb.png'))
print('ok')
