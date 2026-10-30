"""Sink the Game Boy bay of the finished Super Game Boy faces, as the slot on the shelf does
(slot_chrome.rs draw_sgb_mouth): the bay darker than the top around it, a soft shade falling
from its rim. Run once on freshly rendered faces: python3 tools/sgb/carts/bay_shade.py"""
import numpy as np
from PIL import Image

DIM = 0.22            # SGB_MOUTH_DIM
SHADE_ALPHA = 0.7     # SGB_MOUTH_SHADE_ALPHA
SHADE_PX = 10.0       # SGB_MOUTH_SHADE (14 in the band) at the faces' smaller size
STEPS = 8

# (face, bay left, bay right, lip, smile centre, smile half width), from the renderers
FACES = {
    'ntsc': (0.29, 0.71, 0.298, 0.5, 0.21),
    'pal': (0.262, 0.744, 0.318, 0.503, 0.241),
    'sfc': (0.262, 0.744, 0.318, 0.503, 0.241),
    'sgb2': (0.262, 0.744, 0.318, 0.503, 0.241),
}

def shade(y):
    # The band's stacked steps: darkest under the rim, easing out to nothing.
    keep = np.ones_like(y)
    step = SHADE_PX / STEPS
    for k in range(STEPS):
        t = (STEPS - k) / STEPS
        a = SHADE_ALPHA * t * t / STEPS * 2.2
        keep *= np.where(y < step * (k + 1), 1 - a, 1)
    return keep

for name, (u0, u1, lip, uc, uw) in FACES.items():
    path = f'assets/sgb_face_{name}.png'
    im = np.asarray(Image.open(path).convert('RGBA')).astype(np.float64)
    h, w = im.shape[:2]
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float64)
    smile = 0.033 * (1 - np.clip((xx / w - uc) / uw, -1, 1) ** 2)
    bay = (xx / w >= u0) & (xx / w < u1) & (yy / h < lip + smile)
    top = np.argmax(im[:, :, 3] > 128, axis=0)[None, :]  # the cart's own top edge per column
    keep = np.where(bay, (1 - DIM) * shade(yy - top), 1.0)
    im[:, :, :3] *= keep[:, :, None]
    Image.fromarray(np.clip(im, 0, 255).round().astype(np.uint8)).save(path)
    print(path)
