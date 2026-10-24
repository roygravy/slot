"""The Super Game Boy stickers, from the US cart's photo: its 'Super' and 'GAME BOY' lettering
lifted as shapes, then laid out and inked for the PAL, Super Famicom and SGB2 carts."""
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter
from scipy import ndimage as nd
D = '/tmp/claude-0/sgb/'
S = 4
photo = Image.open(D + 'st_us.png').convert('RGB')
a = np.asarray(photo).astype(int)
r, g, b = a[..., 0], a[..., 1], a[..., 2]
yy, xx = np.mgrid[0:a.shape[0], 0:a.shape[1]]
inside = (xx > 45) & (xx < 1515) & (yy > 20) & (yy < 515)
red = (r > 150) & (g < 130) & (b < 120) & inside
blue = (b > 110) & (r < 130) & (g < 130) & inside & (yy > 262)
red = nd.binary_opening(red, iterations=2); blue = nd.binary_opening(blue, iterations=2)
def crop_mask(m):
    ys, xs = np.where(m)
    return Image.fromarray((m[ys.min():ys.max() + 1, xs.min():xs.max() + 1] * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.2))
SUPER, GB = crop_mask(red), crop_mask(blue)
# the US sticker itself: straight from the photo, inside its own rounded edge
def us(size):
    st = photo.crop((40, 18, 1522, 518)).resize(size, Image.Resampling.LANCZOS).convert('RGBA')
    m = Image.new('L', (size[0] * S, size[1] * S), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, size[0] * S - 1, size[1] * S - 1), radius=7 * S, fill=255)  # the real sticker's corners: ~3% of its width (Roy)
    st.putalpha(m.resize(size, Image.Resampling.LANCZOS))
    return st
def ink(canvas, mask, box, colour):
    x0, y0, x1, y1 = box
    w = x1 - x0; h = int(w * mask.height / mask.width)
    if h > y1 - y0:
        h = y1 - y0; w = int(h * mask.width / mask.height)
    m = mask.resize((w, h), Image.Resampling.LANCZOS)
    layer = Image.new('RGBA', m.size, colour + (255,)); layer.putalpha(m)
    canvas.alpha_composite(layer, (x0, y0))
def plate(size, fill):
    W, H = size[0] * S, size[1] * S
    im = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    m = np.zeros((H, W), bool)
    Y, X = np.mgrid[0:H, 0:W]
    u = (X / W - 0.5) / 0.5
    top = (0.13 * (u ** 2)) * H                     # the top edge arches up in the middle
    m = (Y >= top) & (Y < H)
    # round the corners: an opening with a disc (a square one leaves them sharp), about 4% of
    # the width, as on Roy's photos (Roy)
    r = int(0.04 * W)
    disc = np.hypot(*np.mgrid[-r:r + 1, -r:r + 1]) <= r
    m = np.pad(m, ((r + 1, 0), (0, 0)))  # the bottom is the sticker's edge too
    m = np.pad(m, ((0, r + 1), (r + 1, r + 1)))
    m = nd.binary_opening(m, structure=disc)[r + 1:-(r + 1), r + 1:-(r + 1)]
    mk = Image.fromarray((m * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(S * 0.6))
    fillim = Image.new('RGBA', (W, H), fill + (255,)); fillim.putalpha(mk)
    return fillim, (W, H)
def nes_dots(d, cx, cy, s, colour):
    for (dx, dy, rx, ry) in ((-0.05, -0.30, 0.42, 0.34), (0.55, -0.02, 0.40, 0.34), (-0.55, 0.12, 0.42, 0.28), (0.15, 0.42, 0.45, 0.28)):
        x, y = cx + dx * s, cy + dy * s
        d.ellipse((x - rx * s, y - ry * s, x + rx * s, y + ry * s), fill=colour)
def pal_like(size, style):
    if style == 'sgb2':
        im, (W, H) = plate(size, (64, 44, 138)); colour = (240, 240, 246)
    else:
        im, (W, H) = plate(size, (206, 206, 206)); colour = (34, 34, 38)
    if style == 'sgb2':
        ink(im, SUPER, (int(W * 0.14), int(H * 0.12), int(W * 0.58), int(H * 0.44)), colour)
        ink(im, GB, (int(W * 0.12), int(H * 0.45), int(W * 0.80), int(H * 0.82)), colour)
        f = ImageFont.truetype('/usr/share/fonts/opentype/inter/Inter-ExtraBoldItalic.otf', int(H * 0.72))
        ImageDraw.Draw(im).text((W * 0.81, H * 0.84), '2', font=f, fill=colour + (255,), anchor='ls')
    else:
        ink(im, SUPER, (int(W * 0.15), int(H * 0.13), int(W * 0.58), int(H * 0.42)), colour)
        ink(im, GB, (int(W * 0.09), int(H * 0.45), int(W * 0.63), int(H * 0.82)), colour)
        d = ImageDraw.Draw(im)
        if style == 'pal':
            nes_dots(d, W * 0.82, H * 0.34, H * 0.23, colour + (255,))
            # SUPER NES in a narrow italic, as on the real sticker: squeezed Inter
            f = ImageFont.truetype('/usr/share/fonts/opentype/inter/Inter-BoldItalic.otf', int(H * 0.22))
            tl = Image.new('L', (int(W * 0.5), int(H * 0.3)), 0)
            ImageDraw.Draw(tl).text((tl.width / 2, tl.height * 0.8), 'SUPER NES', font=f, fill=255, anchor='ms')
            tl = tl.resize((int(tl.width * 0.78), tl.height), Image.Resampling.LANCZOS)
            lay = Image.new('RGBA', tl.size, colour + (255,)); lay.putalpha(tl)
            im.alpha_composite(lay, (int(W * 0.83 - tl.width / 2), int(H * 0.80 - tl.height * 0.8)))
        else:  # the Super Famicom seal: a dark disc ringed by its lettering, the four ovals inside
            cx, cy, rr = W * 0.82, H * 0.55, H * 0.30
            d.ellipse((cx - rr, cy - rr, cx + rr, cy + rr), outline=colour + (255,), width=max(2, int(S * 0.6)))
            d.ellipse((cx - rr * 0.68, cy - rr * 0.68, cx + rr * 0.68, cy + rr * 0.68), fill=colour + (255,))
            nes_dots(d, cx, cy, rr * 0.42, (206, 206, 206, 255))
    return im.resize(size, Image.Resampling.LANCZOS)
# Roy's photos of the PAL and SGB2 carts: their stickers' artwork lifted as it is (Roy: just copy
# them), the ink as a soft mask over the sticker's own flat colour
def photo_ink(path, box, ink_box, level):
    a = np.asarray(Image.open(path).convert('RGB')).astype(np.float32)
    x0, y0, x1, y1 = box
    m = level(a[y0:y1, x0:x1])
    ix0, iy0, ix1, iy1 = ink_box  # the artwork only: the sticker's edge stays out of it
    keep = np.zeros_like(m); keep[iy0 - y0:iy1 - y0, ix0 - x0:ix1 - x0] = 1
    return Image.fromarray((np.clip(m, 0, 1) * keep * 255).astype(np.uint8))
PAL_INK = photo_ink('/mnt/project-files/super-game-boy/fotos/ref/sgb-pal-konsolenkost.jpg', (227, 373, 573, 484),
                    (258, 393, 552, 468), lambda a: (172 - a.mean(axis=2)) / (172 - 72))
SGB2_INK = photo_ink('/mnt/project-files/super-game-boy/fotos/ref/sgb2-blauw-recht.jpg', (388, 752, 1012, 937),
                     (465, 755, 1002, 912), lambda a: (a[..., 1] - 45) / (175 - 45))
def copied(size, style):
    if style == 'sgb2':
        im, (W, H) = plate(size, (52, 36, 126)); colour = (238, 222, 236); m = SGB2_INK
    else:
        im, (W, H) = plate(size, (206, 206, 206)); colour = (34, 34, 38); m = PAL_INK
    # one scale both ways, so the lettering keeps its own proportions on a flatter sticker
    sw = int(m.width * H / m.height)
    m = m.resize((sw, H), Image.Resampling.LANCZOS)
    full = Image.new('L', (W, H), 0); full.paste(m, ((W - sw) // 2, 0))
    layer = Image.new('RGBA', (W, H), colour + (255,))
    layer.putalpha(Image.fromarray((np.asarray(full).astype(np.float32) * np.asarray(im)[..., 3] / 255).astype(np.uint8)))
    im.alpha_composite(layer)
    return im.resize(size, Image.Resampling.LANCZOS)
def sticker(size, style):
    if style in ('pal', 'sgb2'):
        return copied(size, style)
    return us(size) if style == 'us' else pal_like(size, style)
if __name__ == '__main__':
    m = Image.new('RGBA', (4 * 260, 120), (120, 120, 124, 255))
    for i, st in enumerate(['us', 'pal', 'sfc', 'sgb2']):
        m.alpha_composite(sticker((240, 80), st), (10 + i * 260, 20))
    m.resize((m.width * 2, m.height * 2)).save(D + 'stickers.png')
