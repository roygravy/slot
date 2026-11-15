"""The Game Boy slots' neutral images, coloured when drawn as the carts are (Roy 2026-11-06:
the slots with the same logic as the carts). Run from the repo root; needs scipy.

Solid (also Pearl and Metallic): tools/slot_art.py's draw() is linear in the plastic's colour,
so a slot in any colour is K + colour * T. K (RGBA) is everything that is not plastic: the metal
plate, the cast shadow, the catch-lights; T (grey) is the plastic's own shading and coverage,
stored as T * T_SCALE, with the plastic's coverage as its alpha. The anti-aliased edges stay exactly as drawn. One per mouth:
  dmg   the DMG's deep mouth (tools/slot_art.py)
  flat  the Pocket's and the Light's shallow mouth
  gbc   the Game Boy Color's shallow mouth
  gba   the GBA SP's mouth with its metal plate
Writes assets/slot_{back,front}_n_<mouth>_{k,t}.png.

Clear: each model's clear case in colourless plastic (tools/slot_cases/clear_*_preview.py) is
multiplied by the case's tint, plate and board too, as seen through coloured plastic; only the
GBA's plate where it lies bare in the mouth stays silver, through a mask
(assets/slot_back_c_gba_m.png, 255 = tinted). The GBC's colourless one is GBC_CASE=neutral of
clear_gbc_preview.py."""
import os
import numpy as np
from PIL import Image

T_SCALE = 170.0
SRC = open('tools/slot_art.py').read().split('MOUTHS = ')[0].replace(
    'ROOT = Path(__file__).resolve().parents[1]', 'ROOT=None')
# linear in the plastic: the plate always the light slots' silver, the mottle the same on all
LINEAR = [
    ('steel = np.array([158, 160, 164], np.float32) if plastic.mean() > 80 else np.array(\n                [96, 98, 102], np.float32)',
     'steel = np.array([158, 160, 164], np.float32)'),
    ('c += mottle(74)[..., None] * (0.6 if plastic.mean() < 80 else 1.0)', 'c += mottle(74)[..., None]'),
    # the plastic's own coverage, kept apart from its shading (T's alpha): Pearl and Metallic
    # ramp the shading alone and lay the result down by coverage, so their edges stay smooth
    ('    c = c.reshape(H, SS, W, SS, 3).mean(axis=(1, 3))',
     '    pm = inside | (wall if front else gap)\n    if metal and not front:\n        pm = pm & ~plate\n'
     '    COV.append(pm.reshape(H, SS, W, SS).mean(axis=(1, 3)))\n    c = c.reshape(H, SS, W, SS, 3).mean(axis=(1, 3))'),
]
SHALLOW = [   # the Pocket's, the Light's and the GBC's mouth (Roy 2026-11-05)
    ("cast = 0.50 * np.exp(-(d_out - WALL).clip(0) / 3.5)", "cast = 0.08 * np.exp(-(d_out - WALL).clip(0) / 1.0)"),
    ("deep = deep * (0.80 + 0.20 * smooth(INNER, INNER + 8.0, d_out_px))", "deep = deep * (0.98 + 0.02 * smooth(INNER, INNER + 2.0, d_out_px))"),
    ("side_shade = left_w * (0.46 + 0.12 * t) + (1 - left_w) * (0.70 - 0.06 * t)", "side_shade = left_w * (0.86 + 0.06 * t) + (1 - left_w) * (0.93 - 0.02 * t)"),
    ("deep = 0.60 + 0.12 * smooth(3.0, 30.0, yy)", "deep = 0.94 + 0.02 * smooth(TOP_BEVEL, 30.0, yy)"),
    ("deep = np.where(rim, 0.92 - 0.10 * (yy / 2.5), deep)", "deep = np.where(yy < TOP_BEVEL + 1, deep * shade, deep)"),
    ("GB_MOUTH = (244, 476)", "GB_MOUTH = (250, 470)"),
]
GBC = [   # the GBC's ribs: round lit tops, their shadow following them (Roy 2026-11-05)
    ("WALL = 2.2", "WALL = 0.5"), ("INNER = 5.5", "INNER = 1.0"),
    ("rib_shade = np.where(dy > 0, 1.0 - 0.25 * np.clip(-(yy - top) / r, 0, 1), 1.0)", "rib_shade = np.where(dy > 0, 1.0 + 0.08 * np.clip(-(yy - top) / r, 0, 1), 1.0)"),
    ("shadow = gap & ~rib & (xs_ > cx) & (xs_ < cx + r + 2.5) & (yy > RIB_TOP + 1)", "shadow = gap & ~rib & (xs_ > cx) & (np.hypot(xs_ - cx - 1.4, np.maximum(top + 0.8 - yy, 0)) < r + 1.0)"),
]
FLAT = [("WALL = 2.2", "WALL = 0.7"), ("INNER = 5.5", "INNER = 1.4")]
MOUTHS = {'dmg': ([], 'gb', False), 'flat': (FLAT + SHALLOW, 'gb', False),
          'gbc': (GBC + SHALLOW, 'gb', False), 'gba': ([], 'gba', True)}


def module(edits):
    src = SRC
    for a, b in LINEAR + edits:
        assert src.count(a) == 1, a
        src = src.replace(a, b)
    g = {'__file__': 'tools/slot_art.py', 'COV': []}
    exec(src, g)
    return g


for name, (edits, mouth, metal) in MOUTHS.items():
    g = module(edits)
    m = g[mouth + '_mouth']()
    for part, front in (('back', False), ('front', True)):
        k = g['draw'](m, np.zeros(3, np.float32), front, metal).astype(np.float32)
        a = g['draw'](m, np.full(3, 128, np.float32), front, metal).astype(np.float32)
        t = ((a[..., :3] - k[..., :3]) / 128).mean(2)
        Image.fromarray(k.astype(np.uint8), 'RGBA').save(f'assets/slot_{part}_n_{name}_k.png', optimize=True)
        cov = g['COV'][-1]
        la = np.dstack([np.clip(t * T_SCALE + 0.5, 0, 255), cov * 255 + 0.5]).astype(np.uint8)
        Image.fromarray(la, 'LA').save(
            f'assets/slot_{part}_n_{name}_t.png', optimize=True)
        print(name, part, 'T max', round(float(t.max()), 3))

# the clear GBA back's plate where it lies bare in the mouth, below the plastic back wall: kept
# silver. Under the clear plastic a plate takes the plastic's colour like the board does (Roy
# 2026-11-06), so the DMG's (all under plastic) has no mask.
os.environ['OUT'] = '/tmp/slot_neutral_out'
os.makedirs('/tmp/slot_neutral_out', exist_ok=True)
path = 'tools/slot_cases/clear_gba_preview.py'
d = {'__file__': os.path.abspath(path)}
exec(open(path).read(), d)
bare = np.asarray(d['pcov'], np.float32) * (d['yf'] >= d['MT']) * (d['xf'] >= d['PX0']) * (d['xf'] < d['PX1'])
Image.fromarray(((1 - bare) * 255).round().astype(np.uint8), 'L').save('assets/slot_back_c_gba_m.png', optimize=True)
print('mask gba')
