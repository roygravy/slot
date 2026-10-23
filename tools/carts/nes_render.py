"""Render the NES cart front as moulded plastic: a height field lit from above, 50% grey flat."""
import sys, numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as nd

photo, out = sys.argv[1], sys.argv[2]
W, H, CW, SS = 312, 337, 302, 4
w, h = CW * SS, H * SS

# Silhouette from the photo, cleaned: threshold at high res, then smooth a little.
im = Image.open(photo).convert('L').crop((61, 48, 1851, 2047)).resize((w, h), Image.Resampling.LANCZOS)
inside = np.asarray(im) < 200
inside = nd.binary_opening(nd.binary_closing(inside, iterations=3), iterations=3)
inside = nd.binary_fill_holes(inside)

def U(u): return u * w
def V(v): return v * h

def poly(draw_fn):
    m = Image.new('L', (w, h), 0); draw_fn(ImageDraw.Draw(m)); return np.asarray(m) > 127

def rect(u0, v0, u1, v1, r=0.0):
    return poly(lambda d: d.rounded_rectangle([U(u0), V(v0), U(u1), V(v1)], radius=r * w, fill=255))

height = np.zeros((h, w), np.float32)
# body, rounded over at its outer edge
dist = nd.distance_transform_edt(inside)
height += np.where(inside, np.clip(dist / (3.0 * SS), 0, 1) ** 0.5, 0) * 10

channel = rect(0.156, -0.01, 0.374, 0.995) & inside
grip = rect(0.160, -0.01, 0.370, 0.132, 0.004) & inside
label = rect(0.400, -0.01, 0.866, 0.678, 0.010) & inside
panel = rect(0.163, 0.902, 0.367, 0.990, 0.014)
height -= channel * 3.0
height -= grip * 9.0
for k in range(3):
    v0 = 0.040 + k * 0.035
    height += rect(0.160, v0, 0.370, v0 + 0.012) * 3.5
for k in range(20):
    v0 = 0.151 + k * 0.0375
    height += rect(0.164, v0, 0.366, v0 + 0.0175, 0.003) * 2.6
height += panel * 2.6
height -= label * 1.2
tri = poly(lambda d: d.polygon([(U(0.458), V(0.741)), (U(0.547), V(0.741)), (U(0.502), V(0.810))], fill=255))
height -= tri * 0.9
yy, xx = np.mgrid[0:h, 0:w]
# the screw dimple: a shallow round dish with a soft rim and a tiny bump in the middle
r = np.hypot(xx - U(0.502), yy - V(0.576)) / (0.017 * w)
height += (np.where(r < 1, -np.clip(1 - r * r, 0, 1) ** 0.6, 0) + 0.18 * np.exp(-((r - 1.02) / 0.10) ** 2)
           + 0.25 * np.exp(-(r / 0.28) ** 2) * (r < 1)) * 1.8
# a fine seam where the channel's ridge meets the right half
height -= rect(0.372, -0.01, 0.376, 0.995) * 0.8

hs = nd.gaussian_filter(height, 1.1 * SS / 2)
gy, gx = np.gradient(hs)
nz = 1.0 / np.sqrt(gx * gx + gy * gy + 1)
lx, ly, lz = -0.25, -0.75, 0.62
ln = np.sqrt(lx * lx + ly * ly + lz * lz); lx, ly, lz = lx / ln, ly / ln, lz / ln
lam = (-gx * lx - gy * ly + lz) * nz
flat = lz
shade = lam / flat
# soft occlusion: recesses read a touch darker
occ = np.clip((nd.gaussian_filter(hs, 6 * SS) - hs) * 0.05, 0, 0.22)
grey = 119 * (1 + (shade - 1) * 1.5) * (1 - occ)
grip_soft = nd.gaussian_filter((grip & ~(hs > hs[grip].mean() + 2)).astype(np.float32), SS)
grey *= 1 - 0.22 * grip_soft
grey *= 1 - 0.07 * nd.gaussian_filter(tri.astype(np.float32), SS / 2)
rng = np.random.default_rng(7)
grain = nd.gaussian_filter(rng.normal(0, 1, (h, w)).astype(np.float32), 1.0 * SS)
grey += grain / grain.std() * 2.8  # coarse enough to survive the shelf's scale (Roy)

def down(a):
    return np.asarray(Image.fromarray(a.astype(np.float32), 'F').resize((CW, H), Image.Resampling.BOX))
A = np.clip(down(inside.astype(np.float32)), 0, 1)
G = down(grey * inside) / np.maximum(A, 1e-3)
def save(gv, name, alpha=A):
    c = np.zeros((H, W, 4), np.uint8); off = (W - CW) // 2
    c[:, off:off + CW, :3] = np.clip(np.where(alpha > 0, gv, 128), 0, 255).round().astype(np.uint8)[..., None]
    c[:, off:off + CW, 3] = (alpha * 255).round().astype(np.uint8)
    Image.fromarray(c, 'RGBA').save(f'{out}/{name}')
save(G, 'nes_neutral_render.png')
m = np.zeros((H, W, 4), np.uint8); m[..., :3] = 255; m[:, 5:5 + CW, 3] = (A * 255).round().astype(np.uint8)
Image.fromarray(m, 'RGBA').save(f'{out}/nes_cart_render.png')
# clear: faint relief, darker rim, diagonal sheen
Ad = A > 0.5
rim = Ad & ~nd.binary_erosion(Ad, iterations=5)
yy, xx = np.mgrid[0:H, 0:CW]
sheen = np.abs(xx + yy - CW / 3) < CW / 14
clear = 128 + (G - 128) * 0.45
clear = np.where(rim, 100, clear)
save(clear, 'nes_clear_render.png')
print('ok')
