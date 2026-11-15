"""The slot. x GRAVY border in the Super Game Boy style, drawn as real pixel art: 256x224,
a handful of flat colours, hard 1px edges, no anti-aliasing anywhere."""
import numpy as np
from PIL import Image, ImageDraw, ImageFont
import sys
OUT=sys.argv[1]  # usage: gravy_border.py OUT.png [2], then border_bin.py OUT.png assets/sgb_border_gravy[2].bin
SGB2=len(sys.argv)>2  # the Super Game Boy 2's: the same layout in Super Famicom grey
BG=(0xC0,0xB0,0xC8); BEZ=(0x30,0x30,0x40); RIM=(0x58,0x58,0x70); OUTL=(0x20,0x20,0x28)
RED=(0xD0,0x18,0x38); BLUE=(0x50,0x38,0xC8); NAVY=(0x28,0x28,0x88); SHADOW=(0x90,0x80,0x98)
LEDD=(0x50,0x08,0x10); LEDH=(0xFF,0xA0,0xA0)
W,H=256,224
def shape(x0,y0,x1,y1,r,rb):
    """1-bit mask: rounded corners r, the bottom right one big (rb), stepped like pixel art."""
    yy,xx=np.mgrid[0:H,0:W]+0.5
    m=(xx>=x0)&(xx<x1)&(yy>=y0)&(yy<y1)
    for cx,cy,rr,sx,sy in ((x0+r,y0+r,r,-1,-1),(x1-r,y0+r,r,1,-1),(x0+r,y1-r,r,-1,1),(x1-rb,y1-rb,rb,1,1)):
        corner=((xx-cx)*sx>0)&((yy-cy)*sy>0)
        m&=~(corner&((xx-cx)**2+(yy-cy)**2>rr*rr))
    return m
def paint(img,mask,c): img[mask]=c
def text_bits(t,font,size):
    F=ImageFont.truetype(font,size); b=F.getbbox(t)
    im=Image.new('L',(b[2]+2,b[3]+2),0); ImageDraw.Draw(im).text((0,0),t,font=F,fill=255)
    return np.asarray(im)>110, b
def stamp(img,bits,x,y,c,shadow=None):
    h,w=bits.shape
    if shadow is not None:
        img[y+1:y+1+h,x+1:x+1+w][bits]=shadow
    img[y:y+h,x:x+w][bits]=c

S = ["..######..",".########.","####..####","####......",".######...","..#######.",".....#####","......####","####..####",".########.","..######.."]
O = ["..#######..",".#########.","####...####","####...####","####...####","####...####","####...####","####...####","####...####",".#########.","..#######.."]
T = [".####...",".####...",".####...","########","########",".####...",".####...",".####...",".####...",".####...",".####...",".####...",".#######","..######"]
L = ["####"]*14
P = ["####"]*4
def slot_bits():
    letters=[(S,3),(L,0),(O,3),(T,0),(P,10)]
    w=sum(len(g[0]) for g,_ in letters)+2*(len(letters)-1)
    b=np.zeros((14,w),bool); x=0
    for g,top in letters:
        for j,row in enumerate(g):
            for i,ch in enumerate(row):
                if ch=='#': b[top+j,x+i]=True
        x+=len(g[0])+2
    return b
def sgb1(img):
    img[:]=BG
    outer=shape(9,17,247,194,4,34)
    inner=shape(10,18,246,193,3,33)
    inner2=shape(11,19,245,192,3,32)
    paint(img,outer,OUTL); paint(img,inner,RIM); paint(img,inner2,BEZ)
    # the red and blue lines across the top of the bezel
    img[27,16:240]=RED; img[30,16:240]=BLUE
    led(img,25,84)
    img[40:184,48:208]=0
    X=14; base=219
    wordmark(img,X,base,NAVY,RED,SHADOW)
def led(img,x,y):
    # power lamp: a dark ring and one bright pixel
    LED=['.ooo.','oRRRo','oHRRo','oRRRo','.ooo.']
    for j,row in enumerate(LED):
        for i,ch in enumerate(row):
            if ch!='.': img[y+j,x+i]={'o':LEDD,'R':RED,'H':LEDH}[ch]
def wordmark(img,X,base,ink,x_ink,shadow,scale=1):
    """slot. x GRAVY with the 1px shadow the original's lettering has; base is the baseline."""
    IXI='/usr/share/fonts/opentype/inter/Inter-ExtraBoldItalic.otf'
    x_,_=text_bits('x',IXI,13); g,_=text_bits('GRAVY',IXI,21)
    s=slot_bits()
    stamp(img,s,X,base-2-s.shape[0]+1,ink,shadow); X+=s.shape[1]+4
    stamp(img,x_,X,base-g.shape[0]//2-x_.shape[0]//2-1,x_ink,shadow); X+=x_.shape[1]+3
    stamp(img,g,X,base-g.shape[0]+1,ink,shadow)
    return X+g.shape[1]
# the SGB2's border (Roy 2026-11-12, sg1): the SGB1 layout on the Super Famicom's light grey,
# the original's red and blue lines, dark grey lettering and a red 2
SFC=(0xC8,0xC8,0xCC); SFCS=(0x98,0x98,0xA0); INK2=(0x48,0x48,0x50)
def sgb2(img):
    img[:]=SFC
    paint(img,shape(9,17,247,194,4,34),OUTL); paint(img,shape(10,18,246,193,3,33),RIM)
    paint(img,shape(11,19,245,192,3,32),BEZ)
    img[27,16:240]=RED; img[30,16:240]=BLUE
    led(img,25,84)
    img[40:184,48:208]=0
    end=wordmark(img,14,219,INK2,RED,SFCS)
    two,_=text_bits('2',IXI2,21); stamp(img,two,end+5,219-two.shape[0]+1,RED,SFCS)
IXI2='/usr/share/fonts/opentype/inter/Inter-ExtraBoldItalic.otf'
img=np.zeros((H,W,3),np.uint8)
sgb2(img) if SGB2 else sgb1(img)
Image.fromarray(img).save(OUT)
