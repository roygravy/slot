"""Bake the band's teal backing into the clear Super Game Boy 2 face, once.

The zoom used to lay a teal rectangle under the face only while the camera moved, so the cart
turned teal in one frame (Roy). With the backing in the face itself the cart looks the same on
the shelf, in the studio and through the zoom.

    python3 tools/sgb/carts/sgb2_backing.py assets/sgb_face_sgb2.png
"""
import sys

import numpy as np
from PIL import Image

TEAL = np.array([0.09, 0.47, 0.52]) * 255
COVER = 0.3


def main(path):
    im = np.asarray(Image.open(path).convert("RGBA")).astype(float)
    rgb, a = im[..., :3], im[..., 3:] / 255
    inside = a > 0
    back = COVER * (1 - a)
    out_a = np.where(inside, a + back, 0)
    safe = np.where(out_a > 0, out_a, 1)
    out_rgb = (rgb * a + TEAL * back) / safe
    out = np.concatenate([out_rgb, out_a * 255], axis=-1)
    Image.fromarray(np.clip(out.round(), 0, 255).astype(np.uint8), "RGBA").save(path)


if __name__ == "__main__":
    main(sys.argv[1])
