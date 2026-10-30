"""Match the band art's colour, shading and opacity to the shelf face it zooms from, keeping the
band's own sharp detail (Roy 2026-10-29: the colours jumped at the end of the zoom). Run after
sgb_band_hd.py. usage: sgb_band_match.py [DETAIL] [OUT]"""
import sys, numpy as np
from PIL import Image
from scipy import ndimage as nd
G = float(sys.argv[1]) if len(sys.argv) > 1 else 0.8
OUT = sys.argv[2] if len(sys.argv) > 2 else 'assets'
GEOM = {'pal': (123, 472, 448), 'sfc': (123, 472, 448), 'sgb2': (123, 472, 448), 'ntsc': (89, 543, 475)}
for m, (x, w, h) in GEOM.items():
    face = Image.open(f'assets/sgb_face_{m}.png').convert('RGBA').resize((w, h), Image.Resampling.BICUBIC)
    Fb = np.asarray(face).astype(np.float32)
    for part in ('front', 'back'):
        B = np.asarray(Image.open(f'assets/sgb_band_{m}_{part}.png').convert('RGBA')).astype(np.float32)
        F = np.zeros_like(B)
        F[6:87, x:x + w] = Fb[0:81]
        both = (B[..., 3] > 0) & (F[..., 3] > 0)
        out = B.copy()
        for k in range(3):
            # premultiplied blur so transparent edges do not bleed in
            def low(a, al):
                s = nd.gaussian_filter(a * al, 2.5); n = nd.gaussian_filter(al, 2.5)
                return s / np.maximum(n, 1e-3)
            ab, af = B[..., 3] / 255, F[..., 3] / 255
            lb, lf = low(B[..., k], ab), low(F[..., k], af)
            v = lf + (B[..., k] - lb) * G
            out[..., k] = np.where(both, v, B[..., k])
        # a clear shell lets as much through zoomed as on the shelf
        out[..., 3] = np.where(both, np.minimum(B[..., 3], F[..., 3]), B[..., 3])
        if m == 'sgb2':
            # The clear band is drawn as two layers, back and front. Together they must let
            # through just what the face does, or the band fades in thicker (Roy 2026-10-30).
            if part == 'front':
                front_a = out[..., 3] / 255
            else:
                fa = F[..., 3] / 255
                want = 1 - (1 - fa) / np.maximum(1 - front_a, 1e-3)
                out[..., 3] = np.where(both, np.clip(want, 0, 1) * 255, B[..., 3])
        Image.fromarray(np.clip(out, 0, 255).round().astype(np.uint8), 'RGBA').save(f'{OUT}/sgb_band_{m}_{part}.png', optimize=True)
    print(m)
