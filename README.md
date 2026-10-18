<div align="center">

# slot. x GRAVY

**A Game Boy–centric frontend for the Anbernic RG35XX Plus, built on [slot.](https://github.com/BrandonKowalski/slot)**

Real cartridges on a shelf. A real cartridge slot. On your shelf in 3–5 seconds. Just pick a game and push it in.

<img src=".github/resources/gravy/carousel.gif" width="640" alt="Scrolling the Game Boy shelf and pushing Mud Warriors into the slot">

[**Download slot-x-gravy-v1.0**](https://github.com/roygravy/slot/releases) · [Install](#install) · [What's new](#whats-new) · [Controls](#controls)

</div>

---

## What is slot. x GRAVY?

slot. x GRAVY is a custom build of [slot.](https://github.com/BrandonKowalski/slot), Brandon Kowalski's bespoke Game Boy, Game Boy Color and Game Boy Advance frontend, tuned for the **Anbernic RG35XX Plus** and its 640×480 screen.

Where slot. is modelled on the GBA SP, slot. x GRAVY leans into the handheld it runs on: the RG35XX Plus is a love letter to the original Game Boy, so Game Boy carts go into a Game Boy slot, the plastic looks like plastic, and every menu is written for people who just want to play.

It plays GB, GBC and GBA games only (unzipped ROMs), on mGBA, gpSP and Gambatte.

### Ridiculously fast

Power on and your shelf is there in **3 to 5 seconds**. Push a cart in and you're playing, right where your last save state left off.

---

## What's new

### Two themes, two slots

A **Dark** and a **Light** housing, each with its own clean wallpaper. Game Boy and Game Boy Color carts slide into a Game Boy cartridge slot; GBA carts get their own slot with the finger scoop. Both are drawn as moulded plastic, and every cart sits flush with the top of its slot.

<img src=".github/resources/gravy/themes.png" width="100%" alt="The Game Boy shelf in the dark and the light theme">

<img src=".github/resources/gravy/slots.png" width="100%" alt="A GBA cart and a Game Boy Color cart seated in their slots, dark and light">

### Favorites that stand out

Press **Y** on any cart to make it a favorite. Favorites stand a little taller on their shelf under a soft, warm spotlight, and get a shelf of their own where the light stays put in the middle as you scroll through it. Prefer something quieter? Pick **Star** instead.

<img src=".github/resources/gravy/favorites.png" width="100%" alt="The Favorites shelf in dark and the GBA shelf in light">

### Carts without label art

No scan for a game? It still gets a proper cartridge: bare plastic with the title written on by hand in marker, dark on Game Boy carts, white on the GBA's dark shell.

<img src=".github/resources/gravy/marker.png" width="100%" alt="Carts with no label art, titled in marker">

### Menus anyone can use

Every settings page lists its options on the left with the value on the right, flat pill switches for anything that's on or off, and a plain-English line at the bottom saying what the selected option does. The light theme gets light menus.

<img src=".github/resources/gravy/menus.png" width="100%" alt="The main menu and the General page">
<img src=".github/resources/gravy/menus2.png" width="100%" alt="The Screen and Gameplay pages">

**Screen Look** sets the whole picture in one go: *Authentic* turns on everything that makes the screen look like the real hardware, *Clean* turns it all off. Or fine-tune it per console:

| Setting | What it does |
|---|---|
| LCD Grid | Thin lines between the pixels, like the real screen |
| Original Colors | Softer colors, the way the original screen showed them |
| Color Palettes | Tint Game Boy games; SELECT + L2 / R2 picks a palette |
| See-Through Pixels | Pale pixels let the screen behind them show through |
| Motion Blur | Moving pixels fade out slowly, like an old LCD |

### Every button, explained — and a proper power menu

A **Buttons** page in the menu lists every shortcut, everywhere. Hold **POWER** for **Sleep**, **Restart** or **Power Off**.

<img src=".github/resources/gravy/buttons_power.png" width="100%" alt="The Buttons page and the power menu">

### Play

<img src=".github/resources/gravy/gameplay.png" width="100%" alt="Goodboy Galaxy on mGBA and Mud Warriors on Gambatte">
<img src=".github/resources/gravy/ingame.png" width="100%" alt="The in-game menu, and the Game Boy Color shelf in light">

### And more

- Carts sit a little smaller on the carousel, with soft drop shadows that lift them off the wallpaper
- Game titles under the shelf at the same size as the shelf names
- A lighter GBA LCD grid
- Games start on your Favorites shelf when you have favorites, otherwise on Game Boy
- Sleep after 1 minute by default, power off after 5 minutes asleep

---

## Install

1. Download **slot-x-gravy-v1.0.zip** from the [releases](https://github.com/roygravy/slot/releases).
2. Unzip it and copy the **contents** of the `slot-x-gravy-v1.0` folder onto the second SD card of your RG35XX Plus.
3. Put your games in `Games/GB`, `Games/GBC` and `Games/GBA`, and optional label art (PNG, named like the ROM) in the matching `Labels` folders.
4. Put the BIOS files (`gb_bios.bin`, `gbc_bios.bin`, `gba_bios.bin`) in `BIOS/` if you have them.

ROMs must be unzipped; `.zip` and `.7z` are not read.

---

## Controls

The full list is on the **Buttons** page in the menu. The essentials:

| On the shelf | |
|---|---|
| Left / Right | Move along the shelf |
| L1 / R1 | Switch shelf |
| Tap A / Hold A | Play from your last save / start fresh |
| Y | Favorite |
| MENU | Settings |

| In a game | |
|---|---|
| Hold MENU | Save, eject and back to the shelf |
| Double tap MENU | Save state switcher |
| Hold L2 / Hold R2 | Rewind / fast-forward |
| SELECT + Up / Down | Brightness |
| SELECT + MENU | Link cable to another slot. device |

---

## Building

slot. x GRAVY builds the same way slot. does; see [the slot. README](SLOT_README.md) and `taskfile.yml`. The cartridge slots are drawn by `tools/slot_art.py`. Screenshots in this page were rendered headless with `cargo run --release -p slot --no-default-features --example shoot`.

---

## Credits

- **[slot.](https://github.com/BrandonKowalski/slot)** by Brandon Kowalski, the frontend slot. x GRAVY is built on, with artwork by Markus and contributions from its community
- **mGBA**, **gpSP** and **Gambatte**, the emulator cores (their licenses and sources ship in `System/licenses`)
- **Permanent Marker** by Font Diner (Apache 2.0) for the handwritten titles
- **Liberation Sans** by Red Hat (SIL OFL 1.1), subset as "slot Serial", for the serial sticker on the About page
- **Goodboy Galaxy** (demo) and **Mud Warriors**, the free games in the screenshots

Like slot., slot. x GRAVY was put together with help from Claude.

Licensed under the same terms as slot.; see [LICENSE](LICENSE).
