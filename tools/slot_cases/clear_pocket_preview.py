"""The clear Game Boy Pocket and Light slots (Roy 2026-11-05): the flat Pocket/Light slot of
tools/slot_cases/flat_gb.py made in the clear Play It Loud's and the Atomic Purple's clear plastic
(tools/slot_cases/clear_gb_preview.py, clear_gbc_preview.py), over the Pocket's own board: the top
band of the MGB-ECPU (Light: MGB-LCPU, the same top band), laid out off Roy's photo of it. The
Pocket and Light have the DMG's slide switch without its lock (no tab, no guide hole): the
switch's plastic slider shows through the clear shell (slot_chrome.rs draw_power_slider).
Usage: python3 tools/slot_cases/clear_pocket_preview.py [NAME ...] (default: all of CASES).
BOARD_ONLY=<png> renders the board alone. Needs scipy. Run from the repo root."""
import os,sys
import numpy as np
from PIL import Image,ImageDraw,ImageFont
from scipy.ndimage import gaussian_filter,distance_transform_edt as edt
src=open('tools/slot_art.py').read().split('MOUTHS = ')[0].replace('ROOT = Path(__file__).resolve().parents[1]','ROOT=None')
# the flat Pocket/Light slot (as tools/slot_cases/flat_gb.py), with the Game Boy Color's lit rib tops
for _a,_b in (("WALL = 2.2","WALL = 0.7"),("INNER = 5.5","INNER = 1.4"),
    ("cast = 0.50 * np.exp(-(d_out - WALL).clip(0) / 3.5)","cast = 0.08 * np.exp(-(d_out - WALL).clip(0) / 1.0)"),
    ("deep = deep * (0.80 + 0.20 * smooth(INNER, INNER + 8.0, d_out_px))","deep = deep * (0.98 + 0.02 * smooth(INNER, INNER + 2.0, d_out_px))"),
    ("side_shade = left_w * (0.46 + 0.12 * t) + (1 - left_w) * (0.70 - 0.06 * t)","side_shade = left_w * (0.86 + 0.06 * t) + (1 - left_w) * (0.93 - 0.02 * t)"),
    ("deep = 0.60 + 0.12 * smooth(3.0, 30.0, yy)","deep = 0.94 + 0.02 * smooth(TOP_BEVEL, 30.0, yy)"),
    ("deep = np.where(rim, 0.92 - 0.10 * (yy / 2.5), deep)","deep = np.where(yy < TOP_BEVEL + 1, deep * shade, deep)"),
    ("rib_shade = np.where(dy > 0, 1.0 - 0.25 * np.clip(-(yy - top) / r, 0, 1), 1.0)","rib_shade = np.where(dy > 0, 1.0 + 0.08 * np.clip(-(yy - top) / r, 0, 1), 1.0)"),
    ("shadow = gap & ~rib & (xs_ > cx) & (xs_ < cx + r + 2.5) & (yy > RIB_TOP + 1)","shadow = gap & ~rib & (xs_ > cx) & (np.hypot(xs_ - cx - 1.4, np.maximum(top + 0.8 - yy, 0)) < r + 1.0)")):
    assert src.count(_a)==1,_a;src=src.replace(_a,_b)
# shallow mouths: the rims run under the seated pak, a few px in on each side (Roy 2026-11-05)
src=src.replace('GB_MOUTH = (244, 476)','GB_MOUTH = (250, 470)')
g={'__file__':'tools/slot_art.py'};exec(src,g)
mouth=g['gb_mouth']();W,H=720,58
base=np.array([236,236,230],np.float32)
PX0,PX1=250,470

# The clear cases Nintendo made of the Pocket and the Light (Roy: clear and solid both options,
# as with the carts). Per case: the rim's colour seen edge-on, its glint, the empty mouth's smoke,
# what the plastic lets through (a filter over the board), the front shell's tint.
CASES={
    "pocket_clear":dict(rim=(176,182,180),glint=(244,246,244),smoke=(150,156,154),through=(1,1,1),shell=(1,1,1)),
    "pocket_iceblue":dict(rim=(132,170,196),glint=(226,240,250),smoke=(112,150,176),through=(0.84,0.94,1.0),shell=(0.80,0.92,1.0)),
    # Japan's Clear Purple (1997, Roy 2026-11-06: all the original colours)
    "pocket_clearpurple":dict(rim=(146,122,186),glint=(236,226,250),smoke=(122,100,166),through=(0.90,0.80,1.0),shell=(0.86,0.74,1.0)),
    "light_clear":dict(rim=(176,182,180),glint=(244,246,244),smoke=(150,156,154),through=(1,1,1),shell=(1,1,1)),
    "light_yellow":dict(rim=(206,180,96),glint=(252,240,200),smoke=(182,158,84),through=(1.0,0.94,0.70),shell=(1.0,0.90,0.62)),
}

# ---- the board: the MGB-ECPU's top band, off Roy's photo of the whole board (photo px; the
# board's top edge at photo row 40, its sides at 430 and 1070). One photo px is S strip px; SW1's
# right end lines up with the right end of the switch's plastic slider bar (strip 509.5, as on
# the DMG, slot_chrome.rs draw_power_slider), the board's top edge at strip row BT.
_tb=open('tools/carts/tv_boards.py').read().split('# ---- SNES')[0]
_g={'__file__':'tools/carts/tv_boards.py'};exec(_tb,_g)
# Scale and place off Roy's photo of a clear Pocket seen from the top (2026-11-05): the
# connector's middle at strip 372.8, the board's top edge at row 31
# then lifted so SW1's top meets the slider bar's foot (row 17, slot_chrome.rs
# POCKET_SLIDER_BOTTOM) and the bar meets the top rim (Roy 2026-11-06: no gaps)
S_=0.42;BT=16.5-4*0.42
def Q(px,py): return 372.8+(px-781)*S_, BT+(py-40)*S_
BX0,_=Q(430,0);BR,_=Q(1070,0)
m4=_g['rr'](H,W,(round(BX0),round(BT),round(BR),H+8),2)
vias=[]
traces=[[Q(560,100),Q(560,108),Q(520,108)],[Q(612,100),Q(612,112)],[Q(890,120),Q(905,135),Q(905,140)],
        [Q(965,92),Q(965,120),Q(980,135)],[Q(1000,92),Q(1012,104),Q(1012,140)]]
bd=_g['build'](m4,[],[],[],vias,traces).astype(np.float32)
B=bd[...,:3].copy();board=bd[...,3]/255>0.5
GBV=_g['gb'][18:25,86:93]          # the Game Boy cart board's small via, copied over
for px,py in ((500,54),(546,54),(592,54),(637,54),(658,128),(538,122),(940,96),(1000,128),(1055,58),(960,140)):
    x_,y_=Q(px,py)
    if y_<H-4: _g["paste"](B,GBV,round(x_)-3,round(y_)-3)
K=4
yy,xx=np.mgrid[0:H*K,0:W*K]/K
L4=np.zeros((H*K,W*K,4),np.float32)
def over(mask,rgb,a=1.0):
    m=mask.astype(np.float32)[...,None]*a
    L4[...,:3]=L4[...,:3]*(1-m)+np.asarray(rgb,np.float32)*m;L4[...,3:]=np.maximum(L4[...,3:],m*255)
def rr4(x0,y0,x1,y1,r): return np.hypot(np.maximum(np.maximum(x0+r-xx,xx-(x1-r)),0),np.maximum(np.maximum(y0+r-yy,yy-(y1-r)),0))<=r
def box(p0,p1,r=0.6):
    (x0,y0),(x1,y1)=Q(*p0),Q(*p1);return rr4(x0,y0,x1,y1,r)
def shadow(mask,dx=1.0,dy=1.4,s=1.3,a=0.55):
    sh=gaussian_filter(np.roll(np.roll(mask.astype(np.float32),int(dy*K),0),int(dx*K),1),s*K)*(1-mask)
    L4[...,3]=np.maximum(L4[...,3],sh*255*a);L4[...,:3]*=(1-sh*a)[...,None]
nz=lambda s,sd:gaussian_filter(np.random.default_rng(sd).normal(0,1,(H*K,W*K)),s)
SILK=(206,220,210)
def silk_box(p0,p1,w=0.6):
    (x0,y0),(x1,y1)=Q(*p0),Q(*p1);o=rr4(x0,y0,x1,y1,1.0)&~rr4(x0+w,y0+w,x1-w,y1-w,0.6);over(o,SILK,0.9)
def gold(m,sd):
    over(m,np.stack([206+nz(1.2,sd)*12,168+nz(1.2,sd)*10,74+nz(1.2,sd)*8],-1))
    over(m&~np.roll(np.roll(m,int(0.6*K),0),int(0.6*K),1),(232,206,120))
def hole(cx,cy,r):
    x,y=Q(cx,cy);d=np.hypot(xx-x,yy-y);over(d<=r,(196,170,96));over(d<=r*0.68,(12,22,16))
# the gold mounting pads in the top corners, their screw holes (and the left one's test square)
gold(box((432,42),(477,140)),4);hole(448,98,5.0);over(box((442,54),(454,66),0.3),(40,44,30))
gold(box((1023,72),(1068,150)),5);hole(1050,105,5.0)
# two-pad parts: tan ceramic caps (silver ends), small black resistors, in their silk frames
def cap(p0,p1,vert=False,body=(178,150,108)):
    m=box(p0,p1,0.5);shadow(m,0.8,1.0,0.8,0.45)
    (x0,y0),(x1,y1)=Q(*p0),Q(*p1);v=np.clip((yy-y0)/max(y1-y0,1),0,1) if not vert else np.clip((xx-x0)/max(x1-x0,1),0,1)
    over(m,np.stack([body[0]+30*(1-v),body[1]+26*(1-v),body[2]+20*(1-v)],-1))
    e=0.24*((x1-x0) if not vert else (y1-y0))
    ends=(rr4(x0,y0,x0+e,y1,0.4)|rr4(x1-e,y0,x1,y1,0.4)) if not vert else (rr4(x0,y0,x1,y0+e,0.4)|rr4(x0,y1-e,x1,y1,0.4))
    over(m&ends,np.stack([196+20*(1-v)]*3,-1))
def res(p0,p1):
    m=box(p0,p1,0.4);shadow(m,0.8,1.0,0.8,0.45);over(m,(26,28,30))
    (x0,y0),(x1,y1)=Q(*p0),Q(*p1);e=0.22*(y1-y0)
    over(m&(rr4(x0,y0,x1,y0+e,0.3)|rr4(x0,y1-e,x1,y1,0.3)),(190,192,196))
silk_box((538,76),(574,100));cap((542,82),(570,96))                    # C21
silk_box((572,70),(594,100));res((577,75),(590,96))                    # C22
silk_box((598,74),(640,100));cap((603,79),(635,95))                    # C20
silk_box((642,74),(664,100));res((647,78),(660,96))                    # R1
silk_box((640,104),(662,140));cap((644,108),(658,138),vert=True)      # C23
# U4, DMG-REG: a navy chip with its gull-wing legs both sides (its top half shows)
x0,y0=Q(522,116);x1,y1=Q(604,190);ch=rr4(x0,y0,x1,y1,0.7);shadow(ch,1.0,1.4,1.3,0.6)

for y_ in np.arange(y0+1.0,y1,1.25):
    for a_,b_ in ((x0-3.0,x0),(x1,x1+3.0)):
        lg=rr4(a_,y_,b_,y_+0.6,0.2);over(lg,(186,190,196))
v=np.clip((yy-y0)/max(y1-y0,1),0,1);over(ch,np.stack([10+8*v,22+10*v,40+14*v],-1))
over(ch&(np.minimum(xx-x0,yy-y0)<0.5),(40,56,82))
x,y=Q(530,124);over(np.hypot(xx-x,yy-y)<=1.2,(24,36,58))
# -- the LCD's FPC connector and ribbon, after Roy's photo of a clear Pocket from the top
# (2026-11-05): photo px -> strip by P2 (the mouth's 910 photo px = its 232 strip px)
def P2(x,y): return 244+(x-355)*0.255,(y-195)*0.255-(31-BT)   # lifted with the board
def box2(p0,p1,r=0.6):
    (x0,y0),(x1,y1)=P2(*p0),P2(*p1);return rr4(x0,y0,x1,y1,r)
silk_box((664,48),(896,138))
# the dark grey housing, its white latch nubs only at its two ends, silver hold-downs below them
ho=box2((685,340),(1030,396),0.6);shadow(ho,1.0,1.4,1.4,0.5)
(_,hy0),(_,hy1)=P2(0,340),P2(0,396);v=np.clip((yy-hy0)/(hy1-hy0),0,1)
over(ho,np.stack([40-12*v,40-12*v,44-12*v],-1))
over(ho&(yy<hy0+0.8),(70,70,74))
for x_ in np.arange(P2(690,0)[0],P2(1025,0)[0],1.1): over(ho&rr4(x_,hy0+1,x_+0.25,hy1,0.1),(22,22,26))
for a_ in (675,1008):
    n_=box2((a_,333),(a_+33,356),0.8);shadow(n_,0.8,1.0,1.0,0.45)
    (_,ny0),(_,ny1)=P2(0,333),P2(0,356);over(n_,np.stack([246-30*np.clip((yy-ny0)/(ny1-ny0),0,1)]*3,-1))
for a_ in (690,1004):
    t_=box2((a_,396),(a_+24,416),0.5);over(t_,np.stack([186+30*np.clip(1-np.abs(yy-P2(0,404)[1])/2.5,0,1)]*3,-1))
# its pins (Roy 2026-11-06: shorter, with life): each a short tinned leg out from under the
# housing's lip, bent down onto its own gold pad, a shiny solder fillet at its foot; each a
# touch different, as hand-soldered tin is
(_,py0),(_,py1)=P2(0,396),P2(0,426)
prng=np.random.default_rng(31)
for k_,x_ in enumerate(np.linspace(P2(722,0)[0],P2(992,0)[0],24)):
    j=prng.normal(0,1,4)
    py1=P2(0,426)[1]   # all feet level (Roy)
    pad=rr4(x_-0.45,py1-2.8,x_+1.75,py1+0.9,0.5);over(pad,(214,170,70))          # the gold pad
    over(rr4(x_-0.6,py0,x_+1.9,py0+0.9,0.2),(14,14,16),0.8)                       # shade under the lip
    lg=rr4(x_,py0+0.4,x_+1.3,py1-1.0+0.4*j[0],0.4);shadow(lg,0.6,0.8,0.6,0.4)
    hi=np.clip(1-np.abs(xx-x_-0.45)/0.5,0,1)                                       # the leg's lit edge
    over(lg,np.stack([150+70*hi+6*j[1]]*3,-1)*np.array([1,1,1.03]))
    cy=py1-1.4+0.3*j[2];fl=np.hypot((xx-x_-0.65)/1.15,(yy-cy)/0.95)<=1.0          # the solder fillet
    shadow(fl,0.6,0.8,0.6,0.35)
    gl=np.clip(1-np.hypot(xx-x_-0.35,yy-cy+0.35)/0.9,0,1)
    over(fl,np.stack([168+80*gl+5*j[3]]*3,-1))
# -- the LCD's ribbon (Roy 2026-11-06): straight up out of the connector, then a diagonal step
# inward by three traces' width on each side, then straight up again to the fold; separate
# copper traces with darker film between them, all following that shape
(BX0r,BY),(BX1r,_)=P2(719,342),P2(999,342);TY=max(P2(0,245)[1],1.0)
N=22;PITCH=(BX1r-BX0r)/N
Y_STEP0,Y_STEP1=BY-4.0,BY-8.0             # where the diagonal starts and ends (rows)
INSET=3*PITCH
def edge_in(y):                             # how far each side has stepped in at row y
    return INSET*np.clip((Y_STEP0-y)/(Y_STEP0-Y_STEP1),0,1)
inn=edge_in(yy)
# each trace's place across: the bottom's evenly, squeezed into the narrower top
x_at=lambda f:(BX0r+inn)+(BX1r-BX0r-2*inn)*f
film=(xx>=x_at(0))&(xx<=x_at(1))&(yy>=TY+0.6)&(yy<=BY)
shadow(film,1.0,1.4,1.6,0.45);over(film,(120,52,24),0.85)
ribm=np.zeros_like(film);col=np.zeros(xx.shape+(3,),np.float32)
for k in range(N):
    xl,xr=x_at((k+0.18)/N),x_at((k+0.82)/N)
    m=film&(xx>=xl)&(xx<=xr)
    c=np.clip(1-np.abs((xx-(xl+xr)/2)/((xr-xl)/2+1e-6)),0,1)
    ribm|=m;col=np.where(m[...,None],np.stack([206+40*c,96+40*c,40+24*c],-1),col)
tt=np.clip((yy-TY)/(BY-TY),0,1)
over(ribm,col*(1+0.22*np.exp(-((tt-0.25)/0.16)**2))[...,None])
# -- SW1: the silver slide switch at the top edge, its two dark bracket ends, its pins below
sw=box((906,44),(1036,80),0.8);shadow(sw,1.2,1.6,1.5,0.6)
_,sy0=Q(0,44);_,sy1=Q(0,80);v=np.clip((yy-sy0)/(sy1-sy0),0,1)
ct=180+30*np.exp(-((v-0.25)/0.16)**2)-24*v+nz((0.3*K,1.6*K),5)*7
over(sw,np.stack([ct,ct+2,ct+5],-1))
for a_ in (914,1026): over(box((a_,46),(a_+4,78),0.3),(44,44,46))        # its dark bracket edges
for cx_ in (909,1032):   # the round rivets at its ends
    x,y=Q(cx_,60);d=np.hypot(xx-x,yy-y);over(d<=3.0,np.stack([150+70*np.clip(1-np.hypot(xx-x+0.8,yy-y+0.8)/3,0,1)]*3,-1));over(d<=1.0,(70,70,74))
over(box((924,70),(1016,77),0.3),(50,50,52))                                         # the slider's slit
for px in (930,966,1000,1018):   # pins 1, C, 2, 3 and their pads
    m=box((px-5,80),(px+5,92),0.4);over(m,np.stack([186+nz(1.0,8)*16]*3,-1))
# its silkscreen: SW1, ON<->OFF, the pin names; P2 and the connector's pin numbers; U4
tl=Image.new('L',(W*K,H*K),0);td=ImageDraw.Draw(tl)
FB='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
def txt(s,px,py,sz,rot=0):
    x,y=Q(px,py);f=ImageFont.truetype(FB,int(sz*K))
    if rot:
        im=Image.new('L',(int(sz*K*len(s)),int(sz*K*1.3)),0);ImageDraw.Draw(im).text((0,0),s,font=f,fill=255)
        im=im.rotate(rot,expand=True);tl.paste(im,(int(x*K),int(y*K)),im)
    else: td.text((x*K,y*K),s,font=f,fill=255)
txt('SW1',926,102,7.0);txt('ON\u2194OFF',902,118,6.6);txt('1',920,90,6.0);txt('C',956,88,6.0);txt('2',994,88,6.0);txt('3',1022,80,6.0)
txt('P2',894,94,6.6,90);txt('18',664,120,6.6);txt('\u25c01',874,118,6.6)
txt('C21',538,58,7.2);txt('C22',570,52,7.2);txt('C20',602,58,7.2);txt('R1',646,58,7.2);txt('C23',664,104,5.0,90)
txt('U4',486,120,5.0,90);txt('R4',1004,124,5.0,90)
over(np.asarray(tl,np.float32)/255>0.5,SILK,0.92)
swl=np.asarray(gaussian_filter(L4,(1.0,1.0,0)))
swl=np.asarray(Image.fromarray(np.clip(swl,0,255).astype(np.uint8),'RGBA').resize((W,H),Image.LANCZOS),np.float32)
al=swl[...,3:]/255;B=B*(1-al)+swl[...,:3]*al
board=board|(al[...,0]>0.7)    # the ribbon stands above the board's top edge
if os.environ.get('BOARD_ONLY'):
    Image.fromarray(np.dstack([np.clip(B,0,255),board*255]).astype(np.uint8),'RGBA').save(os.environ['BOARD_ONLY']);raise SystemExit
plate=np.zeros((H,W),bool)
Ish=gaussian_filter(B.copy(),(0.45,0.45,0))*0.96+6   # seen through the clear plastic: a hair soft
behind=board|plate
def layer(front):
    p=g['draw'](mouth,base,front,False).astype(np.float32)
    rgb,a=p[...,:3],p[...,3]/255
    if front:   # no cast shadow along the mouth's top: it cut the top glint off there (Roy: gaps)
        a=np.where((a<0.99)&(np.arange(H)[:,None]<RIM_W+3),0,a)
    rgb=gaussian_filter(rgb,(0.7,0.7,0))
    lum=rgb.mean(2);m=np.median(lum[(a>0.99)&~behind]);d=np.abs(lum-m)
    t=np.clip((d-9)/16,0,1)*a*0.75
    # the plate sits behind the slot's back wall: over it the wall's ribs and lip stay visible
    wall=behind&(np.arange(W)[None,:]>=PX0)&(np.arange(W)[None,:]<PX1)
    gy,gx=np.gradient(gaussian_filter(lum,0.6));edge=np.clip((np.abs(gx)+np.abs(gy)-3)/10,0,1)*0.6
    t=np.where(behind,np.where(wall,np.maximum(edge,0.05)*a,np.minimum(np.maximum(t,0.09*a),0.18)),np.maximum(t,0.22*a))
    if not front: t=np.where(~behind&(np.arange(W)[None,:]>=PX0)&(np.arange(W)[None,:]<PX1),np.minimum(t,0.2),t)
    # thick plastic seen edge-on, like the clear carts' rims: a dense band along every cut edge
    # of the shell (the top and the mouth), bright on its outer pixel
    solid=a>0.5;dd=edt(solid);dd[0,:]=np.minimum(dd[0,:],1);dd=np.minimum(dd,edt(np.pad(solid,((1,0),(0,0)),constant_values=False))[1:])
    dd=np.where(solid,gaussian_filter(dd,1.4),dd)   # iso-lines round the corners off
    dd0=dd.copy()
    if not front:   # the mouth's side walls, seen edge-on through the back
        xs=np.arange(W)[None,:].repeat(H,0)
        # the shallow mouth's side walls are only ~1 mm: a thin rim, not the full RIM_W band (Roy)
        ds=np.minimum(np.where(xs<PX0+2,np.abs(xs-PX0)+1,99),np.where(xs>PX1-3,np.abs(PX1-1-xs)+1,99))
        # (no own line any more: the front shell's edge is the same 1 mm wall, a second line
        # beside it read as a hard double edge, Roy 2026-11-05)
    if front:   # the mouth's sides in the front shell: the same thin ~1 mm rim (Roy)
        dh=edt(solid,sampling=(1000,1));side=(dh<=dd+0.5)&(dh<=RIM_W+1)
        dh=np.where(dh<=SIDE_W,1+(dh-1)*(RIM_W-1)/max(SIDE_W-1,1),99)
        dd=np.where(side,np.minimum(dh,np.arange(H)[:,None]+1.0),dd)
    # a thick clear edge with a ROUND profile (Roy 2026-11-05: the hard bands read blocky): the
    # plastic gets denser smoothly toward the edge, one soft glint runs a little inside it, and
    # everything fades out with no step into the face
    u=np.clip((dd-0.5)/RIM_W,0,1);rim=solid&(dd<=RIM_W+2)
    dens=(1-u)**1.6;glint=np.exp(-((u-0.20)/0.13)**2)
    t=np.where(solid,np.maximum(t,(0.18+0.62*dens)*a),t)
    rimc=RIMC+dens[...,None]*np.array([12,12,12],np.float32)+glint[...,None]*(GLINT-RIMC)*0.85
    w=np.clip(dens*1.4+glint,0,1)[...,None]
    # the glint is one unbroken line: dense enough that nothing behind breaks it up (Roy: gaps)
    t=np.where(solid,np.maximum(t,(0.25+0.65*np.maximum(dens,glint))*a),t)
    # the shell is soft in the hand: every edge rounds off in a broad, gentle falloff of
    # light into the face, not a block with a line around it (Roy 2026-11-05)
    soft=np.exp(-(dd0-1).clip(0)/6.0)*solid
    t=np.maximum(t,(0.16+0.30*soft)*a);rgb=rgb+soft[...,None]*SOFTC
    global RIMW;RIMW=gaussian_filter(w[...,0]*solid,0.6)
    rgb=np.where(solid[...,None],rgb*(1-w)+rimc*w,rgb)
    rgb=gaussian_filter(rgb,(1.2,1.2,0));t=gaussian_filter(t,1.0)   # soft, rounded, no hard lines
    # a broad soft sheen across the flat faces, as light falls on a curved clear shell
    xs_=np.arange(W)[None,:];ys_=np.arange(H)[:,None]
    sheen=np.exp(-(((xs_-0.35*ys_)%240-60)/38.0)**2)*0.06*a
    t=np.clip(t+sheen,0,1);rgb=rgb+sheen[...,None]*300
    # the clear plastic's grain, as the carts have
    gr=g['mottle'](11+front)[:H,:W]
    # a touch frosted rather than glass-clear (Roy 2026-11-03: a little duller, subtle)
    # smooth like the clear carts (assets/gbc_clear.png): an even haze, no grain (Roy 2026-11-05)
    rgb=rgb+2;t=np.clip(t+0.03*a,0,1)
    # the Game Boy's moulded plastic finish (Roy 2026-11-05: the real shell structure, no flow
    # lines, no grain filter): a fine spark-eroded stipple of tiny bumps and pits, each lit from
    # the top left like the shell, plus a slow uneven haze
    tr=np.random.default_rng(21+front)
    n0=tr.normal(0,1,(H*2,W*2));bump=gaussian_filter(n0,0.8)+0.6*gaussian_filter(tr.normal(0,1,(H*2,W*2)),2.2)   # two sizes of pit
    by,bx=np.gradient(bump);lit=-(bx*0.55+by*0.8)
    lit=np.array(Image.fromarray(lit.astype(np.float32),'F').resize((W,H),Image.BOX));lit/=lit.std()+1e-6
    haze=gaussian_filter(tr.normal(0,1,(H,W)),(7,22));haze/=haze.std()+1e-6
    face=np.clip((dd0-2)/4,0,1)*a      # on the faces, not on the round edges
    # a finish only shows where it catches light: strong in the shell's lit areas (toward the
    # top, where the light falls on it), fading where you look through the plastic; it frosts
    # the plastic a touch rather than painting dots on it
    lum_=rgb.mean(2);catch=np.clip((lum_-np.percentile(lum_[a>0.5],30))/60,0,1)
    catch=np.clip(0.25+0.55*catch+0.35*np.exp(-np.arange(H)[:,None]/14.0),0,1)
    t=np.clip(t+face*(0.012*haze+0.022*catch*np.clip(lit,0,None)),0,1)
    rgb=rgb+(face*(6.5*catch*lit+1.6*haze))[...,None]
    return rgb,t,a
RIM_W=8
SEE_THROUGH=float(os.environ.get('SEE_THROUGH','0.45'))   # how much of the board shows, 1 = as before
SIDE_W=2
for name in (sys.argv[1:] or CASES):
    c=CASES[name]
    RIMC=np.array(c['rim'],np.float32);GLINT=np.array(c['glint'],np.float32)
    SOFTC=(GLINT-RIMC)*0.2+np.array([24,24,24],np.float32)
    smoke=np.array(c['smoke'],np.float32);THROUGH=np.array(c['through'],np.float32)
    rgb,t,a=layer(False)
    sw_=(np.clip((0.55-t)/0.15,0,1)*(1-RIMW))[...,None]
    rgb=np.where(behind[...,None],rgb,rgb*(1-sw_)+smoke*sw_)
    tb=1-(1-t)*SEE_THROUGH   # st7: a little less see-through (Roy 2026-11-12)
    col=np.where(behind[...,None],(rgb*tb[...,None]+(1-tb[...,None])*Ish)*THROUGH,rgb)
    al=np.where(behind,a,t)
    # the two slide ribs on the back wall in smooth glossy clear (as the clear DMG and GBC)
    yf,xf=np.mgrid[0:H,0:W].astype(np.float32)+0.5
    for cx in (PX0+(PX1-PX0)*0.24,PX0+(PX1-PX0)*0.76):
        r=3.0;top=9.0+r
        d=np.hypot(xf-cx,np.maximum(top-yf,0));m=np.clip(r-d+0.5,0,1)
        nx=np.clip((xf-cx)/r,-1,1);ny=np.clip((top-yf)/r,0,1)
        hl=np.clip(-nx*0.8+ny*0.6,0,1)**2;sh=np.clip(nx,0,1)**1.5
        col=col*(1-0.30*sh*m)[...,None]+(m*hl*0.75)[...,None]*(GLINT-col)
        col=col+(m*0.10)[...,None]*(RIMC+30-col)
        sd=np.clip(1-np.abs(xf-cx-r-1.2)/1.6,0,1)*(yf>top-1)*(1-m);col=col*(1-0.18*sd)[...,None]
        al=np.maximum(al,np.maximum(m*0.55,sd*0.2))
    bk=np.dstack([col,al*255])
    rgb,t,a=layer(True)
    rgb=rgb*np.array(c['shell'],np.float32)
    fr=np.dstack([rgb,np.maximum(t,0.05*a)*255])
    # the top edge's glint runs unbroken across the mouth (as clear_gb_preview.py)
    bk=np.clip(bk,0,255)/255;fr=np.clip(fr,0,255)/255
    ref=PX0-24;R=7
    fa=fr[:R,ref,3:];ba=bk[:R,ref,3:];oa=fa+ba*(1-fa)
    orgb=(fr[:R,ref,:3]*fa+bk[:R,ref,:3]*ba*(1-fa))/np.maximum(oa,1e-6)
    prof=np.concatenate([orgb,oa],1)[:,None,:]
    wgt=np.clip((R-np.arange(R))/3.0,0,1)[:,None,None]
    bk[:R,PX0:PX1]=bk[:R,PX0:PX1]*(1-wgt)+prof*wgt
    for part,arr in (('back',bk),('front',fr)):
        out=f'assets/slot_{part}_gb_{name}.png'
        Image.fromarray(np.clip(arr*255,0,255).astype(np.uint8),'RGBA').save(out,optimize=True);print('wrote',out)
