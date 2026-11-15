"""Make the clear Super Game Boy 2 plastic less see-through, as the clear carts and slots were
(Roy 2026-11-13): 40% of what shone through is closed. Soft outline pixels (alpha < 64) stay.

    python3 tools/sgb/carts/sgb2_less_clear.py assets/sgb_face_sgb2.png assets/sgb_zoom_sgb2.png \
        assets/sgb_band_sgb2_back.png assets/sgb_band_sgb2_front.png
"""
import sys

import numpy as np
from PIL import Image

KEEP = 0.6


def main(paths):
    for path in paths:
        im = np.asarray(Image.open(path).convert("RGBA")).copy()
        a = im[..., 3].astype(float)
        plastic = (a >= 64) & (a < 255)
        a[plastic] = 255 - (255 - a[plastic]) * KEEP
        im[..., 3] = np.clip(a.round(), 0, 255).astype(np.uint8)
        Image.fromarray(im, "RGBA").save(path)


if __name__ == "__main__":
    main(sys.argv[1:])
