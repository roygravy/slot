/* Configure library by modifying this file */
/* slot.: 16-bit 565 in (the cores' frames, or 8888 brought down to it), 32-bit XRGB out, the
   picture slot. draws. */

#ifndef SNES_NTSC_CONFIG_H
#define SNES_NTSC_CONFIG_H

#define SNES_NTSC_IN_FORMAT SNES_NTSC_RGB16
#define SNES_NTSC_OUT_DEPTH 32

/* Type of input pixel values */
#define SNES_NTSC_IN_T unsigned short

/* Each raw pixel input value is passed through this. You might want to mask
the pixel index if you use the high bits as flags, etc. */
#define SNES_NTSC_ADJ_IN( in ) in

/* For each pixel, this is the basic operation:
output_color = SNES_NTSC_ADJ_IN( SNES_NTSC_IN_T ) */

#endif
