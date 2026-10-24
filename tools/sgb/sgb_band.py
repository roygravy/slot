"""The slot band while a Super Game Boy sits in the SNES: its top, zoomed so the black opening in
its bay is as wide as the Game Boy slot.

usage: sgb_band.py FACE.png MOUTH.npy OUT_PREFIX
FACE is the finished cart (assets/sgb_face_*.png), MOUTH the black opening's mask at the
render's size. Writes OUT_PREFIX_back.png (behind the Game Boy cart) and OUT_PREFIX_front.png
(in front of it: the lip below the opening)."""
import sys
import numpy as np
from PIL import Image

A = 'assets/'
W, H = 720, 118
RISE = 60           # the SGB stands this far above the console's 58px band (Roy)
HOLE = (246, 474)   # the Game Boy slot's opening in slot_front_gb_*.png
LIP_Y = 44 + RISE   # where the lip below the opening crosses the middle of the band

face = Image.open(sys.argv[1]).convert('RGBA')
mouth = np.load(sys.argv[2])
out = sys.argv[3]
mouth = np.asarray(Image.fromarray((mouth * 255).astype(np.uint8)).resize(face.size, Image.Resampling.BILINEAR)) > 127
ys, xs = np.where(mouth)
zoom = (HOLE[1] - HOLE[0]) / (xs.max() + 1 - xs.min())
fw, fh = round(face.width * zoom), round(face.height * zoom)
big = face.resize((fw, fh), Image.Resampling.LANCZOS)
mbig = np.asarray(Image.fromarray(mouth.astype(np.uint8) * 255).resize((fw, fh), Image.Resampling.BILINEAR)) > 127
cx = (xs.min() + xs.max() + 1) / 2 * zoom
mid = int(cx)
lip = np.where(mbig[:, mid])[0].max() + 1
ox, oy = round(W / 2 - cx), LIP_Y - lip

def band():
    # the SGB alone: no console band beside it (Roy)
    im = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    im.alpha_composite(big, (ox, oy))
    return im

back = band()
front = np.asarray(band()).copy()
# the Game Boy cart shows above the lip, inside the bay's opening
cols = np.where(mbig.any(axis=0))[0]
for x in cols:
    bottom = np.where(mbig[:, x])[0].max() + 1 + oy
    tx = x + ox
    if 0 <= tx < W:
        front[:max(0, min(H, bottom)), tx, 3] = 0
back.save(out + '_back.png', optimize=True)
Image.fromarray(front).save(out + '_front.png', optimize=True)
