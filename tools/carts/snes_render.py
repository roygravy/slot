"""Render the NTSC SNES cart: front (50% grey), clear front, silhouette, inside of the back shell, board."""
import sys, numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

out, font_path = sys.argv[1], sys.argv[2]
W, H, SS = 400, 257, 4
w, h = W * SS, H * SS
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
# the raised centre only stands out at the top: the bottom is one straight edge (Roy's photo)
centre = rect(0.156, 0.0, 0.844, 1.0, 0.007)
cart = rect(0.0, 0.032, 1.0, 0.993, 0.014) | rect(0.156, 0.0, 0.844, 0.5, 0.007)
# a small fillet where the raised centre steps down to the side pieces, not a hard inner corner
fr = int(0.006 * w)
fy, fx = np.mgrid[-fr:fr + 1, -fr:fr + 1]
cart = nd.binary_closing(np.pad(cart, fr), fx * fx + fy * fy <= fr * fr)[fr:-fr, fr:-fr]
# padded, so the edges on the image border get the same bevel as the rest
dist = nd.distance_transform_edt(np.pad(cart, 1))[1:-1, 1:-1]
edge = np.clip(dist / (3.0 * SS), 0, 1) ** 0.5

def gamebit(u, v, height):
    """The security screw both SNES carts have (Roy's photo): a dark gap round a flush head
    with six round lobes, in a shallow countersink. Returns the gap, for its shadow."""
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
height -= sides * 3.0  # the raised centre stands well proud of the side pieces (Roy)
for v in (0.211, 0.367, 0.522, 0.681, 0.837):
    g = (rect(0.0, v - 0.006, 0.156, v + 0.006) | rect(0.844, v - 0.006, 1.0, v + 0.006)) & cart
    height -= g * 2.2
label = rect(0.195, -0.05, 0.809, 0.469, 0.012)
height -= label * 1.2
window = rect(0.194, 0.617, 0.809, 1.05, 0.03)
height -= window * 4.5  # deep, and runs out through the bottom edge (Roy's photo)
yy, xx = np.mgrid[0:h, 0:w]
# a raised tongue in the middle of the window, rising by a short ramp with slanted shoulders (Roy)
notch = mask(lambda d: d.polygon([(U(0.405), V(0.69)), (U(0.42), V(0.672)), (U(0.58), V(0.672)),
                                  (U(0.595), V(0.69)), (U(0.595), V(1.05)), (U(0.405), V(1.05))], fill=255))
ramp = np.clip((yy - V(0.672)) / (V(0.715) - V(0.672)), 0, 1)
height += notch * window * ramp * 3.0
gaps = gamebit(0.087, 0.917, height) | gamebit(0.913, 0.917, height)
height *= cart
hs = nd.gaussian_filter(height, SS * 0.55)
grey = 119 * light(height) + grain(1.2)  # the Game Boy carts' level (Roy)
grey *= 1 - 0.65 * nd.gaussian_filter(gaps.astype(np.float32), 0.5 * SS)
# the tall wall of the centre throws a soft shadow onto the side pieces, the same on both sides
near = nd.distance_transform_edt(~centre) / w
grey *= 1 - sides * 0.42 * np.exp(-near / 0.014)
# and the walls of the deep window and of its tongue shade the floor next to them
wall = nd.distance_transform_edt(np.pad(window, ((1, 0), (1, 1)), constant_values=False))[1:, 1:-1] / w
grey *= 1 - window * 0.30 * np.exp(-wall / 0.010)
tongue = nd.distance_transform_edt(~notch) / w
grey *= 1 - (window & ~notch) * 0.30 * np.exp(-tongue / 0.010)
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
wall &= ~rect(0.29, 0.95, 0.71, 1.1)
height += wall * 7 + shell * 1
for u0, u1 in ((0.04, 0.16), (0.84, 0.955)):
    height += (rect(u0, 0.06, u1, 0.075) | rect(u0 if u0 < 0.5 else u1 - 0.015, 0.06, (u0 + 0.015) if u0 < 0.5 else u1, 0.23)) * 6
for u0 in (0.035, 0.94):
    height += (rect(u0, 0.25, u0 + 0.025, 0.58) | rect(u0, 0.63, u0 + 0.025, 0.86)) * 5
    hu = (0.035, 0.15) if u0 < 0.5 else (0.85, 0.965)
    height += rect(hu[0], 0.60, hu[1], 0.615) * 4
for u in (0.092, 0.908):
    height += disc(u, 0.92, 0.03) * 6 - disc(u, 0.92, 0.012) * 9
for u in (0.226, 0.41, 0.595, 0.78):
    height -= rect(u - 0.01, 0.0, u + 0.01, 0.03) * 5
height *= shell
grey = 128 * light(height) + grain(1.2)
holes = disc(0.092, 0.92, 0.012) | disc(0.908, 0.92, 0.012)
grey = np.where(holes, grey * 0.55, grey)
save(grey3(grey), shell, 'snes_interior.png')

# ---- board ----
board = rect(0.134, 0.28, 0.871, 0.855, 0.006) | rect(0.293, 0.80, 0.725, 0.985, 0.003)
board &= ~(disc(0.134, 0.52, 0.012) | disc(0.871, 0.52, 0.012))
GREEN, TRACE, VIA, SILK = np.array([2, 70, 34.]), np.array([0, 44, 18.]), np.array([196, 210, 190.]), np.array([225, 232, 222.])
NAVY, PIN, GOLD = np.array([4, 24, 44.]), np.array([196, 198, 200.]), np.array([150, 96, 30.])
rgb = np.zeros((h, w, 3), np.float32) + GREEN
img = Image.new('L', (w, h), 0); d = ImageDraw.Draw(img)
N = 31
ends = []
for k in range(N):
    x = U(0.305 + (k + 0.5) * 0.41 / N)
    top = V(0.80 - (k % 6) * 0.025)
    bend = x + ((k % 9) - 4) * 4 * SS
    d.line([(x, V(0.87)), (x, top), (bend, top - 8 * SS)], fill=255, width=int(1.5 * SS))
    ends.append((bend, top - 8 * SS))
for k in range(8):
    y = V(0.33 + k * 0.025)
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
chip(0.19, 0.45, 0.53, 0.60, 20)
chip(0.60, 0.34, 0.80, 0.45, 9)
chip(0.60, 0.58, 0.76, 0.68, 8)
rgb[rect(0.56, 0.62, 0.575, 0.67, 0.004)] = [40, 110, 190]
vimg = Image.new('L', (w, h), 0); vd = ImageDraw.Draw(vimg)
for k, (x, y) in enumerate(ends):
    if k % 3 == 0:
        vd.ellipse([x - 1.6 * SS, y - 1.6 * SS, x + 1.6 * SS, y + 1.6 * SS], fill=255)
for u, v in [(0.16, 0.31), (0.84, 0.31), (0.16, 0.82), (0.84, 0.82), (0.57, 0.36), (0.83, 0.72)]:
    vd.ellipse([U(u) - 2 * SS, V(v) - 2 * SS, U(u) + 2 * SS, V(v) + 2 * SS], fill=255)
rgb[np.asarray(vimg) > 127] = VIA
fingers = np.zeros((h, w), bool)
for k in range(N):
    u = 0.305 + k * 0.41 / N
    fingers |= rect(u + 0.0015, 0.88, u + 0.41 / N - 0.002, 0.985, 0.0015)
rgb[fingers] = GOLD
t = Image.new('L', (w, h), 0); td = ImageDraw.Draw(t)
td.text((U(0.60), V(0.29)), 'SHVC-SLOT-01', font=ImageFont.truetype(font_path, int(0.018 * w)), fill=255)
td.text((U(0.19), V(0.66)), '2026 Slot.', font=ImageFont.truetype(font_path, int(0.022 * w)), fill=255)
rgb[np.asarray(t) > 110] = SILK
shade = light(board.astype(np.float32) * 3 + fingers * -0.5 + traces * 0.3, gain=1.2)
rgb = rgb * shade[..., None] + grain(2.0)[..., None]
save(np.clip(rgb, 0, 255), board & cart, 'snes_board.png')
print('ok')
