"""Render the NTSC Super Game Boy: front (50% grey) and silhouette."""
import os, sys, numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

out, font_path = sys.argv[1], sys.argv[2]
W, H, SS = 400, 350, 4
# SGB_F renders it that many times larger, every detail with it: the band's zoomed top (Roy)
F = float(os.environ.get('SGB_F', '1'))
SS = SS * F
w, h = int(W * SS), int(H * SS)
W, H = round(W * F), round(H * F)
def U(u): return u * w
def V(v): return v * h
def mask(fn):
    m = Image.new('L', (w, h), 0); fn(ImageDraw.Draw(m)); return np.asarray(m) > 127
def rect(u0, v0, u1, v1, r=0.0):
    return mask(lambda d: d.rounded_rectangle([U(u0), V(v0), U(u1), V(v1)], radius=r * w, fill=255))
def disc(u, v, r):
    return mask(lambda d: d.ellipse([U(u) - r * w, V(v) - r * w, U(u) + r * w, V(v) + r * w], fill=255))
def light(height, gain=1.5, blur=0.55):
    hs = nd.gaussian_filter(height, SS * blur)
    gy, gx = np.gradient(hs)
    lx, ly, lz = 0.0, -0.75, 0.62  # head on, so the cart's halves mirror (Roy)
    n = np.sqrt(lx * lx + ly * ly + lz * lz); lx, ly, lz = lx / n, ly / n, lz / n
    lam = (-gx * lx - gy * ly + lz) / np.sqrt(gx * gx + gy * gy + 1)
    occ = np.clip((nd.gaussian_filter(hs, 6 * SS) - hs) * 0.05, 0, 0.22)
    return (1 + (lam / lz - 1) * gain) * (1 - occ)
rng = np.random.default_rng(5)
def grain(s):
    g = nd.gaussian_filter(rng.normal(0, 1, (h, w)).astype(np.float32), 0.6 * SS)  # finer and fainter than before: smoother plastic (Roy 2026-10-24)
    return g / g.std() * s
def down(a):
    return np.asarray(Image.fromarray(a.astype(np.float32), 'F').resize((W, H), Image.Resampling.BOX))
def save(rgb, alpha, name):
    A = np.clip(down(alpha.astype(np.float32)), 0, 1)
    c = np.zeros((H, W, 4), np.uint8)
    for k in range(3):
        c[..., k] = np.clip(down(rgb[..., k] * alpha) / np.maximum(A, 1e-3), 0, 255).round().astype(np.uint8)
    c[..., 3] = (A * 255).round().astype(np.uint8)
    Image.fromarray(c, 'RGBA').save(f'{out}/{name}')
    return A
def grey3(g): return np.repeat(g[..., None], 3, 2)
# ---- silhouette ---- (measured on Roy's straight-on photo of a US Super Game Boy)
centre = rect(0.158, 0.0, 0.842, 1.0, 0.007)
cart = rect(0.0, 0.027, 1.0, 0.995, 0.014) | rect(0.158, 0.0, 0.842, 0.5, 0.007)
fr = int(0.006 * w)
fy, fx = np.mgrid[-fr:fr + 1, -fr:fr + 1]
cart = nd.binary_closing(np.pad(cart, fr), fx * fx + fy * fy <= fr * fr)[fr:-fr, fr:-fr]
dist = nd.distance_transform_edt(np.pad(cart, 1))[1:-1, 1:-1]
edge = np.clip(dist / (3.0 * SS), 0, 1) ** 0.5
yy, xx = np.mgrid[0:h, 0:w]
def gamebit(u, v, height):
    r = np.sqrt((xx - U(u)) ** 2 + (yy - V(v)) ** 2) / w
    th = np.arctan2(yy - V(v), xx - U(u))
    lobes = 0.0105 * (1 + 0.09 * np.cos(6 * th))
    sink = r < 0.017
    gap = (r < 0.0135) & (r >= lobes)
    head = r < lobes
    height -= sink * 0.8 + gap * 3.0
    height += head * np.sqrt(np.clip(1 - (r / 0.0115) ** 2, 0, 1)) * 0.8
    return gap

# ---- front ----
height = edge * 10
sides = cart & ~centre
height -= sides * 3.0
for v in (0.151, 0.258, 0.368, 0.481, 0.588, 0.693, 0.797, 0.903):  # measured on Roy's photo
    g = (rect(0.0, v - 0.005, 0.158, v + 0.005) | rect(0.842, v - 0.005, 1.0, v + 0.005)) & cart
    height -= g * 2.2
# the Game Boy bay: a recess open at the top, its floor ending in a smile-shaped lip; the dark
# mouth the Game Boy cart goes into sits just above that lip
u = (xx / w - 0.5) / 0.21
smile = 0.033 * (1 - np.clip(u, -1, 1) ** 2)       # the lip dips in the middle, as on the PAL cart (Roy)
bay = rect(0.29, -0.05, 0.71, 0.36, 0.02) & (yy / h < 0.298 + smile)
height -= bay * 2.6
for f in (0.25, 0.75):
    uc = 0.29 + f * 0.42
    height += (np.abs(xx - U(uc)) < 0.004 * w) * bay * (yy / h < 0.235 + smile * 0.8) * 1.2
mouth = bay & (yy / h < 0.298 + smile) & (yy / h > 0.235 + smile * 0.8)  # narrower (Roy)
# the black piece fills the bay from wall to wall down to the smile; where the grey floor above
# butts against it there's a seam, the two mouldings meeting (Roy)
seam = bay & ~mouth & nd.binary_dilation(mouth, iterations=int(1.0 * SS))
height -= seam * 1.0
label = rect(0.195, 0.405, 0.805, 0.672, 0.024)
height -= label * 2.0
window = rect(0.196, 0.708, 0.804, 1.05, 0.03)
height -= window * 4.5
pipe = rect(0.40, 0.65, 0.60, 1.05)
t = np.clip((yy - V(0.708)) / (V(0.96) - V(0.708)), 0, 1)
fade = 1 - t * t * (3 - 2 * t)
height -= pipe * window * fade * 4.0
gaps = (gamebit(0.079, 0.313, height) | gamebit(0.921, 0.313, height)
        | gamebit(0.079, 0.949, height) | gamebit(0.921, 0.949, height))
height *= cart
grey = 119 * light(height) + grain(1.3)
grey *= 1 - 0.06 * nd.gaussian_filter(label.astype(np.float32), SS)
grey *= 1 - 0.65 * nd.gaussian_filter(gaps.astype(np.float32), 0.5 * SS)
near = nd.distance_transform_edt(~centre) / w
grey *= 1 - sides * 0.42 * np.exp(-near / 0.014)
wall = nd.distance_transform_edt(np.pad(window, ((1, 0), (1, 1)), constant_values=False))[1:, 1:-1] / w
grey *= 1 - window * 0.30 * np.exp(-wall / 0.010)
side = nd.distance_transform_edt(pipe) / w
grey *= 1 - (pipe & window) * fade * 0.45 * np.exp(-side / 0.010)
# the bay's floor in the shade of its walls, the mouth nearly black, lit a touch at its far edge
# only the side walls shade the floor: the smile below the mouth is the same flat floor, ending
# in an upright rim like slot.'s Game Boy slot well, not a groove (Roy)
bw = np.minimum(np.abs(xx - U(0.29)), np.abs(xx - U(0.71))) / w
grey *= 1 - bay * 0.25 * np.exp(-bw / 0.012)
lip = nd.distance_transform_edt(np.pad(bay, ((0, 1), (0, 0)), constant_values=False))[:-1] / w
# the head-on light can't see upright ridges: their lit face and their shadow by hand
for fr in (0.25, 0.75):
    uc = 0.29 + fr * (0.71 - 0.29)
    on = bay & (yy / h < 0.235 + smile * 0.8) & (np.abs(xx - U(uc)) < 0.004 * w)
    beside = bay & (yy / h < 0.235 + smile * 0.8) & (xx >= U(uc) + 0.004 * w) & (xx < U(uc) + 0.009 * w)
    grey = np.where(on, grey * 1.10, grey)
    grey = np.where(beside, grey * 0.78, grey)
mo = mouth.astype(np.float32)
A = save(grey3(grey), cart, 'sgb_neutral.png')
m = np.zeros((H, W, 4), np.uint8); m[..., :3] = 255; m[..., 3] = (A * 255).round().astype(np.uint8)
Image.fromarray(m, 'RGBA').save(f'{out}/sgb_cart.png')
np.save(f'{out}/sgb_mouth.npy', down(mo))
print('ok')
