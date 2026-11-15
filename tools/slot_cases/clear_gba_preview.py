"""The Game Boy Advance SP slot in clear plastic, over the top band of its own C/AGS-CPU board
and the cart bay's metal plate, which runs on past the mouth into the case (Roy 2026-11-06).
Built like the clear DMG, Pocket and Atomic Purple slots (tools/slot_cases/clear_gb_preview.py,
clear_pocket_preview.py, clear_gbc_preview.py): the board the way the cart boards are made
(tools/carts/tv_boards.py build()), the shell from tools/slot_art.py's GBA mouth with its finger
scoop. Writes assets/slot_{back,front}_gba_<case>.png. BOARD_ONLY=<png> renders the board alone.
Run from the repo root; needs scipy."""
import os
import numpy as np
from PIL import Image,ImageDraw,ImageFont
from scipy.ndimage import gaussian_filter
from scipy.ndimage import distance_transform_edt as edt
src=open('tools/slot_art.py').read().split('MOUTHS = ')[0].replace('ROOT = Path(__file__).resolve().parents[1]','ROOT=None')
g={'__file__':'tools/slot_art.py'};exec(src,g)
mouth=g['gba_mouth']();W,H=720,58
PX0,PX1=233,487                     # the GBA mouth (tools/slot_art.py gba_mouth)
MT=g['METAL_TOP']
PLATE_OUT=18                        # how far the plate runs on into the case past the mouth
PLATE_TOP=11                        # under the plastic it runs up to almost the board's top edge (BT 9) (Roy)
_tb=open('tools/carts/tv_boards.py').read().split('# ---- SNES')[0]
_g={'__file__':'tools/carts/tv_boards.py'};exec(_tb,_g)

# ---- the board: the top band of an SP's C/AGS-CPU-10, laid out after Roy's photo of it (the
# cart side, the cart going in from the top). A GBA cart's connector is ~1 photo px per strip px
# wide there, so the photo maps 1:1, its connector centred on the mouth. That photo is a mirror
# image (its silkscreen reads backwards), so the board is laid out in the photo's frame and then
# flipped, its silkscreen written last the right way round: the power switch's side on the left,
# as seen from the back.
BT=9                                 # the board's top edge
def Q(px,py): return px+58, py-48+BT
K=4
yy,xx=np.mgrid[0:H*K,0:W*K]/K
def rr4(x0,y0,x1,y1,r): return np.hypot(np.maximum(np.maximum(x0+r-xx,xx-(x1-r)),0),np.maximum(np.maximum(y0+r-yy,yy-(y1-r)),0))<=r
BX0,BX1=Q(123,0)[0],Q(473,0)[0]
m4=rr4(BX0,BT,BX1,H+8,2.0)
# the round bite out of its top edge round the shell's middle screw post, and the two holes the
# top corners' posts go through
NC=Q(300,45);m4&=np.hypot(xx-NC[0],yy-NC[1])>11
HOLES=[Q(138,70),Q(460,70)]
for hx,hy in HOLES: m4&=np.hypot(xx-hx,yy-hy)>4.6
vias=[Q(122+58,65),Q(129+58,65)] if False else []
traces=[[Q(170,80),Q(182,80),Q(186,84)],[Q(232,74),Q(246,74),Q(252,68)],[Q(372,92),Q(395,92)],[Q(440,80),Q(440,94)]]
bd=_g['build'](m4,[],[],[],vias,traces).astype(np.float32)
B=bd[...,:3].copy();board=bd[...,3]/255>0.5
L4=np.zeros((H*K,W*K,4),np.float32)
def over(mask,rgb,a=1.0):
    m=mask.astype(np.float32)[...,None]*a
    L4[...,:3]=L4[...,:3]*(1-m)+np.asarray(rgb,np.float32)*m;L4[...,3:]=np.maximum(L4[...,3:],m*255)
def shadow(mask,dx=1.0,dy=1.4,s=1.3,a=0.55):
    sh=gaussian_filter(np.roll(np.roll(mask.astype(np.float32),int(dy*K),0),int(dx*K),1),s*K)*(1-mask)
    L4[...,3]=np.maximum(L4[...,3],sh*255*a);L4[...,:3]*=(1-sh*a)[...,None]
nz=lambda s,sd:gaussian_filter(np.random.default_rng(sd).normal(0,1,(H*K,W*K)),s)
def gold(m,sd,k=1.0):
    over(m,np.stack([(198+nz(1.4,sd)*12)*k,(166+nz(1.4,sd)*10)*k,(92+nz(1.4,sd)*8)*k],-1))
def via(x,y,r=1.9):   # the board's little vias: a gold ring round a dark drill
    over(np.hypot(xx-x,yy-y)<=r,(206,176,96));over(np.hypot(xx-x,yy-y)<=r*0.5,(16,30,20))
def part(x0,y0,x1,y1,body=(26,26,28)):   # a tiny two-pad part: tin ends, dark body between
    m=rr4(x0,y0,x1,y1,0.4);shadow(m,0.6,0.8,0.6,0.45)
    vert=(y1-y0)>(x1-x0);e=0.32
    over(m,(196,198,200))
    if vert: b=rr4(x0,y0+(y1-y0)*e,x1,y1-(y1-y0)*e,0.2)
    else: b=rr4(x0+(x1-x0)*e,y0,x1-(x1-x0)*e,y1,0.2)
    over(b,body)
    over(m&(np.minimum(yy-y0,xx-x0)<0.5),(236,238,240),0.6)
# -- the gold strip along the top edge, either side of the middle post (where the cart bay's
# metal plate presses down on the board), the bite between them
for a,b_ in ((152,290),(312,447)):
    x0,y0=Q(a,48);x1,y1=Q(b_,62);m=rr4(x0,y0,x1,y1,0.8)&m4;gold(m,4)
    over(m&(yy<y0+0.7),(232,206,130));over(m&(yy>y1-0.7),(150,120,62))
# -- left: the vias by R21, its five resistors in a row, the group of parts below them
for x,y in ((180,65),(187,65),(233,67),(191,83),(215,85),(206,96),(170,82),(250,98)): via(*Q(x,y),2.4)
for i in range(5):
    x0,y0=Q(190+i*7,70);part(x0,y0,x0+4,y0+8)
    over(rr4(x0-1,y0-1,x0+5,y0+9,0.8)&~rr4(x0-0.4,y0-0.4,x0+4.4,y0+8.4,0.5),(206,220,210),0.8)
for i in range(2):
    for j in range(3):
        x0,y0=Q(166+i*19,88+j*8);part(x0,y0,x0+10,y0+4.2,(70,54,40) if i else (26,26,28))
# the gold pad by the left edge
x0,y0=Q(131,89);gold(rr4(x0,y0,x0+11,y0+11,0.6),5)
# -- middle: the unpopulated 2x12 pad field under the gold strip, R25..R28's parts beside it
x0,y0=Q(281,64)
over(rr4(x0-1.2,y0-1.2,x0+37.2,y0+11.2,1.4)&~rr4(x0-0.5,y0-0.5,x0+36.5,y0+10.5,1.0),(206,220,210),0.9)
for i in range(12):
    for j in range(2): gold(rr4(x0+1+i*3,y0+1+j*5.4,x0+2.6+i*3,y0+3.8+j*5.4,0.3),6)
# -- the white serial sticker's top, below the middle
x0,y0=Q(253,92);st=rr4(x0,y0,x0+102,y0+30,0.6);shadow(st,0.8,1.0,1.0,0.4);over(st,(238,238,232))
# -- right: C55's via, two caps, C33/C34
for x,y in ((375,80),(387,98),(438,98),(452,92)): via(*Q(x,y),2.4)
# -- the photo's frame flipped to the real board's
L4=L4[:,::-1].copy();B=B[:,::-1].copy();board=board[:,::-1].copy()
# -- silkscreen, in the boards' white, the right way round: placed where the photo's mirrored
# text sits (its left end in photo px), so here it ends there
tl=Image.new('L',(W*K,H*K),0);td=ImageDraw.Draw(tl)
FB='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
def silk(txt,px,py,sz,w=None):
    f=ImageFont.truetype(FB,int(sz*K));tw=td.textlength(txt,font=f)/K
    x_,y_=Q(px,py);td.text(((W-x_-tw)*K,y_*K),txt,font=f,fill=255);return W-x_-tw,tw
for txt,px,py,sz in (('C/AGS-CPU-10',358,66,7.4),('3-2',330,67,6.2),('R21',176,72,5.2),
                     ('R22',229,80,5.2),('C55',368,86,5.2)):
    silk(txt,px,py,sz)
# (no Nintendo logo on it, Roy 2026-11-06)
over(np.asarray(tl,np.float32)/255>0.5,(206,220,210),0.9)
sw=np.asarray(gaussian_filter(L4,(1.0,1.0,0)))
sw=np.asarray(Image.fromarray(np.clip(sw,0,255).astype(np.uint8),'RGBA').resize((W,H),Image.LANCZOS),np.float32)
al=sw[...,3:]/255;B=B*(1-al)+sw[...,:3]*al
board=board|(al[...,0]>0.7)
if os.environ.get('BOARD_ONLY'):
    Image.fromarray(np.dstack([np.clip(B,0,255),board*255]).astype(np.uint8),'RGBA').save(os.environ['BOARD_ONLY']);raise SystemExit

# ---- the cart bay's metal plate: under the cart, over the board. In the mouth it is the plate
# the finger scoop shows (tools/slot_art.py METAL_TOP); it runs on past the mouth into the case
# on both sides (Roy 2026-11-06), seen there through the clear plastic, over the board
base=np.array([236,236,230],np.float32)
rng=np.random.default_rng(3)
v_=np.linspace(0,1,H)[:,None]
# brushed steel like the GBA slot's own plate (tools/slot_art.py), its sheen low across it
metal=np.clip(150-16*v_+22*np.exp(-((v_-0.70)/0.14)**2)+np.zeros((1,W))+gaussian_filter(rng.normal(0,1,(H,W)),(0.3,14))*4,0,255)
M=np.dstack([metal*0.97,metal,metal*1.03])
PL0,PL1=PX0-PLATE_OUT,PX1+PLATE_OUT
yf,xf=np.mgrid[0:H,0:W].astype(np.float32)+0.5
pr=3.0
pd=np.hypot(np.maximum(np.maximum(PL0+pr-xf,xf-(PL1-pr)),0),np.maximum(PLATE_TOP+pr-yf,0))
pcov=np.clip(pr-pd+0.5,0,1)*(xf>PL0-0.5)*(xf<PL1+0.5)*(yf>PLATE_TOP-0.5)
# (its top edge straight across, Roy 2026-11-06)
plate=pcov>0.5
# the plate's edges: lit along its top and left, in shade on the right
lit=np.clip(1-np.abs(pd-(pr-0.8))/0.9,0,1)*pcov
M2=M+(lit*np.clip(1.0-(xf-PL0)/(PL1-PL0)*0.9,0.2,1)*70)[...,None]
M2=M2*(1-0.25*np.clip(1-(PL1-xf)/1.5,0,1)*pcov)[...,None]
cast=gaussian_filter(np.roll(np.roll(pcov,2,0),2,1),1.8)*(1-pcov)
B=B*(1-0.55*cast[...,None])
inner=B*(1-pcov[...,None])+M2*pcov[...,None]
Ish=inner.copy()
Ish=gaussian_filter(Ish,(0.45,0.45,0))*0.96+6
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
    if not front:   # the mouth's side walls, seen edge-on through the back
        xs=np.arange(W)[None,:].repeat(H,0)
        dd=np.minimum(dd,np.where(xs<PX0+RIM_W+1,np.abs(xs-PX0)+1,99));dd=np.minimum(dd,np.where(xs>PX1-RIM_W-2,np.abs(PX1-1-xs)+1,99))
    # soft, rounded thick clear edges like the Atomic Purple GBC (Roy 2026-11-05: the hard
    # bands read as blocks; a Game Boy feels soft in the hand)
    dd=np.where(solid,gaussian_filter(dd,1.4),dd);dd0=dd
    u=np.clip((dd-0.5)/RIM_W,0,1)
    dens=(1-u)**1.6;glint=np.exp(-((u-0.20)/0.13)**2)
    t=np.where(solid,np.maximum(t,(0.18+0.62*dens)*a),t)
    rimc=np.array([176,182,180],np.float32)+dens[...,None]*np.array([12,12,12],np.float32)+glint[...,None]*(np.array([244,246,244],np.float32)-176)*0.85
    w=np.clip(dens*1.4+glint,0,1)[...,None]
    # the glint is one unbroken line: dense enough that nothing behind breaks it up (Roy: gaps)
    t=np.where(solid,np.maximum(t,(0.25+0.65*np.maximum(dens,glint))*a),t)
    rgb=np.where(solid[...,None],rgb*(1-w)+rimc*w,rgb)
    soft=np.exp(-(dd0-1).clip(0)/6.0)*solid
    t=np.maximum(t,(0.16+0.30*soft)*a);rgb=rgb+soft[...,None]*np.array([28,28,28],np.float32)
    global RIMW;RIMW=gaussian_filter(w[...,0]*solid,0.6)
    rgb=gaussian_filter(rgb,(1.2,1.2,0));t=gaussian_filter(t,1.0)
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
# the clear case: the SP's own clear (Roy's photo); the GBA's clear launch colours can follow
CLEAR={'clear':(1.0,1.0,1.0)}   # Glacier (0.76,0.90,1.04) and Fuchsia (1.04,0.70,0.88) later
OUT=os.environ.get('OUT','assets')
def save(name,tint):
    tint=np.array(tint,np.float32)
    smoke=np.array([150,156,154],np.float32)*tint
    rgb,t,a=layer(False)
    sw_=(np.clip((0.55-t)/0.15,0,1)*(1-RIMW))[...,None]
    rgb=np.where(behind[...,None],rgb,rgb*(1-sw_)+smoke*sw_)
    # in the mouth, below where the plastic back wall stops (METAL_TOP), the plate lies bare:
    # no plastic over it. The wall stops in a 1 mm step down (Roy 2026-11-06): its cut edge a
    # dense lit band, its shadow soft on the metal just under it
    inm=(xf>=PX0)&(xf<PX1)&(a>0.5)
    t=np.where(inm&(yf>=MT),0,t)
    STEP=4.0
    e=np.clip(1-np.abs(yf-(MT-STEP/2))/(STEP/2),0,1)*inm*(yf<MT)
    rgb=rgb*(1-e[...,None]*0.6)+np.array([238,242,240],np.float32)*tint*e[...,None]*0.6
    t=np.maximum(t,e*0.75)
    tb=1-(1-t)*SEE_THROUGH   # st7: a little less see-through (Roy 2026-11-12)
    col=np.where(behind[...,None],rgb*tb[...,None]+(1-tb[...,None])*Ish,rgb)
    shade=np.clip(1-(yf-MT)/3.0,0,1)*inm*(yf>=MT)*0.35
    col=col*(1-shade[...,None])
    al=np.where(behind,a,t)
    col=np.where(behind[...,None],col*tint,col*(0.55+0.45*tint))   # the plastic's colour over what it shows
    # the two slide ribs on the back wall (where tools/slot_art.py puts the GBA's), glossy clear,
    # down to the plate
    for cx in (PX0+(PX1-PX0)*0.30,PX0+(PX1-PX0)*0.70):
        r=3.0;top=9.0+r
        d=np.hypot(xf-cx,np.maximum(top-yf,0));m=np.clip(r-d+0.5,0,1)*(yf<MT-0.5)
        nx=np.clip((xf-cx)/r,-1,1);ny=np.clip((top-yf)/r,0,1)
        hl=np.clip(-nx*0.8+ny*0.6,0,1)**2;sh=np.clip(nx,0,1)**1.5
        col=col*(1-0.30*sh*m)[...,None]+(m*hl*0.75)[...,None]*(np.array([246,248,246],np.float32)-col)
        col=col+(m*0.10)[...,None]*(np.array([214,220,218],np.float32)-col)
        sd=np.clip(1-np.abs(xf-cx-r-1.2)/1.6,0,1)*(yf>top-1)*(yf<MT-0.5)*(1-m);col=col*(1-0.18*sd)[...,None]
        al=np.maximum(al,np.maximum(m*0.55,sd*0.2))
    Image.fromarray(np.clip(np.dstack([col,al*255]),0,255).astype(np.uint8),'RGBA').save(f'{OUT}/slot_back_gba_{name}.png')
    rgb,t,a=layer(True)
    rgb=rgb*(0.45+0.55*tint)
    Image.fromarray(np.clip(np.dstack([rgb,np.maximum(t,0.05*a)*255]),0,255).astype(np.uint8),'RGBA').save(f'{OUT}/slot_front_gba_{name}.png')

for name,tint in CLEAR.items(): save(name,tint)
