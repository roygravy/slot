import numpy as np, sys
from PIL import Image, ImageDraw, ImageFont
R='/tmp/claude-0/sgb/r/'
def overlay(b,c):
    b=b/255; c=c/255
    return np.where(b<0.5, 2*b*c, 1-2*(1-b)*(1-c))*255
def tint(neutral, rgb, out):
    n=np.asarray(Image.open(neutral)).astype(np.float32)
    o=n.copy()
    for k in range(3): o[...,k]=overlay(n[...,0], rgb[k])
    return o
def sticker(size, style):
    W,H=size; S=4
    im=Image.new('RGBA',(W*S,H*S),(0,0,0,0)); d=ImageDraw.Draw(im)
    bg={'us':(232,232,236),'pal':(210,210,212),'sgb2':(64,52,140)}[style]
    d.rounded_rectangle([0,0,W*S-1,H*S-1],radius=6*S,fill=bg+(255,))
    red,blue={'us':((214,48,40),(48,48,160)),'pal':((30,30,34),(30,30,34)),'sgb2':((232,232,240),(232,232,240))}[style]
    f1=ImageFont.truetype('/usr/share/fonts/opentype/inter/InterDisplay-MediumItalic.otf',int(H*S*0.40))
    f2=ImageFont.truetype('/usr/share/fonts/opentype/inter/Inter-BoldItalic.otf',int(H*S*0.40))
    t2='GAME BOY'+(' 2' if style=='sgb2' else '')
    w2=d.textlength(t2,font=f2)
    x0=(W*S-w2)/2
    d.text((x0+W*S*0.05,H*S*0.04),'Super',font=f1,fill=red)
    d.line([(x0+W*S*0.33,H*S*0.42),(x0+w2*0.95,H*S*0.30)],fill=red,width=int(1.6*S))
    d.text((x0,H*S*0.44),t2,font=f2,fill=blue)
    return im.resize((W,H),Image.Resampling.LANCZOS)
def make(colour, style, out, clear=False):
    o=tint(R+'sgb_neutral.png', colour, out)
    im=Image.fromarray(np.clip(o,0,255).astype(np.uint8),'RGBA')
    if clear:
        a=np.asarray(im).copy(); a[...,3]=(a[...,3]*0.80).astype(np.uint8); im=Image.fromarray(a)
    # sticker rect in render px (400x350): u .205..795, v .415..662
    st=sticker((int(0.59*400),int(0.247*350)),style)
    im.alpha_composite(st,(int(0.205*400),int(0.415*350)))
    im=im.resize((360,315),Image.Resampling.LANCZOS)
    im.save(out)
make((134,134,138),'us','/tmp/claude-0/sgb/sgb_us.png')
make((198,198,201),'pal','/tmp/claude-0/sgb/sgb_pal.png')
make((40,140,215),'sgb2','/tmp/claude-0/sgb/sgb_2.png',clear=True)
m=Image.new('RGBA',(3*380,330),(40,40,44,255))
for i,n in enumerate(['sgb_us','sgb_pal','sgb_2']): m.alpha_composite(Image.open(f'/tmp/claude-0/sgb/{n}.png'),(10+i*375,8))
m.save('/tmp/claude-0/sgb/variants.png')
