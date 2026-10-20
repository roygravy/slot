<div align="center">

# slot. x GRAVY

**My own take on [slot.](https://github.com/BrandonKowalski/slot), Brandon Kowalski's Game Boy–centric frontend, for the Anbernic RG35XX Plus**

slot.'s real cartridges on a shelf and its real cartridge slot, with a little extra gravy on top.

<img src=".github/resources/gravy/carousel.gif" width="640" alt="Scrolling the GBA shelf and pushing Goodboy Galaxy into the slot">

[**Download slot-x-gravy-v1.0.2**](https://github.com/roygravy/slot/releases) · [Install](#install) · [What GRAVY adds](#what-gravy-adds) · [Controls](#controls)

</div>

---

## What is slot. x GRAVY?

My own take on [slot.](https://github.com/BrandonKowalski/slot), Brandon Kowalski's bespoke Game Boy, Game Boy Color and Game Boy Advance frontend. Everything that makes it special is slot.'s: the shelf of cartridges, pushing a cart into the slot to play, save states, rewind, the link cable, Markus's artwork, and the speed (power on and your shelf is there in **3 to 5 seconds**).

New to slot.? Start at **[slot-cfw.fyi](https://slot-cfw.fyi)**.

GRAVY adds a little extra on top, tuned for the RG35XX Plus. It plays GB, GBC and GBA games (unzipped ROMs) on mGBA, gpSP and Gambatte.

---

## What GRAVY adds

### Screens that look like the real thing

Each console gets its own screen: the Game Boy's green dot matrix, the Game Boy Color's warm LCD and the Game Boy Advance's crisp one. Pale pixels let the light through, every pixel casts a soft shadow, and moving pixels leave a faint trail.

<img src=".github/resources/gravy/screen.png" width="100%" alt="Close-ups of the Game Boy, Game Boy Color and Game Boy Advance screens">

Want it clean? **SELECT + B** turns every effect off, and on again.

<img src=".github/resources/gravy/effects.gif" width="480" alt="SELECT + B switching the screen effects off and on in Goodboy Galaxy">

<img src=".github/resources/gravy/gameplay.png" width="100%" alt="Goodboy Galaxy on the GBA screen and Hauntsfield in Game Boy green">

### Every console, its own settings

Dot Matrix, palettes, Pixel Transparency, Ghosting and a Black Border, set per console, from the menu or right in the game.

<img src=".github/resources/gravy/menus2.png" width="100%" alt="The Game Boy screen settings and the in-game Screen page">

### Screenshots

**VOL+ and POWER** takes a screenshot, effects and all, in games and in the menus. Flick through them on the Screenshots page.

<img src=".github/resources/gravy/shots.png" width="100%" alt="The Screenshots page, and the power-off countdown">

### Power and reset

Tap **POWER** to sleep. Hold it and a bar counts down to power off; let go to cancel. **START + SELECT** resets the game. With **Resume Game** on, the handheld powers straight back into your game.

### Two slots, new wallpapers

Game Boy games get a Game Boy slot, GBA games a GBA SP slot, in moulded plastic, with new wallpapers for the Dark and Light themes.

<img src=".github/resources/gravy/themes.png" width="100%" alt="The Game Boy Color shelf in the dark and the light theme">

<img src=".github/resources/gravy/slots.png" width="100%" alt="The empty GBA slot and a Game Boy cart seated in its slot, dark and light">

### Favorites in the spotlight

Star a cart with **Y** and it stands a little taller under a warm spotlight.

<img src=".github/resources/gravy/favorites.png" width="100%" alt="The Favorites shelf in dark and light">

### Carts without label art

No scan? The cart gets its title written on by hand.

<img src=".github/resources/gravy/marker.png" width="100%" alt="Carts with no label art, titled in marker">

### Clean menus

Options on the left, values on the right, simple switches and a plain-English line for what each one does. Every button is explained on the **Buttons** pages.

<img src=".github/resources/gravy/menus.png" width="100%" alt="The main menu and the General page">
<img src=".github/resources/gravy/buttons.png" width="100%" alt="The Buttons pages">

### Cheats

Switch a game's cheats on and off while you play. The whole RetroArch cheat collection for GB, GBC and GBA comes along, and your own `.cht` files work too.

<img src=".github/resources/gravy/cheats.png" width="100%" alt="The in-game menu and the Cheats page">

---

## Install

1. Download **slot-x-gravy-v1.0.2.zip** from the [releases](https://github.com/roygravy/slot/releases).
2. Unzip it and copy the **contents** of the `slot-x-gravy-v1.0.2` folder onto the second SD card of your RG35XX Plus.
3. Put your games in `Games/GB`, `Games/GBC` and `Games/GBA`, and optional label art (PNG, named like the ROM) in the matching `Labels` folders.
4. Put the BIOS files (`gb_bios.bin`, `gbc_bios.bin`, `gba_bios.bin`) in `BIOS/` if you have them.

ROMs must be unzipped; `.zip` and `.7z` are not read.

---

## Controls

The full list is on the **Buttons** pages in the menu. The essentials:

| On the shelf | |
|---|---|
| Left / Right | Move along the shelf |
| L1 / R1 | Switch shelf |
| Tap A / Hold A | Play from your last save / start fresh |
| Y | Favorite |
| MENU | Settings |

| In a game | |
|---|---|
| Tap MENU | In-game menu |
| Hold MENU | Save and eject |
| SELECT + R1 / L1 | Save / load a state |
| Hold L2 / Hold R2 | Rewind / fast-forward |
| SELECT + B | Screen effects on / off |
| START + SELECT | Reset the game |

| Everywhere | |
|---|---|
| VOL+ and POWER | Screenshot |
| POWER / Hold POWER | Sleep / power off |
| SELECT + Up / Down | Brightness |

---

## Building

slot. x GRAVY builds the same way slot. does; see [the slot. README](SLOT_README.md) and `taskfile.yml`. The cartridge slots are drawn by `tools/slot_art.py`. Screenshots in this page were rendered headless with `cargo run --release -p slot --no-default-features --example shoot`.

---

## Credits

- **[slot.](https://github.com/BrandonKowalski/slot)** by Brandon Kowalski, with artwork by Markus and contributions from its community: everything slot. x GRAVY is built on, and most of what it does
- **mGBA**, **gpSP** and **Gambatte**, the emulator cores (their licenses and sources ship in `System/licenses`)
- **Permanent Marker** by Font Diner (Apache 2.0) for the handwritten titles
- **Liberation Sans** by Red Hat (SIL OFL 1.1), subset as "slot Serial", for the serial sticker on the About page
- The cheats from the **[libretro-database](https://github.com/libretro/libretro-database)** by the libretro team and contributors (CC BY-SA 4.0)
- **Goodboy Galaxy** (demo), **Hauntsfield** and **Mud Warriors**, the free games in the screenshots

Like slot., slot. x GRAVY was put together with help from Claude.

Licensed under the same terms as slot.; see [LICENSE](LICENSE).

---

## AI Disclosure

None of this is meant to be taken seriously. Every line of code I added, every pixel of the art I added and every word on this page was generated by Claude; I mostly pointed at things and said "a bit rounder" a lot. All the original art that isn't mine belongs to its respective makers.

I made slot. x GRAVY purely for myself, so that my device feels like *my* device. If you enjoy it too, that's a lovely bonus.

All the praise and credit goes to [Brandon](https://github.com/BrandonKowalski/slot) and the very special base he built. slot. is the whole meal; I'm just pouring a little of my own gravy over it.
