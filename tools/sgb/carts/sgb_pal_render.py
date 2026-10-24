"""Render the PAL SNES / Super Famicom cart: front (50% grey), clear front, silhouette, inside of the back shell, board."""
import os, sys, numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

out, font_path = sys.argv[1], sys.argv[2]
SGB2 = len(sys.argv) > 3
W, H, SS = 400, 380, 4
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
def light(height, gain=1.5, blur=0.9):
    hs = nd.gaussian_filter(height, SS * blur)
    gy, gx = np.gradient(hs)
    lx, ly, lz = 0.0, -0.55, 0.81
    n = np.sqrt(lx * lx + ly * ly + lz * lz); lx, ly, lz = lx / n, ly / n, lz / n
    lam = (-gx * lx - gy * ly + lz) / np.sqrt(gx * gx + gy * gy + 1)
    occ = np.clip((nd.gaussian_filter(hs, 6 * SS) - hs) * 0.05, 0, 0.22)
    return (1 + (lam / lz - 1) * gain) * (1 - occ)
rng = np.random.default_rng(5)
def grain(s):
    g = nd.gaussian_filter(rng.normal(0, 1, (h, w)).astype(np.float32), 1.0 * SS)  # coarse enough to survive the shelf's scale (Roy)
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
    lx, ly, lz = 0.0, -0.75, 0.62  # head on, so the cart's halves mirror (Roy)
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
VENTS = [(0.072 + k * 0.045) * 271 / 380 for k in range(5)]   # the PAL game carts' vents, same size and pitch (Roy)   # measured on the Top Gear photo
VH = 0.022 * 271 / 380
# where each vent meets the side, the outline has a small rounded notch
for v in VENTS:
    for cx in (-0.002 * w, w + 0.002 * w):
        cart &= ~(((_x - cx) / (0.006 * w)) ** 2 + ((_y - V(v + VH / 2)) / (VH * h * 0.4)) ** 2 < 1)  # shallow (Roy 2026-10-23)
dist = nd.distance_transform_edt(np.pad(cart, 1))[1:-1, 1:-1]
height = np.sin(np.clip(dist / (4.0 * SS), 0, 1) * np.pi / 2) * 4
yy, xx = np.mgrid[0:h, 0:w]
# the side pieces are pillowed: they round off toward the left and right edges
# Seen from above the cart is a flat D at each end: the front stays flat across the middle and
# rolls round over the last part toward the left and right edge (Roy's top-view photo)
dx = np.minimum(xx - inset, w - 1 - inset - xx)
R = 0.085 * w
t = np.clip((R - dx) / R, 0, 1)                    # 0 where the front is flat, 1 at the very side
height -= np.where(cart, (R - np.sqrt(np.clip(R * R - (R * t) ** 2, 0, None))) / SS * 0.9, 0)

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

# ---- front (PAL / Super Famicom Super Game Boy, Roy's photos) ----
vent = np.zeros((h, w), bool)
for v in VENTS:
    vent |= (rect(-0.05, v, 0.040, v + VH, VH * h / w / 2) | rect(0.960, v, 1.05, v + VH, VH * h / w / 2))
height -= nd.gaussian_filter(vent.astype(np.float32), 0.5 * SS) * 1.1
# the top shell's lower edge stands a step proud of the bottom plate; toward the sides the step
# curves up a little and dies out before it reaches the edge (Roy's photo)
uo = np.abs(xx / w - 0.5)
SEAM_V, SEAM_END, SEAM_R = 0.669, 0.45, 1e-6       # straight (Roy)
_e = np.clip(uo - (SEAM_END - SEAM_R), 0, SEAM_R)
lift = (SEAM_R - np.sqrt(np.clip(SEAM_R ** 2 - _e ** 2, 0, None))) * w / h
STEP = (yy / h > SEAM_V - lift) & (uo < SEAM_END)
die = np.clip((SEAM_END - uo) / 0.07, 0, 1) ** 1.5   # straight, fading out toward its ends (Roy)
height -= STEP * die * 1.6
height -= ((np.abs(yy / h - (SEAM_V - lift)) < 0.002) & (uo < SEAM_END)) * die * 1.2
# the top shell's centre plate edges, and lower down the bottom plate's, set further in (Roy's photo)
for u in (0.158, 0.847):
    height -= ((np.abs(xx - U(u)) < 0.003 * w) & (yy / h < SEAM_V - lift)) * 2.4
for u in (0.167, 0.837):
    height -= ((np.abs(xx - U(u)) < 0.003 * w) & (yy / h > SEAM_V - lift)) * 2.4
# the lower piece: a seam right across, where the bottom plate meets the top shell
# the Game Boy bay, open at the top, its lip a shallow smile; the dark mouth just above the lip
uu = (xx / w - 0.503) / 0.241
smile = 0.033 * (1 - np.clip(uu, -1, 1) ** 2)  # the same smile as the NTSC cart (Roy)
bay = rect(0.262, -0.05, 0.744, 0.45, 0.03) & (yy / h < 0.318 + smile)
height -= bay * 2.6
# two upright ridges inside the bay, as in slot.'s Game Boy slot
for f in (0.25, 0.75):
    uc = 0.262 + f * (0.744 - 0.262)
    height += (np.abs(xx - U(uc)) < 0.004 * w) * bay * (yy / h < 0.255 + smile * 0.8) * 1.2
mouth = bay & (yy / h < 0.318 + smile) & (yy / h > 0.255 + smile * 0.8)  # narrower (Roy)
# the black piece fills the bay from wall to wall down to the smile; where the grey floor above
# butts against it there's a seam, the two mouldings meeting (Roy)
seam = bay & ~mouth & nd.binary_dilation(mouth, iterations=int(1.0 * SS))
height -= seam * 1.0
# the maker's badge, traced from the real logo (Roy): pressed into the PAL/SFC cart, ring and
# name alike, and lying on top of the SGB2's (Roy)
PILL = (0.403, 0.389, 0.597, 0.389 + 0.194 * w / h * 160 / 728)  # the logo's own proportions (Roy)
def logo_mask(name):
    if not os.path.exists(f'/tmp/claude-0/sgb/{name}'):
        print(f'no {name}: no badge')
        return np.zeros((h, w), bool)
    m = Image.open(f'/tmp/claude-0/sgb/{name}').resize((int(U(PILL[2]) - U(PILL[0])), int(V(PILL[3]) - V(PILL[1]))), Image.Resampling.LANCZOS)
    full = np.zeros((h, w), bool)
    full[int(V(PILL[1])):int(V(PILL[1])) + m.height, int(U(PILL[0])):int(U(PILL[0])) + m.width] = np.asarray(m) > 127
    return full
ring = logo_mask('nin_ring.png')
letters = logo_mask('nin_letters.png')
pill = nd.binary_fill_holes(ring)
relief = 0.9 if SGB2 else -0.9
height += ring * relief
height += letters * relief
uu2 = (xx / w - 0.5) / 0.236
label = rect(0.262, 0.46, 0.735, 0.626, 0.03) & (yy / h >= 0.471 + 0.023 * np.clip(uu2, -1, 1) ** 2)  # measured on Roy's PAL photo: the sticker fills it, its top edge arches
label = nd.binary_opening(label, iterations=int(2 * SS))
height -= label * 2.0
# the grip at the bottom: the NTSC cart's deep window with the half-pipe channel (Roy)
window = rect(0.232, 0.728, 0.768, 1.05, 0.03)
height -= window * 4.5
pipe = rect(0.40, 0.70, 0.60, 1.05)
tp = np.clip((yy - V(0.728)) / (V(0.97) - V(0.728)), 0, 1)
fade = 1 - tp * tp * (3 - 2 * tp)
height -= pipe * window * fade * 4.0
gaps = (gamebit(0.058, 0.288, height) | gamebit(0.942, 0.288, height)
        | gamebit(0.058, 0.929, height) | gamebit(0.942, 0.929, height))
if SGB2:  # a fifth screw on the right, just under the seam, beside the link port (Roy's photo)
    gaps |= gamebit(0.942, 0.70, height)
height *= cart
grey = 119 * crisp(height) + grain(2.8)
grey *= 1 - 0.06 * nd.gaussian_filter(label.astype(np.float32), SS)
grey *= 1 - (0.10 if SGB2 else 0.20) * nd.gaussian_filter(letters.astype(np.float32), 0.5 * SS)  # the raised name catches a little shade
grey *= 1 - 0.65 * nd.gaussian_filter(gaps.astype(np.float32), 0.5 * SS)
grey *= 1 - 0.20 * nd.gaussian_filter(vent.astype(np.float32), 0.4 * SS)
# only the side walls shade the floor: the smile below the mouth is the same flat floor, ending
# in an upright rim like slot.'s Game Boy slot well, not a groove (Roy)
bw = np.minimum(np.abs(xx - U(0.262)), np.abs(xx - U(0.744))) / w
grey *= 1 - bay * 0.25 * np.exp(-bw / 0.012)
lip = nd.distance_transform_edt(np.pad(bay, ((0, 1), (0, 0)), constant_values=False))[:-1] / w
# the head-on light can't see upright ridges: their lit face and their shadow by hand
for fr in (0.25, 0.75):
    uc = 0.262 + fr * (0.744 - 0.262)
    on = bay & (yy / h < 0.255 + smile * 0.8) & (np.abs(xx - U(uc)) < 0.004 * w)
    beside = bay & (yy / h < 0.255 + smile * 0.8) & (xx >= U(uc) + 0.004 * w) & (xx < U(uc) + 0.009 * w)
    grey = np.where(on, grey * 1.10, grey)
    grey = np.where(beside, grey * 0.78, grey)
ww = nd.distance_transform_edt(np.pad(window, ((1, 0), (1, 1)), constant_values=False))[1:, 1:-1] / w
grey *= 1 - window * 0.30 * np.exp(-ww / 0.010)
side = nd.distance_transform_edt(pipe) / w
grey *= 1 - (pipe & window) * fade * 0.45 * np.exp(-side / 0.010)
np.save(f'{out}/' + ('sgb2' if SGB2 else 'sgb_pal') + '_mouth.npy', down(mouth.astype(np.float32)))
np.save(f'{out}/' + ('sgb2' if SGB2 else 'sgb_pal') + '_label.npy', down(label.astype(np.float32)))  # the sticker's shape
turn = np.sqrt(np.clip(1 - t * t, 0, 1))
roll = -0.12 * (1 - turn) + 0.03 * np.exp(-((t - 0.35) / 0.18) ** 2)
grey *= 1 + np.where(cart & ~vent, roll, 0)
edge = (dist < 3 * SS) & (nd.gaussian_filter(vent.astype(np.float32), 1.5 * SS) < 0.05)
NAME = 'sgb2' if SGB2 else 'sgb_pal'
A = save(grey3(grey), cart, NAME + '_neutral.png')
G = np.asarray(Image.open(f'{out}/{NAME}_neutral.png'))[..., 0].astype(np.float32)
Ad = A > 0.5
rim = Ad & ~nd.binary_erosion(Ad, iterations=max(1, round(5 * F)))
clear = np.where(rim, 100, 128 + (G - 128) * 0.45)
c = np.zeros((H, W, 4), np.uint8); c[..., :3] = np.clip(clear, 0, 255).astype(np.uint8)[..., None]; c[..., 3] = (A * 255).round().astype(np.uint8)
Image.fromarray(c, 'RGBA').save(f'{out}/{NAME}_clear.png')
print('ok')
