import numpy as np, sys
sys.path.insert(0,'/tmp/claude-0/sgb')
from PIL import Image, ImageDraw, ImageFont
import colour as C
from stickers import sticker
R=C.R
def make(neutral, colour, style, rect, size, clear=False):
    o=C.tint(R+neutral, colour, None)
    im=Image.fromarray(np.clip(o,0,255).astype(np.uint8),'RGBA')
    # the mouth is black plastic whatever the shell's colour: a little lighter than black, its
    # top edge in shadow under the lip of the bay
    mo = np.load(R + neutral.replace('_neutral.png', '_mouth.npy').replace('_clear.png', '_mouth.npy'))
    a = np.asarray(im).astype(np.float32)
    rows = np.cumsum(mo > 0.5, axis=0)
    shade = np.clip(1 - 0.35 * np.exp(-rows / 4.0), 0, 1)
    # a soft sheen lower down the black moulding, and the grey piece's edge catching the light
    # just above the seam where the two mouldings meet (Roy)
    depth = rows / np.maximum(rows.max(axis=0, keepdims=True), 1)
    sheen = 1 + 0.18 * np.clip(depth, 0, 1) ** 2
    for k, v in enumerate((44, 44, 48)):
        a[..., k] = a[..., k] * (1 - mo) + v * shade * sheen * mo
    above = (np.roll(mo, -1, axis=0) > 0.3) & (mo < 0.3)
    a[..., :3] = np.where(above[..., None], np.clip(a[..., :3] * 1.12, 0, 255), a[..., :3])
    im = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), 'RGBA')
    if clear:
        a=np.asarray(im).copy(); a[...,3]=(a[...,3]*0.72).astype(np.uint8); im=Image.fromarray(a)
    if rect is None:
        # the sticker fills the recess moulded for it, a hair inside its edge (Roy)
        lm = np.load(R + neutral.replace('_neutral.png', '_label.npy').replace('_clear.png', '_label.npy'))
        from scipy import ndimage as nd
        lm = nd.gaussian_filter(nd.binary_erosion(lm > 0.5, iterations=1).astype(np.float32), 0.5)
        ys, xs = np.where(lm > 0.5); x, y, x1, y1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
        st = sticker((x1 - x, y1 - y), style)
        base = Image.new('RGBA', st.size, st.getpixel((st.width // 2, st.height - 3))[:3] + (255,))
        base.alpha_composite(Image.fromarray(np.where(np.asarray(st)[..., 3:] > 0, np.asarray(st), 0).astype(np.uint8)))
        a = np.asarray(base).copy(); a[..., 3] = (lm[y:y1, x:x1] * 255).astype(np.uint8)
        im.alpha_composite(Image.fromarray(a), (x, y))
    else:
        x,y,w,h=rect
        im.alpha_composite(sticker((w,h),style),(x,y))
    return im.resize(size,Image.Resampling.LANCZOS)
US=make('sgb_neutral.png',(134,134,138),'us',(82,145,236,86),(360,315))
PAL=make('sgb_pal_neutral.png',(198,198,201),'pal',None,(336,319))
SFC=make('sgb_pal_neutral.png',(198,198,201),'sfc',None,(336,319))
B2=make('sgb2_clear.png',(40,150,225),'sgb2',None,(336,319),clear=True)
# the link port: a clear housing standing out of the right side at the label's height, a dark
# socket in its face for the link cable (cosmetic, Roy)
_d=ImageDraw.Draw(B2); _y=int(0.555*319)
_d.rounded_rectangle((327,_y-13,336,_y+13),3,fill=(70,175,235,215),outline=(30,110,170,235))
_d.line((329,_y-11,329,_y+11),fill=(150,215,250,200))
_d.rounded_rectangle((331,_y-6,334,_y+6),1,fill=(20,40,60,235))
for n,im in [('sgb_us',US),('sgb_pal',PAL),('sgb_sfc',SFC),('sgb_2',B2)]: im.save(f'/tmp/claude-0/sgb/{n}.png')
F=ImageFont.truetype('/usr/share/fonts/opentype/inter/Inter-SemiBold.otf',20)
m=Image.new('RGBA',(4*370+30,400),(21,21,24,255)); d=ImageDraw.Draw(m)
for i,(t,im) in enumerate([('NTSC (VS)',US),('PAL',PAL),('Japans (SFC)',SFC),('SGB2, blauw doorzichtig',B2)]):
    x=15+i*370; d.text((x,14),t,font=F,fill=(235,235,240)); m.alpha_composite(im,(x+(360-im.width)//2,60))
m.save('/mnt/project-files/previews/super-game-boy/sgb-carts-v5.png',optimize=True)
