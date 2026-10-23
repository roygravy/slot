"""Render the PAL SNES / Super Famicom cart: front (50% grey), clear front, silhouette, inside of the back shell, board."""
import sys, numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

out, font_path = sys.argv[1], sys.argv[2]
W, H, SS = 400, 271, 4    # 127 x 86 mm
w, h = W * SS, H * SS
def U(u): return u * w
def V(v): return v * h
def mask(fn):
    m = Image.new('L', (w, h), 0); fn(ImageDraw.Draw(m)); return np.asarray(m) > 127
def rect(u0, v0, u1, v1, r=0.0):
    return mask(lambda d: d.rounded_rectangle([U(u0), V(v0), U(u1), V(v1)], radius=r * w, fill=255))
def disc(u, v, r):
    return mask(lambda d: d.ellipse([U(u) - r * w, V(v) - r * w, U(u) + r * w, V(v) + r * w], fill=255))
def light(height, gain=1.5, blur=0.9):
    hs = nd.gaussian_filter(height, SS * blur)
    gy, gx = np.gradient(hs)
    lx, ly, lz = -0.2, -0.55, 0.81
    n = np.sqrt(lx * lx + ly * ly + lz * lz); lx, ly, lz = lx / n, ly / n, lz / n
    lam = (-gx * lx - gy * ly + lz) / np.sqrt(gx * gx + gy * gy + 1)
    occ = np.clip((nd.gaussian_filter(hs, 6 * SS) - hs) * 0.05, 0, 0.22)
    return (1 + (lam / lz - 1) * gain) * (1 - occ)
rng = np.random.default_rng(5)
def grain(s):
    g = nd.gaussian_filter(rng.normal(0, 1, (h, w)).astype(np.float32), 0.6 * SS)
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

# ---- silhouette ----
# Measured from Roy's straight-on photo of a blank PAL cart (2026-10-22) and drawn with the same
# recipe as the NTSC, GB and NES renders: a rounded moulded edge, light from above, fine grain.
def crisp(height, gain=1.7):
    hs = nd.gaussian_filter(height, SS * 0.55)
    gy, gx = np.gradient(hs)
    lx, ly, lz = -0.25, -0.75, 0.62
    n = np.sqrt(lx * lx + ly * ly + lz * lz); lx, ly, lz = lx / n, ly / n, lz / n
    lam = (-gx * lx - gy * ly + lz) / np.sqrt(gx * gx + gy * gy + 1)
    occ = np.clip((nd.gaussian_filter(hs, 6 * SS) - hs) * 0.05, 0, 0.22)
    return (1 + (lam / lz - 1) * gain) * (1 - occ)
def round_rect(u0, v0, u1, v1, r_top, r_bot):
    """A rectangle with bigger top corners than bottom ones."""
    return (rect(u0, v0, u1, v1 - (v1 - v0) / 2, r_top) | rect(u0, v0 + (v1 - v0) / 2, u1, v1, r_bot)
            | rect(u0, v0 + 0.1, u1, v1 - 0.1))
BULGE = 0.0
cart = round_rect(0.0, 0.0, 1.0, 0.996, 0.01, 0.015)  # bottom a bit less round (Roy 2026-10-23)
# the left and right sides bow out a little, like a flat D: widest halfway, pulled in at the ends
_y, _x = np.mgrid[0:h, 0:w]
_v = _y / h
inset = BULGE * w * (1 - np.sin(np.clip(_v / 0.996, 0, 1) * np.pi)) ** 1.6
cart &= (_x >= inset) & (_x <= w - 1 - inset)
# small notches in the top edge where the seams beside the middle plate meet it (all the photos)
for u in (0.161, 0.841):
    cart &= ~(rect(u - 0.004, -0.05, u + 0.004, 0.005, 0.002))
VENTS = [0.072 + k * 0.045 for k in range(5)]   # measured on the Top Gear photo
VH = 0.022
# where each vent meets the side, the outline has a small rounded notch
for v in VENTS:
    for cx in (-0.002 * w, w + 0.002 * w):
        cart &= ~(((_x - cx) / (0.006 * w)) ** 2 + ((_y - V(v + VH / 2)) / (VH * h * 0.4)) ** 2 < 1)  # shallow (Roy 2026-10-23)
dist = nd.distance_transform_edt(np.pad(cart, 1))[1:-1, 1:-1]
height = np.sin(np.clip(dist / (4.0 * SS), 0, 1) * np.pi / 2) * 10
yy, xx = np.mgrid[0:h, 0:w]
# the side pieces are pillowed: they round off toward the left and right edges
# Seen from above the cart is a flat D at each end: the front stays flat across the middle and
# rolls round over the last part toward the left and right edge (Roy's top-view photo)
dx = np.minimum(xx - inset, w - 1 - inset - xx)
R = 0.085 * w
t = np.clip((R - dx) / R, 0, 1)                    # 0 where the front is flat, 1 at the very side
height -= np.where(cart, (R - np.sqrt(np.clip(R * R - (R * t) ** 2, 0, None))) / SS * 0.9, 0)

# ---- front ----
# vents: five soft rounded grooves running in from each side, and where each one meets the side
# the outline itself has a small rounded notch (Roy's Top Gear close-up)
vent = np.zeros((h, w), bool)
for v in VENTS:
    band = rect(-0.05, v, 1.05, v + VH, VH * h / w / 2)
    vent |= (rect(-0.05, v, 0.040, v + VH, VH * h / w / 2) | rect(0.960, v, 1.05, v + VH, VH * h / w / 2))
vd = nd.distance_transform_edt(np.pad(vent, 1))[1:-1, 1:-1]
height -= nd.gaussian_filter(vent.astype(np.float32), 0.5 * SS) * 1.1  # shallower (Roy 2026-10-23)
# the label recess: deep, with big rounded corners
label = rect(0.0818, 0.0504, 0.9152, 0.4891, 0.0342)  # the sticker's rect + 1.5px all round, radius r10 + 1.5: concentric (Roy)
height -= label * 2.0
# the middle plate below the label and the side pieces either side; a fine seam marks the plate's
# edges all the way up to the top rim (no step across the side pieces: Roy)
for u in (0.161, 0.841):
    seam = (np.abs(xx - U(u)) < 0.0022 * w) & ~label
    height -= seam * 1.4

# the grip: a long pill-shaped well inside a soft, slightly raised flat lip that rounds over
# into it; four ribs run along its floor, the middle segment a step deeper (Roy's photo)
slot = rect(0.209, 0.634, 0.814, 0.696, 0.024)
rim = rect(0.199, 0.621, 0.824, 0.709, 0.033)
rim_d = nd.distance_transform_edt(np.pad(rim, 1))[1:-1, 1:-1]
height += np.sin(np.clip(rim_d / (2.0 * SS), 0, 1) * np.pi / 2) * 0.5
slot_in = nd.distance_transform_edt(np.pad(slot, 1))[1:-1, 1:-1]
height -= np.sin(np.clip(slot_in / (2.2 * SS), 0, 1) * np.pi / 2) * 3.4
mid = rect(0.397, 0.62, 0.596, 0.72) & slot
height -= mid * 0.6
period = V(0.696 - 0.634) / 4.5
ribs = 0.5 + 0.5 * np.cos((yy - V(0.634)) / period * 2 * np.pi)
height += slot * ribs * 0.9 * np.clip(slot_in / (1.5 * SS), 0, 1)
# screws: a deep round well with the screw head, its hole off to the upper right
# real carts: a small screw head sitting in a shallow countersink
wells = disc(0.063, 0.929, 0.019) | disc(0.937, 0.929, 0.019)
holes = disc(0.063, 0.929, 0.011) | disc(0.937, 0.929, 0.011)
height -= wells * 2.6
# the screw head is domed, so it catches the light on one side (Roy: it read faint)
for u in (0.063, 0.937):
    rr = np.sqrt((xx - U(u)) ** 2 + (yy - V(0.929)) ** 2) / (0.011 * w)
    height += np.sqrt(np.clip(1 - rr * rr, 0, 1)) * 2.4
height *= cart
# the flat face sits at the Game Boy carts' level, so the same colour reads the same (Roy)
grey = 119 * crisp(height) + grain(1.2)
# the recess and the wells catch less light than the open plastic
grey *= 1 - 0.06 * nd.gaussian_filter(label.astype(np.float32), SS)
grey *= 1 - 0.5 * nd.gaussian_filter((wells & ~holes).astype(np.float32), 0.6 * SS)
grey *= 1 - 0.20 * nd.gaussian_filter(slot.astype(np.float32), 0.8 * SS) * (1 + 0.08 * mid)
grey = np.where(holes, grey * 0.8, grey)
# the side vents are cut deep: their floor sits in shadow
grey *= 1 - 0.20 * nd.gaussian_filter(vent.astype(np.float32), 0.4 * SS)
# the rounded ends turn away from the light: a soft sheen where they start to curve, then the
# plastic darkens as it turns toward the side
turn = np.sqrt(np.clip(1 - t * t, 0, 1))
roll = -0.12 * (1 - turn) + 0.03 * np.exp(-((t - 0.35) / 0.18) ** 2)
grey *= 1 + np.where(cart & ~vent, roll, 0)
# the outer edge stays light, as on the photo, instead of reading as a dark outline
edge = (dist < 3 * SS) & (nd.gaussian_filter(vent.astype(np.float32), 1.5 * SS) < 0.05)
grey = np.where(edge & (dx > R), np.maximum(grey, 119 * 0.86), grey)
# the notched side edges must not read as dark arcs around each vent
grey = np.where((dist < 3 * SS) & (dx <= R), np.maximum(grey, 119 * 0.80), grey)
A = save(grey3(grey), cart, 'snes_neutral.png')
m = np.zeros((H, W, 4), np.uint8); m[..., :3] = 255; m[..., 3] = (A * 255).round().astype(np.uint8)
Image.fromarray(m, 'RGBA').save(f'{out}/snes_cart.png')
G = np.asarray(Image.open(f'{out}/snes_neutral.png'))[..., 0].astype(np.float32)
Ad = A > 0.5
rim = Ad & ~nd.binary_erosion(Ad, iterations=5)
clear = np.where(rim, 100, 128 + (G - 128) * 0.45)
c = np.zeros((H, W, 4), np.uint8); c[..., :3] = np.clip(clear, 0, 255).astype(np.uint8)[..., None]; c[..., 3] = m[..., 3]
Image.fromarray(c, 'RGBA').save(f'{out}/snes_clear.png')

# ---- inside of the back shell ----
shell = cart
height = np.zeros((h, w), np.float32)
wall = shell & ~nd.binary_erosion(shell, iterations=int(4.5 * SS))
wall &= ~rect(0.26, 0.95, 0.74, 1.1)
height += wall * 7 + shell * 1
for u in (0.2, 0.42, 0.62, 0.8):
    height += rect(u - 0.006, 0.02, u + 0.006, 0.13) * 5
for v in (0.3, 0.55):
    height += (rect(0.02, v - 0.008, 0.08, v + 0.008) | rect(0.92, v - 0.008, 0.98, v + 0.008)) * 5
height += (rect(0.88, 0.62, 0.97, 0.64) | rect(0.88, 0.62, 0.895, 0.86)) * 5
height += (rect(0.03, 0.62, 0.12, 0.64) | rect(0.105, 0.62, 0.12, 0.86)) * 5
for u in (0.06, 0.94):
    height += disc(u, 0.9, 0.032) * 6 - disc(u, 0.9, 0.013) * 9
for u, v in ((0.62, 0.33), (0.62, 0.66)):
    height -= disc(u, v, 0.025) * 0.6
height *= shell
grey = 128 * light(height) + grain(1.2)
holes = disc(0.06, 0.9, 0.013) | disc(0.94, 0.9, 0.013)
grey = np.where(holes, grey * 0.55, grey)
save(grey3(grey), shell, 'snes_interior.png')

# ---- board ----
board = rect(0.13, 0.36, 0.87, 0.87, 0.006) | rect(0.27, 0.82, 0.73, 0.985, 0.003)
board &= ~(disc(0.13, 0.6, 0.012) | disc(0.87, 0.6, 0.012))
GREEN, TRACE, VIA, SILK = np.array([2, 70, 34.]), np.array([0, 44, 18.]), np.array([196, 210, 190.]), np.array([225, 232, 222.])
NAVY, PIN, GOLD = np.array([4, 24, 44.]), np.array([196, 198, 200.]), np.array([150, 96, 30.])
rgb = np.zeros((h, w, 3), np.float32) + GREEN
img = Image.new('L', (w, h), 0); d = ImageDraw.Draw(img)
N = 31
ends = []
for k in range(N):
    x = U(0.285 + (k + 0.5) * 0.43 / N)
    top = V(0.80 - (k % 6) * 0.025)
    bend = x + ((k % 9) - 4) * 4 * SS
    d.line([(x, V(0.87)), (x, top), (bend, top - 8 * SS)], fill=255, width=int(1.5 * SS))
    ends.append((bend, top - 8 * SS))
for k in range(8):
    y = V(0.40 + k * 0.022)
    d.line([(U(0.17), y), (U(0.56), y)], fill=255, width=int(1.2 * SS))
traces = np.asarray(img) > 127
rgb[traces] = TRACE
def chip(u0, v0, u1, v1, pins):
    box = rect(u0, v0, u1, v1, 0.003)
    silk = rect(u0 - 0.009, v0 - 0.016, u1 + 0.009, v1 + 0.016, 0.004) & ~rect(u0 - 0.006, v0 - 0.011, u1 + 0.006, v1 + 0.011, 0.004)
    rgb[silk] = SILK
    for k in range(pins):
        u = u0 + (k + 0.5) / pins * (u1 - u0)
        for v in (v0 - 0.01, v1 + 0.001):
            rgb[rect(u - 0.003, v, u + 0.003, v + 0.009)] = PIN
    sheen = 1 + 0.25 * np.clip(1 - (yy - V(v0)) / max(V(v1) - V(v0), 1), 0, 1)
    rgb[box] = NAVY[None, :] * sheen[box][:, None]
    rgb[disc(u0 + 0.01, (v0 + v1) / 2, 0.004) & box] = NAVY * 0.6
chip(0.19, 0.50, 0.53, 0.64, 20)
chip(0.60, 0.42, 0.80, 0.52, 9)
chip(0.60, 0.62, 0.76, 0.71, 8)
rgb[rect(0.56, 0.66, 0.575, 0.71, 0.004)] = [40, 110, 190]
vimg = Image.new('L', (w, h), 0); vd = ImageDraw.Draw(vimg)
for k, (x, y) in enumerate(ends):
    if k % 3 == 0:
        vd.ellipse([x - 1.6 * SS, y - 1.6 * SS, x + 1.6 * SS, y + 1.6 * SS], fill=255)
for u, v in [(0.16, 0.39), (0.84, 0.39), (0.16, 0.84), (0.84, 0.84), (0.57, 0.44), (0.83, 0.76)]:
    vd.ellipse([U(u) - 2 * SS, V(v) - 2 * SS, U(u) + 2 * SS, V(v) + 2 * SS], fill=255)
rgb[np.asarray(vimg) > 127] = VIA
fingers = np.zeros((h, w), bool)
for k in range(N):
    u = 0.285 + k * 0.43 / N
    fingers |= rect(u + 0.0015, 0.885, u + 0.43 / N - 0.002, 0.985, 0.0015)
rgb[fingers] = GOLD
t = Image.new('L', (w, h), 0); td = ImageDraw.Draw(t)
td.text((U(0.60), V(0.37)), 'SNSP-SLOT-01', font=ImageFont.truetype(font_path, int(0.018 * w)), fill=255)
td.text((U(0.19), V(0.70)), '2026 Slot.', font=ImageFont.truetype(font_path, int(0.022 * w)), fill=255)
rgb[np.asarray(t) > 110] = SILK
shade = light(board.astype(np.float32) * 3 + fingers * -0.5 + traces * 0.3, gain=1.2)
rgb = rgb * shade[..., None] + grain(2.0)[..., None]
save(np.clip(rgb, 0, 255), board & cart, 'snes_board.png')
print('ok')
