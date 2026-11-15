import os
import numpy as np
from PIL import Image,ImageDraw,ImageFilter
from scipy.ndimage import gaussian_filter
src=open('tools/slot_art.py').read().split('MOUTHS = ')[0].replace('ROOT = Path(__file__).resolve().parents[1]','ROOT=None')
g={'__file__':'tools/slot_art.py'};exec(src,g)
mouth=g['gb_mouth']();W,H=720,58
base=np.array([236,236,230],np.float32)
rng=np.random.default_rng(3)
# the plate in the cart parts' metal: a clean, smooth gradient with one soft sheen (as the cart
# boards' resistors and battery), no streaks
v_=np.linspace(0,1,H)[:,None]
metal=np.clip(196-46*v_+30*np.exp(-((v_-0.3)/0.13)**2)+np.zeros((1,W))+gaussian_filter(rng.normal(0,1,(H,W)),(0.4,10))*1.5,0,255)
M=np.dstack([metal*0.97,metal,metal*1.03])
PX0,PX1=244,476; BX0,BX1=PX0-54,PX1+54
NX0,NY1=PX1-50,34                      # the plate's cut-out for the power switch, top right

# The board is built the way the cart boards are (tools/carts/tv_boards.py build()): the GBA
# board's own mottled green, its cut rim and drop shadow, real parts copied off the Game Boy and
# GBA boards (the 8-pin chip, vias), traces drawn at 4x in the boards' trace green. Laid out after
# Roy's DMG-CPU top-band photo: link-port bracket and chip on the left, the power switch at the
# top edge under the slot's lock, the board stepping down on the right.
_tb=open('tools/carts/tv_boards.py').read().split('# ---- SNES')[0]
_g={'__file__':'tools/carts/tv_boards.py'};exec(_tb,_g)
from scipy.ndimage import distance_transform_edt as edt
# Laid out exactly after Roy's photo of a DMG-CPU board (photo px -> strip by Q: scale 0.18,
# the photo's slight tilt taken out, shifted so SW1's middle sits under the slot's lock).
S_=0.18;SC=464.5
def Q(px,py): return SC+(px-1675)*S_, 10+(py-165-(px-220)*0.03)*S_
# SW1 lines up with the plastic slider bar: its top on the bar's bottom (strip row 18,
# slot_chrome.rs BAND_Y+18), its right end on the bar's right end (strip 509.5); the board
# reaches 8px past it at the top and on the right (Roy)
SWT=18;MARG=8
BX0=round(Q(220,0)[0]);BR=508.75+MARG
BT=SWT-MARG
m4=_g['rr'](H,W,(BX0,BT,BR,H+8),2)
T=lambda pts:[Q(*p) for p in pts]
traces=[[(228,53),(244,53)],[(236,53),(236,58)],[(203,38),(203,46)],
        [(429,18),(429,31),(434,36)],[(479,52),(492,52),(498,58)],[(486,49),(486,46)]]
vias=[]
bd=_g['build'](m4,[],[],[],vias,traces).astype(np.float32)
B=bd[...,:3].copy();bsoft=bd[...,3]/255;board=bsoft>0.5
# the Game Boy cart board's own small vias (a cream ring round a silver drill), copied over
GBV=_g['gb'][18:25,86:93]
for x_,y_ in ((228,53),(236,51),(203,37),(429,17),(429,32),(486,48),(498,54),(479,54)):
    _g['paste'](B,GBV,x_-3,y_-3)
# The parts, at 4x in strip pixels, in the cart boards' own look: navy chips with a soft
# gradient and silver legs, brushed metal with a sheen, gold/tin pads, white silkscreen.
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
def silk_box(x0,y0,x1,y1,w=0.7):
    o=rr4(x0,y0,x1,y1,1.8)&~rr4(x0+w,y0+w,x1-w,y1-w,1.4);over(o,(206,220,210),0.9)
# the cart chips' navy, a soft vertical gradient (sampled off the Game Boy board's chips)
chip=_g['gb'][180:258,82:118,:3].mean((1)) if False else None
def navy(x0,y0,x1,y1,r=0.7):
    m=rr4(x0,y0,x1,y1,r);v=np.clip((yy-y0)/max(y1-y0,1),0,1)
    shadow(m);over(m,np.stack([10+8*v,22+10*v,40+14*v],-1))
    over(m&(np.minimum.reduce([xx-x0,yy-y0])<0.5),(40,56,82))
def silver_disc(cx,cy,r):
    m=np.hypot(xx-cx,yy-cy)<=r;ang=np.arctan2(yy-cy,xx-cx)
    t=170+42*np.cos(2*(ang+0.6))+gaussian_filter(np.random.default_rng(2).normal(0,1,(H*K,W*K)),(0.3*K,2*K))*6
    over(m,np.stack([t,t+2,t+5],-1))
# -- C1, the big electrolytic, seen from above: black sleeve, scored silver top
# whole and round, fully below the shell's top rim (strip row 10); it may overhang the board (Roy)
cx,cy=Q(315,0)[0],20.9;silk=np.hypot(xx-cx,yy-cy);over((silk<=12.6)&(silk>=12.0)&(yy>=10.6),(206,220,210),0.9)
sl=np.hypot(xx-cx,yy-cy)<=10.2;shadow(sl,1.2,1.6,1.6,0.6)
over(sl,np.stack([30+14*np.clip((cx-xx)/11,0,1)]*3,-1)*np.array([1,1,1.15]))
silver_disc(cx,cy,8.2)
for ang in (0.25,1.82):   # the vent score, a K across the top
    d_=np.abs((xx-cx)*np.sin(ang)-(yy-cy)*np.cos(ang));over((d_<0.35)&(np.hypot(xx-cx,yy-cy)<6.8),(120,124,130))
# -- DA1..DA4: four black diode packs in one silkscreen frame
x0,y0=Q(380,225);x1,y1=Q(642,392);silk_box(x0,y0,x1,y1)
for px in (385,450,515,578):
    a0,b0=Q(px,232);a1,b1=Q(px+56,385);navy(a0,b0,a1,b1,0.6)
# -- the left mounting pad: bare tin with its screw hole
a0,b0=Q(222,332);a1,b1=Q(335,440)
pad=rr4(a0,b0,a1,b1,0.8)
over(pad,np.stack([182+gaussian_filter(np.random.default_rng(9).normal(0,1,(H*K,W*K)),1.5)*22]*3,-1))
cx,cy=Q(285,385);hr=np.hypot(xx-cx,yy-cy);over(pad&(hr<=5.0),(214,218,214));over(hr<=3.4,(14,40,24))
# SW1's top lines up with the bottom of the slot's lock tab (strip row 18: GB_SINK + POWER_TAB_DY
# + its 6 rows, slot_chrome.rs) (Roy)
# and its right end lines up with the right end of the switch's plastic slider bar, off (Roy):
# slider right = wall + 11.5 + 27 on screen (slot_chrome.rs draw_power_slider) = strip 509.5
_top=Q(1482,232)[1];DXR=509.5-Q(1872,0)[0]
QS=lambda px,py:(Q(px,py)[0]+DXR,Q(px,py)[1]+SWT-_top)   # can top = the board's top edge (Roy)
# -- SW1: silver can with its long channel, the black slider in it, the dark plastic end with
# the brass spring, the black knob standing up out of its top at the left
a0,b0=QS(1482,232);a1,b1=QS(1872,412)
x0,y0=QS(1470,222);x1,y1=QS(1885,420);silk_box(x0,y0,min(x1,BR-1.5),y1)
can=rr4(a0,b0,a1,b1,1.0);shadow(can,1.2,1.6,1.5,0.6)
v=np.clip((yy-b0)/(b1-b0),0,1)
ct=178+30*np.exp(-((v-0.22)/0.16)**2)-24*v+gaussian_filter(np.random.default_rng(5).normal(0,1,(H*K,W*K)),(0.3*K,1.6*K))*7
over(can,np.stack([ct,ct+2,ct+5],-1))
over(can&(np.minimum(yy-b0,xx-a0)<0.5),(236,238,240));over(can&(np.minimum(b1-yy,a1-xx)<0.5),(110,114,118))
# Roy's photo: a plain symmetric silver block with three dark cut-outs: a notch by the left
# edge, the long slider channel in the middle, a notch by the right edge
cw,chh=a1-a0,b1-b0
def hole(m):
    top=m&~np.roll(m,int(0.8*K),0)
    over(m,(16,16,18));over(top,(6,6,7))
ed=1.4/cw   # the notches stay closed at the sides: a thin wall of the ~2 mm sheet metal (Roy)
for (u0,u1,w0,w1) in ((ed,0.11+ed,0.36,0.64),(0.24,0.76,0.40,0.62),(0.89-ed,1-ed,0.36,0.64)):
    hole(rr4(a0+u0*cw,b0+w0*chh,a0+u1*cw,b0+w1*chh,0.4))
# -- the tinned mounting pad under SW1 and the little orange ceramic cap beside it
a0,b0=QS(1692,422);a1,b1=QS(1902,560)
a1=min(a1,BR+0.5)   # stays on the board
pad=rr4(a0,b0,a1,b1,0.8)
over(pad,np.stack([184+gaussian_filter(np.random.default_rng(10).normal(0,1,(H*K,W*K)),1.5)*22]*3,-1))
cx,cy=QS(1740,452);oc=np.hypot((xx-cx)/1.1,yy-cy)<=5.2;shadow(oc,1.0,1.2,1.0,0.5)
ov=np.clip(1-np.hypot(xx-cx+1.5,yy-cy+1.5)/7,0,1)
over(oc,np.stack([176+50*ov,96+40*ov,52+24*ov],-1))
# -- silkscreen names, the real ones and one of ours (Roy)
from PIL import ImageFont
tl=Image.new('L',(W*K,H*K),0);td=ImageDraw.Draw(tl)
FB='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
for txt,(x_,y_),sz in (('GR4VY',(BX0+1.5,32.0),7.4),):
    td.text((x_*K,y_*K),txt,font=ImageFont.truetype(FB,int(sz*K)),fill=255)
over(np.asarray(tl,np.float32)/255>0.5,(222,228,222),0.92)
sw=Image.fromarray(np.clip(L4,0,255).astype(np.uint8),'RGBA')

swa=np.asarray(gaussian_filter(np.asarray(sw,np.float32),(1.2,1.2,0)))   # soften at 4x, then down
swa=np.asarray(Image.fromarray(np.clip(swa,0,255).astype(np.uint8),'RGBA').resize((W,H),Image.LANCZOS),np.float32)
al=swa[...,3:]/255;B=B*(1-al)+swa[...,:3]*al
board=board|(al[...,0]>0.7)   # parts may overhang the board's edge (C1 over the top) (Roy)

plate=np.zeros((H,W),bool);plate[:,PX0:PX1]=True
plate[:5,:]=False;plate[:NY1+5,NX0:PX1]=False              # cut-out
# round the plate's corners, the outer ones and the cut-out's inner one, as stamped sheet (Roy)
from scipy.ndimage import binary_opening,binary_closing
_d=np.hypot(*np.mgrid[-2:3,-2:3])<=2.2
_p=np.pad(plate,((0,4),(0,0)),mode='edge')
plate=binary_closing(binary_opening(_p,_d),_d)[:H]
# plate edge: a thin bright rim round the cut-out
M2=M.copy();M2[NY1+4:NY1+6,NX0:PX1]=235;M2[5:NY1+5,NX0-1:NX0+1]=235;M2[5:7,PX0:NX0]=235
def sh(m,dy,dx): return np.roll(np.roll(m,dy,0),dx,1)
pf=plate.astype(np.float32)
lite=plate&~sh(plate,2,2)          # top/left edge catches the light
dark=plate&~sh(plate,-2,-2)        # bottom/right edge, in shade
M2[lite]=np.minimum(M2[lite]*0.4+160,252);M2[dark]=M2[dark]*0.45
# the plate's own thickness: a 1px face edge, then its shadow falling on the board (down and right)
cast=gaussian_filter(sh(pf,3,3),2.2)*(1-pf)
B=B*(1-0.62*cast[...,None])
inner=np.where(plate[...,None],M2,B)
Ish=inner.copy();Ish[plate]=M2[plate]*0.86;Ish=Ish*0.94+8
Ish[~plate&board]=B[~plate&board]
# seen through slightly frosted plastic: the board a hair soft and hazy
Ish=gaussian_filter(Ish,(0.45,0.45,0))*0.96+6
behind=board|plate                 # what sits under the clear plastic: plate and board
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
smoke=np.array([150,156,154],np.float32)
rgb,t,a=layer(False)
# smoke fades in smoothly below t 0.5: a hard threshold flipped pixels along the glint (Roy: gaps)
sw_=(np.clip((0.55-t)/0.15,0,1)*(1-RIMW))[...,None]
rgb=np.where(behind[...,None],rgb,rgb*(1-sw_)+smoke*sw_)
tb=1-(1-t)*SEE_THROUGH   # st7: a little less see-through (Roy 2026-11-12)
col=np.where(behind[...,None],rgb*tb[...,None]+(1-tb[...,None])*Ish,rgb)
al=np.where(behind,a,t)
# the two vertical slide ribs on the back wall, smooth glossy clear like the clear carts'
# embossed letters (Roy 2026-11-05)
yf,xf=np.mgrid[0:H,0:W].astype(np.float32)+0.5
for cx in (PX0+(PX1-PX0)*0.24,PX0+(PX1-PX0)*0.76):
    r=3.0;top=9.0+r
    d=np.hypot(xf-cx,np.maximum(top-yf,0));m=np.clip(r-d+0.5,0,1)
    nx=np.clip((xf-cx)/r,-1,1);ny=np.clip((top-yf)/r,0,1)
    hl=np.clip(-nx*0.8+ny*0.6,0,1)**2;sh=np.clip(nx,0,1)**1.5
    col=col*(1-0.30*sh*m)[...,None]+(m*hl*0.75)[...,None]*(np.array([246,248,246],np.float32)-col)
    col=col+(m*0.10)[...,None]*(np.array([214,220,218],np.float32)-col)
    sd=np.clip(1-np.abs(xf-cx-r-1.2)/1.6,0,1)*(yf>top-1)*(1-m);col=col*(1-0.18*sd)[...,None]
    al=np.maximum(al,np.maximum(m*0.55,sd*0.2))
Image.fromarray(np.clip(np.dstack([col,al*255]),0,255).astype(np.uint8),'RGBA').save('assets/slot_back_gb_clear.png')
rgb,t,a=layer(True)
Image.fromarray(np.clip(np.dstack([rgb,np.maximum(t,0.05*a)*255]),0,255).astype(np.uint8),'RGBA').save('assets/slot_front_gb_clear.png')

# the top edge's glint runs unbroken across the mouth: its back wall top gets exactly the
# housing's own top profile (back and front layers over each other), so no gap (Roy 2026-11-05)
bk=np.asarray(Image.open('assets/slot_back_gb_clear.png'),np.float32)/255;fr=np.asarray(Image.open('assets/slot_front_gb_clear.png'),np.float32)/255
ref=PX0-24;R=7          # above the ribs' round tops (y 9)
fa=fr[:R,ref,3:];ba=bk[:R,ref,3:];oa=fa+ba*(1-fa)
orgb=(fr[:R,ref,:3]*fa+bk[:R,ref,:3]*ba*(1-fa))/np.maximum(oa,1e-6)
prof=np.concatenate([orgb,oa],1)[:,None,:]
wgt=np.clip((R-np.arange(R))/3.0,0,1)[:,None,None]
bk[:R,PX0:PX1]=bk[:R,PX0:PX1]*(1-wgt)+prof*wgt
Image.fromarray(np.clip(bk*255,0,255).astype(np.uint8),'RGBA').save('assets/slot_back_gb_clear.png')
