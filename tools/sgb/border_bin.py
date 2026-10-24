"""Turn a 256x224 border picture (four palettes of 16 colours at most, the screen hole in black) into the
Super Game Boy border slot. hands mGBA: 0x2000 bytes of 4bpp SNES tiles, then 0x1000 of map
RAM (32x28 tile map, palettes 4-7, their colours at 0x800).
Usage: border_bin.py BORDER.png OUT.bin"""
import sys
import numpy as np
from PIL import Image

src, out = sys.argv[1], sys.argv[2]
img = np.asarray(Image.open(src).convert('RGB'))
if img.shape[0] == 223:  # a capture missing the first line: repeat the top line
    img = np.concatenate([img[:1], img])
assert img.shape == (224, 256, 3), img.shape
# the screen hole is black colour 0: a tile of nothing but colour 0 lets the game show through
img = img.copy()
img[40:184, 48:208] = 0
# each 8x8 tile picks one of the four border palettes (4-7) of 16 colours; colour 0 of every
# palette is the hole's black
def key(c): return (int(c[0]), int(c[1]), int(c[2]))
tile_cols = {}
for ty in range(28):
    for tx in range(32):
        t = img[ty * 8:ty * 8 + 8, tx * 8:tx * 8 + 8].reshape(-1, 3)
        tile_cols[ty, tx] = {key(c) for c in t} - {(0, 0, 0)}
palettes = []
for cols in sorted(tile_cols.values(), key=len, reverse=True):
    if any(cols <= p for p in palettes):
        continue
    best = min((p for p in palettes if len(p | cols) <= 15), key=lambda p: len(p | cols) - len(p), default=None)
    if best is None:
        palettes.append(set(cols))
    else:
        best |= cols
assert len(palettes) <= 4, f'{len(palettes)} palettes, the border can hold 4'
palettes = [[(0, 0, 0)] + sorted(p) for p in palettes]
tile_pal = {}
index = np.zeros((224, 256), np.uint8)
for (ty, tx), cols in tile_cols.items():
    n = next(i for i, p in enumerate(palettes) if cols <= set(p))
    tile_pal[ty, tx] = n
    t = img[ty * 8:ty * 8 + 8, tx * 8:tx * 8 + 8]
    for i, c in enumerate(palettes[n]):
        index[ty * 8:ty * 8 + 8, tx * 8:tx * 8 + 8][(t == c).all(-1)] = i
colours = sum(len(p) for p in palettes)

def tile_bytes(t):
    b = bytearray(32)
    for r in range(8):
        for plane in range(4):
            v = 0
            for x in range(8):
                v |= ((int(t[r, x]) >> plane) & 1) << (7 - x)
            b[(plane // 2) * 16 + r * 2 + (plane & 1)] = v
    return bytes(b)

tiles = [np.zeros((8, 8), np.uint8)]
lookup = {tiles[0].tobytes(): (0, 0)}
mapram = bytearray(0x1000)
for ty in range(28):
    for tx in range(32):
        t = index[ty * 8:ty * 8 + 8, tx * 8:tx * 8 + 8]
        if 6 <= tx < 26 and 5 <= ty < 23:
            assert not t.any(), f'the screen hole has colour at tile {tx},{ty}'
        key = None
        for flip, v in ((0, t), (0x40, t[:, ::-1]), (0x80, t[::-1, :]), (0xC0, t[::-1, ::-1])):
            if v.tobytes() in lookup:
                n, f0 = lookup[v.tobytes()]
                key = (n, flip)
                break
        if key is None:
            lookup[t.tobytes()] = (len(tiles), 0)
            key = (len(tiles), 0)
            tiles.append(t.copy())
        n, flip = key
        at = (ty * 32 + tx) * 2
        mapram[at] = n & 0xFF
        mapram[at + 1] = ((4 + tile_pal[ty, tx]) << 2) | flip | ((n >> 8) & 3)
assert len(tiles) <= 256, f'{len(tiles)} tiles, the border can hold 256'
for pi, pal in enumerate(palettes):
    for i, (r, g, b) in enumerate(pal):
        v = (r >> 3) | ((g >> 3) << 5) | ((b >> 3) << 10)
        mapram[0x800 + pi * 0x20 + i * 2] = v & 0xFF
        mapram[0x801 + pi * 0x20 + i * 2] = v >> 8
chars = b''.join(tile_bytes(t) for t in tiles).ljust(0x2000, b'\0')
open(out, 'wb').write(chars + bytes(mapram))
print(f'{src}: {len(tiles)} tiles, {len(palettes)} palettes, {colours} colours')
