"""Render the NES cart's inside: the back shell (50% grey, tinted by the game's colour) and the board."""
import sys, numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

out, font_path = sys.argv[1], sys.argv[2]
W, H, CW, SS = 312, 337, 302, 4
w, h = CW * SS, H * SS
cart = np.asarray(Image.open(f'{out}/nes_cart_render.png').split()[3].crop((5, 0, 5 + CW, H)).resize((w, h), Image.Resampling.BICUBIC)) > 127

def U(u): return u * w
def V(v): return v * h
def mask(fn):
    m = Image.new('L', (w, h), 0); fn(ImageDraw.Draw(m)); return np.asarray(m) > 127
def rect(u0, v0, u1, v1, r=0.0):
    return mask(lambda d: d.rounded_rectangle([U(u0), V(v0), U(u1), V(v1)], radius=r * w, fill=255))
def disc(u, v, r):
    return mask(lambda d: d.ellipse([U(u - r), V(v) - r * w, U(u + r), V(v) + r * w], fill=255))

def light(height, gain=1.5):
    hs = nd.gaussian_filter(height, SS * 0.55)
    gy, gx = np.gradient(hs)
    lx, ly, lz = -0.25, -0.75, 0.62
    n = np.sqrt(lx * lx + ly * ly + lz * lz); lx, ly, lz = lx / n, ly / n, lz / n
    lam = (-gx * lx - gy * ly + lz) / np.sqrt(gx * gx + gy * gy + 1)
    occ = np.clip((nd.gaussian_filter(hs, 6 * SS) - hs) * 0.05, 0, 0.25)
    return (1 + (lam / lz - 1) * gain) * (1 - occ)

def down(a):
    return np.asarray(Image.fromarray(a.astype(np.float32), 'F').resize((CW, H), Image.Resampling.BOX))
def save(rgb, alpha, name):
    A = np.clip(down(alpha.astype(np.float32)), 0, 1)
    c = np.zeros((H, W, 4), np.uint8)
    for k in range(3):
        c[:, 5:5 + CW, k] = np.clip(down(rgb[..., k] * alpha) / np.maximum(A, 1e-3), 0, 255).round().astype(np.uint8)
    c[:, 5:5 + CW, 3] = (A * 255).round().astype(np.uint8)
    Image.fromarray(c, 'RGBA').save(f'{out}/{name}')

rng = np.random.default_rng(11)
def grain(s=1.0):
    g = nd.gaussian_filter(rng.normal(0, 1, (h, w)).astype(np.float32), 0.6 * SS)
    return g / g.std() * s

# ---- back shell, seen from inside ----
shell = cart
height = np.zeros((h, w), np.float32)
wall = shell & ~nd.binary_erosion(shell, iterations=int(4.5 * SS))
height += wall * 7 + shell * 1
height += (rect(0.1675, -0.01, 0.305, 0.146, 0.004) & ~rect(0.19, -0.01, 0.282, 0.12, 0.003)) * 6
height += rect(0.19, -0.01, 0.282, 0.12) * 2.5
height += rect(0.62, 0.475, 0.97, 0.49) * 3
height += (rect(0.035, 0.62, 0.065, 0.84) | rect(0.905, 0.62, 0.94, 0.84)) * 4
for u, v in [(0.2425, 0.27), (0.605, 0.28), (0.40, 0.18)]:
    height += disc(u, v, 0.009) * 2.5
ring = disc(0.505, 0.444, 0.032); hole = disc(0.505, 0.444, 0.014)  # above the board (Roy's photo)
height += ring * 5 - hole * 9
height *= shell
grey = 128 * light(height) + grain(1.2)
grey = np.where(hole, grey * 0.55, grey)
save(np.repeat(grey[..., None], 3, 2), shell, 'nes_interior_render.png')

# ---- board ----
board = (rect(0.0475, 0.636, 0.93, 0.996, 0.006) | rect(0.505, 0.539, 0.8975, 0.66, 0.004))
for u in (0.0475, 0.93):
    board &= ~disc(u, 0.70, 0.013)
board &= cart | rect(0, 0.9, 1, 1.1)
GREEN, TRACE, VIA, SILK = np.array([2, 70, 34.]), np.array([0, 44, 18.]), np.array([196, 210, 190.]), np.array([225, 232, 222.])
NAVY, PIN, GOLD = np.array([4, 24, 44.]), np.array([196, 198, 200.]), np.array([150, 96, 30.])
rgb = np.zeros((h, w, 3), np.float32) + GREEN
img = Image.fromarray(np.zeros((h, w), np.uint8)); d = ImageDraw.Draw(img)
# traces: a few routed runs from the fingers up to the chips
ends = []
for k in range(36):
    x = U(0.0925 + (k + 0.5) * 0.8 / 36)
    top = V(0.86 - (k % 5) * 0.012)
    bend = x + ((k % 7) - 3) * 3 * SS
    d.line([(x, V(0.89)), (x, top), (bend, top - 6 * SS)], fill=255, width=int(1.5 * SS))
    ends.append((bend, top - 6 * SS))
for k in range(10):
    y = V(0.70 + k * 0.013)
    d.line([(U(0.16), y), (U(0.18), y)], fill=255, width=SS)
traces = np.asarray(img) > 127
rgb[traces] = TRACE
def chip(u0, v0, u1, v1, pins, vertical=False):
    global rgb
    box = rect(u0, v0, u1, v1, 0.003)
    silk = rect(u0 - 0.012, v0 - 0.012, u1 + 0.012, v1 + 0.012, 0.004) & ~rect(u0 - 0.008, v0 - 0.008, u1 + 0.008, v1 + 0.008, 0.004)
    rgb[silk] = SILK
    for k in range(pins):
        t = (k + 0.5) / pins
        if vertical:
            v = v0 + t * (v1 - v0)
            for u in (u0 - 0.008, u1 + 0.001):
                p = rect(u, v - 0.004, u + 0.007, v + 0.004); rgb[p] = PIN
        else:
            u = u0 + t * (u1 - u0)
            for v in (v0 - 0.007, v1 + 0.001):
                p = rect(u - 0.004, v, u + 0.004, v + 0.006); rgb[p] = PIN
    yy, xx = np.mgrid[0:h, 0:w]
    sheen = 1 + 0.25 * np.clip(1 - (yy - V(v0)) / max(V(v1) - V(v0), 1), 0, 1)
    rgb[box] = (NAVY[None, :] * sheen[box][:, None])
    notch = disc(u0 + 0.012, (v0 + v1) / 2, 0.006) if not vertical else disc((u0 + u1) / 2, v0 + 0.012, 0.006)
    rgb[notch & box] = NAVY * 0.6
chip(0.07, 0.69, 0.145, 0.855, 8, vertical=True)
chip(0.20, 0.735, 0.49, 0.83, 14)
chip(0.545, 0.735, 0.835, 0.83, 14)
chip(0.63, 0.585, 0.78, 0.63, 8)
cap = rect(0.505, 0.592, 0.53, 0.622, 0.006); rgb[cap] = [40, 110, 190]
for u, v in [(0.25, 0.68), (0.35, 0.68), (0.88, 0.60), (0.56, 0.665), (0.17, 0.875), (0.82, 0.70)]:
    via = disc(u, v, 0.0055); rgb[via] = VIA
vimg = Image.fromarray(np.zeros((h, w), np.uint8)); vd = ImageDraw.Draw(vimg)
for k, (x, y) in enumerate(ends):
    if k % 3 == 0 and not (V(0.72) < y < V(0.845)):
        vd.ellipse([x - 1.6 * SS, y - 1.6 * SS, x + 1.6 * SS, y + 1.6 * SS], fill=255)
rgb[np.asarray(vimg) > 127] = VIA
fingers = np.zeros((h, w), bool)
for k in range(36):
    u = 0.0925 + k * 0.8 / 36
    fingers |= rect(u + 0.002, 0.893, u + 0.8 / 36 - 0.003, 1.0, 0.002)
rgb[fingers] = GOLD
f = ImageFont.truetype(font_path, int(0.022 * w))
t = Image.fromarray(np.zeros((h, w), np.uint8)); td = ImageDraw.Draw(t)
td.text((U(0.53), V(0.545)), 'NES-SLOT-01', font=f, fill=255)
td.text((U(0.20), V(0.678)), '2026 Slot.', font=ImageFont.truetype(font_path, int(0.03 * w)), fill=255)
rgb[np.asarray(t) > 110] = SILK
shade = light(board.astype(np.float32) * 3 + fingers * -0.5 + traces * 0.3, gain=1.2)
rgb = rgb * shade[..., None] + grain(2.0)[..., None]
save(np.clip(rgb, 0, 255), board, 'nes_board_render.png')
print('ok')
