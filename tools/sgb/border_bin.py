"""Turn a 256x224 border picture (16 colours at most, the screen hole in black) into the
Super Game Boy border slot. hands mGBA: 0x2000 bytes of 4bpp SNES tiles, then 0x1000 of map
RAM (32x28 tile map, palette 4, with the 16 colours at 0x800).
Usage: border_bin.py BORDER.png OUT.bin"""
import sys
import numpy as np
from PIL import Image

src, out = sys.argv[1], sys.argv[2]
img = np.asarray(Image.open(src).convert('RGB'))
assert img.shape == (224, 256, 3), img.shape
# colour 0 is the screen hole's black: a tile of nothing but colour 0 lets the game show through
colours = [(0, 0, 0)]
for c in map(tuple, img.reshape(-1, 3)):
    if c not in colours:
        colours.append(c)
assert len(colours) <= 16, f'{len(colours)} colours, the border can hold 16'
index = np.zeros((224, 256), np.uint8)
for i, c in enumerate(colours):
    index[(img == c).all(-1)] = i

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
        mapram[at + 1] = 0x10 | flip | ((n >> 8) & 3)
assert len(tiles) <= 256, f'{len(tiles)} tiles, the border can hold 256'
for i, (r, g, b) in enumerate((int(r), int(g), int(b)) for r, g, b in colours):
    v = (r >> 3) | ((g >> 3) << 5) | ((b >> 3) << 10)
    mapram[0x800 + i * 2] = v & 0xFF
    mapram[0x801 + i * 2] = v >> 8
chars = b''.join(tile_bytes(t) for t in tiles).ljust(0x2000, b'\0')
open(out, 'wb').write(chars + bytes(mapram))
print(f'{src}: {len(tiles)} tiles, {len(colours)} colours')
