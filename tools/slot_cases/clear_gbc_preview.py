import numpy as np
from PIL import Image,ImageDraw,ImageFilter
from scipy.ndimage import gaussian_filter
import os
# The clear Game Boy Colors (Roy 2026-11-06: all the original colours): Atomic Purple's look,
# every colour in it scaled by the case's tint over Atomic Purple's own.
TINTS={'neutral':(1.0,1.0,1.0),'atomic':(0.84,0.74,1.0),'clear':(0.96,0.97,0.97),'clear_black':(0.30,0.30,0.33),
       'clear_green':(0.62,0.92,0.62),'midnight_blue':(0.40,0.48,0.88),'water_blue':(0.66,0.86,1.0),
       'ice_blue':(0.80,0.92,1.0)}
NAME=os.environ.get('GBC_CASE','atomic')
F=np.array(TINTS[NAME],np.float32)/np.array(TINTS['atomic'],np.float32)
def C(v): return np.clip(np.array(v,np.float32)*F,0,255)
src=open('tools/slot_art.py').read().split('MOUTHS = ')[0].replace('ROOT = Path(__file__).resolve().parents[1]','ROOT=None')
# the GBC mouth is shallow like the Pocket's: sides ~1 mm above it (Roy 2026-11-05), so the
# same flattening as tools/slot_cases/flat_gb.py
for _a,_b in (("WALL = 2.2","WALL = 0.5"),("INNER = 5.5","INNER = 1.0"),
    ("cast = 0.50 * np.exp(-(d_out - WALL).clip(0) / 3.5)","cast = 0.08 * np.exp(-(d_out - WALL).clip(0) / 1.0)"),
    ("deep = deep * (0.80 + 0.20 * smooth(INNER, INNER + 8.0, d_out_px))","deep = deep * (0.98 + 0.02 * smooth(INNER, INNER + 2.0, d_out_px))"),
    ("side_shade = left_w * (0.46 + 0.12 * t) + (1 - left_w) * (0.70 - 0.06 * t)","side_shade = left_w * (0.86 + 0.06 * t) + (1 - left_w) * (0.93 - 0.02 * t)"),
    # very shallow: a much lighter shadow at the top of the back wall, and its top edge rounds
    # over like the housing's own top edge instead of a flat band (Roy 2026-11-05)
    ("deep = 0.60 + 0.12 * smooth(3.0, 30.0, yy)","deep = 0.94 + 0.02 * smooth(TOP_BEVEL, 30.0, yy)"),
    ("deep = np.where(rim, 0.92 - 0.10 * (yy / 2.5), deep)","deep = np.where(yy < TOP_BEVEL + 1, deep * shade, deep)"),
    # the ribs' round tops catch the light from above instead of going dark, and their shadow
    # follows that round top instead of starting square (Roy 2026-11-05: odd lines at the top)
    ("rib_shade = np.where(dy > 0, 1.0 - 0.25 * np.clip(-(yy - top) / r, 0, 1), 1.0)","rib_shade = np.where(dy > 0, 1.0 + 0.08 * np.clip(-(yy - top) / r, 0, 1), 1.0)"),
    ("shadow = gap & ~rib & (xs_ > cx) & (xs_ < cx + r + 2.5) & (yy > RIB_TOP + 1)","shadow = gap & ~rib & (xs_ > cx) & (np.hypot(xs_ - cx - 1.4, np.maximum(top + 0.8 - yy, 0)) < r + 1.0)")):
    assert src.count(_a)==1,_a;src=src.replace(_a,_b)
# shallow mouths: the rims run under the seated pak, a few px in on each side (Roy 2026-11-05)
src=src.replace('GB_MOUTH = (244, 476)','GB_MOUTH = (250, 470)')
g={'__file__':'tools/slot_art.py'};exec(src,g)
mouth=g['gb_mouth']();W,H=720,58
base=np.array([236,236,230],np.float32)
rng=np.random.default_rng(3)
# the plate in the cart parts' metal: a clean, smooth gradient with one soft sheen (as the cart
# boards' resistors and battery), no streaks
v_=np.linspace(0,1,H)[:,None]
metal=np.clip(196-46*v_+30*np.exp(-((v_-0.3)/0.13)**2)+np.zeros((1,W))+gaussian_filter(rng.normal(0,1,(H,W)),(0.4,10))*1.5,0,255)
M=np.dstack([metal*0.97,metal,metal*1.03])
PX0,PX1=250,470; BX0,BX1=190,530
NX0,NY1=426,34                      # the plate's cut-out for the power switch, top right

# ---- Atomic Purple Game Boy Color (Roy 2026-11-05: the most important GBC). Laid out after
# Roy's photos (previews/slot-studio/gbc-ref/): the top of an Atomic Purple seen from above
# (gbc-atomic-purple-slot.jpg, x scaled onto the strip: photo x 30..915 -> 0..720) for where
# things sit, and the CGB-CPU-01 board (gbc-printplaat-achter.jpg) for what they look like:
# one board across, its top edge a little below the ribbon's; the copper ribbon cable over the
# white FPC connector in the middle, both the same width and lined up on both sides (Roy); the
# gold mounting pads with their holes by the screws; R/C parts and vias on the left; the IR
# LEDs (violet and black) with their tin legs, their white bodies and the black holder on the
# right. Made like the cart boards (tools/carts/tv_boards.py build(): the GBA board's green, its
# cut rim and drop shadow, the real copied two-pad parts and vias).
_tb=open('tools/carts/tv_boards.py').read().split('# ---- SNES')[0]
_g={'__file__':'tools/carts/tv_boards.py'};exec(_tb,_g)
from scipy.ndimage import distance_transform_edt as edt
X=lambda px:(px-30)*720/885
# Roy's photo of the board in its shell (gbc-ref/gbc-bord-in-behuizing.jpg): the board about as
# wide as the cart connector; the ribbon (~half the mouth, Roy) over the board's top edge into
# the white connector just below it; the IR LEDs on the top edge at the right; a gold pad top left
# Measured off that photo at its own scale: a GBC cart (57 mm) is ~236 photo px and the mouth
# 232 strip px, so ~1 strip px per photo px, centred on the board (photo x 242 -> strip 360).
# The board, as wide as the shell inside, so well past the mouth on both sides (Roy).
CX0,CX1=294,426                              # the connector
RX0,RX1=CX0+9,CX1-9                          # the ribbon: the same inset each side (Roy)
BT=31                                        # the board's top edge
# about as wide as the cart connector: a hair past the mouth each side (Roy: not the full width)
m4=_g['rr'](H,W,(210,BT,513,H+8),2)
P=_g['PART'][13:27]                          # R5's part without its name
parts=[(P,266,42),(P,486,32)]
vias=[(252,38),(288,36),(432,36)]
traces=[[(262,44),(266,50),(CX0-2,50)],[(CX1+2,40),(420,40)]]
bd=_g['build'](m4,[],[],parts,vias,traces).astype(np.float32)
B=bd[...,:3].copy();board=bd[...,3]/255>0.5
K=4
yy,xx=np.mgrid[0:H*K,0:W*K]/K
L4=np.zeros((H*K,W*K,4),np.float32)
def over(mask,rgb,a=1.0):
    m=mask.astype(np.float32)[...,None]*a
    L4[...,:3]=L4[...,:3]*(1-m)+np.asarray(rgb,np.float32)*m;L4[...,3:]=np.maximum(L4[...,3:],m*255)
def rr4(x0,y0,x1,y1,r): return np.hypot(np.maximum(np.maximum(x0+r-xx,xx-(x1-r)),0),np.maximum(np.maximum(y0+r-yy,yy-(y1-r)),0))<=r
def shadow(mask,dx=1.0,dy=1.4,s=1.3,a=0.55):
    sh=gaussian_filter(np.roll(np.roll(mask.astype(np.float32),int(dy*K),0),int(dx*K),1),s*K)*(1-mask)
    L4[...,3]=np.maximum(L4[...,3],sh*255*a);L4[...,:3]*=(1-sh*a)[...,None]
nz=lambda s,sd:gaussian_filter(np.random.default_rng(sd).normal(0,1,(H*K,W*K)),s)
def gold_pad(cx,cy,r):         # a mounting pad: tinned gold square, its drill dark through
    m=rr4(cx-r,cy-r,cx+r,cy+r,0.8)
    over(m,np.stack([206+nz(1.2,4)*12,168+nz(1.2,4)*10,74+nz(1.2,4)*8],-1))
    over(m&(np.minimum.reduce([xx-cx+r,yy-cy+r])<0.6),(232,206,120))
    d=np.hypot(xx-cx,yy-cy);over(d<=r*0.62,(196,170,96));over(d<=r*0.42,(12,22,16))
# the big gold mounting pad top left, its hole; the small square pad beside it
m=rr4(222,48,235,H+2,0.6);over(m,np.stack([206+nz(1.2,4)*12,168+nz(1.2,4)*10,74+nz(1.2,4)*8],-1))
d=np.hypot(xx-228.5,yy-58);over(d<=3.2,(196,170,96));over(d<=2.1,(12,22,16))
m=rr4(237,49,245,57,0.5);over(m,np.stack([206+nz(1.2,5)*12,168+nz(1.2,5)*10,74+nz(1.2,5)*8],-1));over(np.hypot(xx-241,yy-53)<=1.2,(30,40,28))
# -- the copper ribbon: fine lengthwise traces, a soft sheen across, darker at its sides
rib=rr4(RX0,10,RX1,49,0.6);shadow(rib,1.0,1.4,1.6,0.5)
u=(xx-RX0)/(RX1-RX0);v=np.clip((yy-10)/39,0,1)
lines=0.80+0.20*(0.5+0.5*np.cos(2*np.pi*(xx-RX0)/2.4))
sheen=1+0.30*np.exp(-((v-0.30)/0.14)**2)+0.12*np.exp(-((v-0.75)/0.1)**2)
side=1-0.25*np.clip(1-np.minimum(u,1-u)/0.04,0,1)
cu=np.stack([214*lines*sheen*side,122*lines*sheen*side,64*lines*sheen*side],-1)+nz((0.3*K,3*K),7)[...,None]*5
over(rib,cu)
# -- the white FPC connector the ribbon goes into: its dark latch bar on top, contacts below
con=rr4(CX0,46,CX1,H+4,0.8);shadow(con,1.0,1.4,1.4,0.5)
cv=np.clip((yy-46)/12,0,1);over(con,np.stack([236-24*cv,236-24*cv,228-24*cv],-1))
over(rr4(CX0+2,46,CX1-2,48.5,0.4),(52,52,56))   # the latch bar the ribbon goes under   # its dark lower lip
for x in np.arange(CX0+4,CX1-3,1.6):
    over(rr4(x,52,x+0.7,H+2,0.1),(128,130,134))
over(rr4(CX0,46,CX0+4,H+4,0.6)|rr4(CX1-4,46,CX1,H+4,0.6),(150,150,146))   # its latch ends   # the latch ends
# -- right: the IR window's black holder, its violet and black LEDs on tin legs, white bodies
HX0,HX1=417,497
# (no black IR holder under the top edge: through the clear rim it broke the glint into gaps, Roy)

# two 3 mm LEDs standing up off the board's top edge (violet sender, black receiver), each on its
# two tin legs, a white plastic bar across the legs (as in Roy's photo)
# the white spacer block first: the LED legs run over it, then down to their solder joints
# the LED group a bit smaller and further left, inside the mouth, its tops level with the
# ribbon's (Roy 2026-11-05): LS scales it about the LEDs' bottom (y 27), GX its left end
LS,GX=0.8,431.0
def lx(x): return GX+(x-440)*LS
def ly(y): return 27-(27-y)*LS
bar=rr4(lx(440),40,lx(480),48,1.0);shadow(bar);over(bar,np.stack([238-26*np.clip((yy-40)/8,0,1)]*3,-1))
for (x0,c) in ((441,(110,84,190)),(463,(28,28,32))):
    for dx in (1.4,12.2):   # two tin legs, out at the LED's outer edges (Roy)
        x=lx(x0+dx);lg=rr4(x,26,x+1.4*LS,55,0.5);shadow(lg,0.8,1.0,0.8,0.45);over(lg,np.stack([172+44*np.clip(1-abs(xx-x-0.7*LS)/(0.7*LS),0,1)]*3,-1))
        sj=np.hypot(xx-x-0.7,(yy-55)*1.3)<=4.0;shadow(sj,0.6,0.8,0.8,0.4)   # a bigger solder joint
        over(sj,np.stack([140+90*np.clip(1-np.hypot(xx-x+0.2,yy-54)/3.5,0,1)]*3,-1))
    # a classic 5 mm LED: round dome on a straight body, the wider collar at its foot; its
    # bottom ~4 px above the board (Roy)
    r=7.5*LS;cx=lx(x0)+r;body=(np.hypot(xx-cx,yy-ly(12.5))<=r)|rr4(lx(x0),ly(12.5),lx(x0)+2*r,ly(25),0.3)
    collar=rr4(lx(x0)-1.2*LS,ly(24),lx(x0)+2*r+1.2*LS,27,0.6);m=body|collar;shadow(m,1.0,1.2,1.2,0.5)
    shade=(0.70+0.45*np.clip(1-abs(xx-cx+2.2*LS)/(8*LS),0,1))[...,None]
    over(m,np.stack([np.full_like(xx,c[0]),np.full_like(xx,c[1]),np.full_like(xx,c[2])],-1)*shade)
    over(collar,np.stack([np.full_like(xx,c[0]),np.full_like(xx,c[1]),np.full_like(xx,c[2])],-1)*0.8)
    over(rr4(lx(x0+3),ly(8),lx(x0+5),ly(22),1.0),(236,232,252),0.55)   # the dome's highlight
# the copper-brown pad top right, its hole
m=rr4(490,46,513,H+2,0.6);over(m,np.stack([170+nz(1.2,6)*10,104+nz(1.2,6)*8,82+nz(1.2,6)*6],-1))
d=np.hypot(xx-501.5,yy-59);over(d<=4.2,(150,96,76));over(d<=2.8,(12,22,16))
# -- silkscreen names off the CGB-CPU-01 board, in its own white
from PIL import ImageFont
tl=Image.new('L',(W*K,H*K),0);td=ImageDraw.Draw(tl)
FB='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
for txt,x_,y_ in (('R10',268,33.5),):
    td.text((x_*K,y_*K),txt,font=ImageFont.truetype(FB,int(6.4*K)),fill=255)
over(np.asarray(tl,np.float32)/255>0.5,(206,220,210),0.9)
sw=np.asarray(gaussian_filter(L4,(1.0,1.0,0)))
sw=np.asarray(Image.fromarray(np.clip(sw,0,255).astype(np.uint8),'RGBA').resize((W,H),Image.LANCZOS),np.float32)
al=sw[...,3:]/255;B=B*(1-al)+sw[...,:3]*al
board=board|(al[...,0]>0.7)
import os
if os.environ.get('BOARD_ONLY'):
    Image.fromarray(np.dstack([np.clip(B,0,255),board*255]).astype(np.uint8),'RGBA').save(os.environ['BOARD_ONLY']);raise SystemExit
plate=np.zeros((H,W),bool)
Ish=B.copy()
# seen through slightly frosted purple plastic: the parts a hair soft and hazy
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
    rimc=C([138,116,188])+dens[...,None]*np.array([16,14,10],np.float32)+glint[...,None]*(C([232,222,250])-138)*0.85
    w=np.clip(dens*1.4+glint,0,1)[...,None]
    # the glint is one unbroken line: dense enough that nothing behind breaks it up (Roy: gaps)
    t=np.where(solid,np.maximum(t,(0.25+0.65*np.maximum(dens,glint))*a),t)
    # the shell is soft in the hand: every edge rounds off in a broad, gentle falloff of
    # light into the face, not a block with a line around it (Roy 2026-11-05)
    soft=np.exp(-(dd0-1).clip(0)/6.0)*solid
    t=np.maximum(t,(0.16+0.30*soft)*a);rgb=rgb+soft[...,None]*C([30,26,36])
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
smoke=C([118,96,170])       # Atomic Purple against nothing
rgb,t,a=layer(False)
# the mouth's edges keep the same round glint as the housing's (Roy 2026-11-05)
# smoke fades in smoothly below t 0.5: a hard threshold flipped pixels along the glint (Roy: gaps)
sw_=(np.clip((0.55-t)/0.15,0,1)*(1-RIMW))[...,None]
rgb=np.where(behind[...,None],rgb,rgb*(1-sw_)+smoke*sw_)
PURPLE=np.clip(np.array([0.88,0.80,1.0],np.float32)*F,0,1.1)   # what the purple plastic lets through
tb=1-(1-t)*SEE_THROUGH   # st7: a little less see-through (Roy 2026-11-12)
col=np.where(behind[...,None],(rgb*tb[...,None]+(1-tb[...,None])*Ish)*PURPLE,rgb)
al=np.where(behind,a,t)
# the two vertical slide ribs on the back wall (where tools/slot_art.py puts them), in smooth
# glossy clear like the embossed letters on the clear carts (Roy 2026-11-05)
yf,xf=np.mgrid[0:H,0:W].astype(np.float32)+0.5
for cx in (PX0+(PX1-PX0)*0.24,PX0+(PX1-PX0)*0.76):
    r=3.0;top=9.0+r
    d=np.hypot(xf-cx,np.maximum(top-yf,0));m=np.clip(r-d+0.5,0,1)
    nx=np.clip((xf-cx)/r,-1,1);ny=np.clip((top-yf)/r,0,1)
    hl=np.clip(-nx*0.8+ny*0.6,0,1)**2;sh=np.clip(nx,0,1)**1.5
    col=col*(1-0.30*sh*m)[...,None]+(m*hl*0.75)[...,None]*(C([236,226,252])-col)
    col=col+(m*0.10)[...,None]*(C([200,180,240])-col)
    sd=np.clip(1-np.abs(xf-cx-r-1.2)/1.6,0,1)*(yf>top-1)*(1-m);col=col*(1-0.18*sd)[...,None]
    al=np.maximum(al,np.maximum(m*0.55,sd*0.2))
Image.fromarray(np.clip(np.dstack([col,al*255]),0,255).astype(np.uint8),'RGBA').save('assets/slot_back_gbc_'+NAME+'.png')
rgb,t,a=layer(True)
rgb=rgb*np.array([0.84,0.74,1.0],np.float32)*F   # the purple shell
Image.fromarray(np.clip(np.dstack([rgb,np.maximum(t,0.05*a)*255]),0,255).astype(np.uint8),'RGBA').save('assets/slot_front_gbc_'+NAME+'.png')


# the top edge's glint runs unbroken across the mouth: its back wall top gets exactly the
# housing's own top profile (back and front layers over each other), so no gap (Roy 2026-11-05)
bk=np.asarray(Image.open('assets/slot_back_gbc_'+NAME+'.png'),np.float32)/255;fr=np.asarray(Image.open('assets/slot_front_gbc_'+NAME+'.png'),np.float32)/255
ref=PX0-24;R=7          # above the ribs' round tops (y 9)
fa=fr[:R,ref,3:];ba=bk[:R,ref,3:];oa=fa+ba*(1-fa)
orgb=(fr[:R,ref,:3]*fa+bk[:R,ref,:3]*ba*(1-fa))/np.maximum(oa,1e-6)
prof=np.concatenate([orgb,oa],1)[:,None,:]
wgt=np.clip((R-np.arange(R))/3.0,0,1)[:,None,None]
bk[:R,PX0:PX1]=bk[:R,PX0:PX1]*(1-wgt)+prof*wgt
Image.fromarray(np.clip(bk*255,0,255).astype(np.uint8),'RGBA').save('assets/slot_back_gbc_'+NAME+'.png')
