"""Finish the PAL render for the firmware: pull the outline at the right, bottom and left ends down to the darkest edge pixel (no light rim, both sides alike), then scale to 336x228. Usage: snes_pal_edges.py RENDER_DIR"""
import sys, numpy as np
from PIL import Image
def fix(src, dst, size):
    im=np.asarray(Image.open(src).convert('RGBA')).astype(np.float32)
    h,w=im.shape[:2]; a=im[...,3]
    out=im.copy()
    def run(get, n, length):
        for i in range(n):
            line=get(i)  # view of shape (length,4)
            op=np.where(line[:,3]>0)[0]
            if len(op)==0: continue
            end=op[-1]; lo=max(op[0], end-6)
            seg=line[lo:end+1,0]
            k=lo+int(np.argmin(seg)); m=line[k,:3].copy()
            for j in range(k+1,end+1):
                line[j,:3]=np.minimum(line[j,:3], m)
    run(lambda i: out[i,:,:], h, w)       # right edge
    run(lambda i: out[:,i,:], w, h)       # bottom edge
    run(lambda i: out[i,::-1,:], h, w)    # left edge, so it matches the right
    img=Image.fromarray(out.clip(0,255).astype(np.uint8),'RGBA')
    img.convert('RGBa').resize(size,Image.Resampling.LANCZOS).convert('RGBA').save(dst,optimize=True)
def seams(path):
    """The two seams cut like the SNES slot's grooves (Roy 2026-11-15): 2px dark, the wall in
    shadow then the floor, and 1px lit, shaded from the plastic beside them. One line: below the
    label and on above it, between the label and the top rim."""
    im=np.asarray(Image.open(path).convert('RGBA')).astype(np.float32)
    a=im[...,3]
    for x in (53, 281):
        for y in list(range(4, 10))+list(range(112, im.shape[0])):
            if y>100 and a[y,x-3:x+6].min()<255: break
            beside=(im[y,x-4,:3]+im[y,x+5,:3])/2
            for i,f in enumerate((0.5, 0.68)):
                im[y,x+i,:3]=beside*f
            # lit as the slot's (x1.08 + 6 on screen), as an overlay base on the SFC grey
            im[y,x+2,:3]=beside*1.2
            im[y,x-1,:3]=im[y,x+3,:3]=beside
    Image.fromarray(im.clip(0,255).astype(np.uint8),'RGBA').save(path,optimize=True)
if sys.argv[1]=='--seams':
    seams('assets/snes_pal_neutral.png'); sys.exit()
P=sys.argv[1].rstrip('/')+'/'
for part in ['neutral','clear']:
    fix(P+'snes_%s.png'%part,'assets/snes_pal_%s.png'%part,(336,228))
seams('assets/snes_pal_neutral.png')
