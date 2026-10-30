/* slot.'s handle on Blargg's snes_ntsc, as Snes9x sets it up for its "snes9x_blargg" option, so a
   picture slot. filters itself (the Super Game Boy's, from mGBA) looks as Snes9x's does. */
#include <stdlib.h>
#include "snes_ntsc.h"

/* cable: 1 rf, 2 composite, 3 s-video, 4 rgb */
snes_ntsc_t *slot_ntsc_new(int cable)
{
	snes_ntsc_t *ntsc = (snes_ntsc_t *) malloc(sizeof(snes_ntsc_t));
	snes_ntsc_setup_t setup;
	if (!ntsc)
		return NULL;
	switch (cable) {
	case 1: setup = snes_ntsc_composite; setup.merge_fields = 0; break;
	case 3: setup = snes_ntsc_svideo; break;
	case 4: setup = snes_ntsc_rgb; break;
	default: setup = snes_ntsc_composite; break;
	}
	snes_ntsc_init(ntsc, &setup);
	return ntsc;
}

void slot_ntsc_free(snes_ntsc_t *ntsc)
{
	free(ntsc);
}

void slot_ntsc_blit(snes_ntsc_t const *ntsc, unsigned short const *in, long in_row_width,
		int burst_phase, int in_width, int in_height, void *out, long out_pitch)
{
	snes_ntsc_blit(ntsc, in, in_row_width, burst_phase, in_width, in_height, out, out_pitch);
}
