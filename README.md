<div align="center">

# slot. x GRAVY

**My own take on [slot.](https://github.com/BrandonKowalski/slot), Brandon Kowalski's Game Boy–centric frontend, for the Anbernic RG35XX Plus**

slot.'s real cartridges on a shelf and its real cartridge slot, with a little extra gravy on top.

<img src=".github/resources/gravy/carousel.gif" width="640" alt="Scrolling the GBA shelf and pushing Goodboy Galaxy into the slot">

[**Download slot-x-gravy-v1.0.1**](https://github.com/roygravy/slot/releases) · [Install](#install) · [What GRAVY adds](#what-gravy-adds) · [Controls](#controls)

</div>

---

## What is slot. x GRAVY?

slot. x GRAVY is a personal modification of [slot.](https://github.com/BrandonKowalski/slot), Brandon Kowalski's bespoke Game Boy, Game Boy Color and Game Boy Advance frontend. Everything that makes it special is slot.'s: the shelf of cartridges, pushing a cart into the slot to play, save states, rewind, the link cable, Markus's artwork, and how ridiculously fast it is (power on and your shelf is there in **3 to 5 seconds**).

New to slot.? Everything you need to know first (what it is, how it works, the full guide) is on **[slot-cfw.fyi](https://slot-cfw.fyi)**.

On top of that, slot. x GRAVY adds a few things of my own, tuned for the RG35XX Plus: since that handheld is a love letter to the original Game Boy, Game Boy carts get a Game Boy slot, there's a little more screen authenticity, and a handful of menu changes. Everything else works exactly like slot.

It plays GB, GBC and GBA games only (unzipped ROMs), on mGBA, gpSP and Gambatte.

### A screen that looks like the real thing

slot. already offers an LCD grid, the softer colors of the original screens and mGBA's 48 Game Boy palettes. slot. x GRAVY turns them all on out of the box, with Game Boy games in the green of the original DMG, and adds two effects of its own: pale pixels let the screen behind them show through, and moving pixels leave a faint trail, like an old LCD.

<img src=".github/resources/gravy/screen.png" width="100%" alt="Close-ups of Goodboy Galaxy and Hauntsfield with the default screen settings">

Prefer it sharp and bright? GRAVY's **Screen Look** setting switches everything at once between *Authentic* (the default) and *Clean*. Or pick and choose:

| Setting | Consoles | What it does | From |
|---|---|---|---|
| LCD Grid | GB, GBC, GBA | Thin lines between the pixels, like the real screen | slot. |
| Original Colors | GB, GBC, GBA | Softer colors, the way the original screen showed them | slot. |
| Color Palettes | GB | Tint Game Boy games with one of mGBA's 48 palettes; SELECT + L2 / R2 flips through them in a game | slot. |
| See-Through Pixels | All | Pale pixels let the screen behind them show through | GRAVY |
| Motion Blur | All | Moving pixels fade out slowly, like an old LCD | GRAVY |

---

## What GRAVY adds

### Two slots, new wallpapers

slot.'s Dark and Light themes get redrawn cartridge slots in moulded plastic: Game Boy games get a Game Boy slot, GBA games a GBA SP slot. Each theme gets a new plain wallpaper, and either wallpaper goes with either theme, next to your own from the `Wallpapers` folder.

<img src=".github/resources/gravy/themes.png" width="100%" alt="The Game Boy Color shelf in the dark and the light theme">

<img src=".github/resources/gravy/slots.png" width="100%" alt="The empty GBA slot and a Game Boy cart seated in its slot, dark and light">

### Favorites that stand out

slot. already stars a cart onto its Favorites shelf with **Y**. In GRAVY, favorites stand a little taller under a soft, warm spotlight, and on the Favorites shelf the light stays put in the middle as you scroll. Prefer slot.'s quieter star? Pick **Star** instead.

<img src=".github/resources/gravy/favorites.png" width="100%" alt="The Favorites shelf in dark and light">

### Carts without label art

No scan for a game? It still gets a proper cartridge: bare plastic with the title written on by hand in marker, dark on Game Boy carts, white on the GBA's dark shell.

<img src=".github/resources/gravy/marker.png" width="100%" alt="Carts with no label art, titled in marker">

### A different look for the menus

slot.'s settings pages get a new layout: options on the left with the value on the right, flat pill switches for anything that's on or off, and a plain-English line at the bottom saying what the selected option does. The light theme gets light menus.

<img src=".github/resources/gravy/menus.png" width="100%" alt="The main menu and the General page">
<img src=".github/resources/gravy/menus2.png" width="100%" alt="The Screen and Gameplay pages">

### Every button, explained, and Sleep in the power menu

A **Buttons** page in the menu lists every shortcut slot. answers to. Holding **POWER** now offers **Sleep** next to slot.'s Restart and Power Off.

<img src=".github/resources/gravy/buttons_power.png" width="100%" alt="The Buttons page and the power menu">

### Playing

<img src=".github/resources/gravy/gameplay.png" width="100%" alt="Goodboy Galaxy on mGBA and Hauntsfield in the original Game Boy green">
<img src=".github/resources/gravy/ingame.png" width="100%" alt="Hauntsfield in its own colours and on the DMG Green palette">

### Cheats

The in-game menu gets a **Cheats** page: switch a game's cheats on and off while you play, and they stay on next time. The whole RetroArch cheat collection for GB, GBC and GBA comes along in the `Cheats` folder, and you can add your own `.cht` files there. While any cheat is on, a little masked robber sits by the battery; the **Cheat Icon** switch hides it.

### And more

- Carts sit a little smaller on the carousel, with soft drop shadows that lift them off the wallpaper
- Game titles under the shelf at the same size as the shelf names
- A lighter GBA LCD grid
- Gambatte as the Game Boy core
- An About page in the style of a Game Boy's back
- Sleep after 1 minute by default, power off after 5 minutes asleep
- Black-and-white menus: main menus centred, sub-pages six rows at a time

---

## Install

1. Download **slot-x-gravy-v1.0.1.zip** from the [releases](https://github.com/roygravy/slot/releases).
2. Unzip it and copy the **contents** of the `slot-x-gravy-v1.0.1` folder onto the second SD card of your RG35XX Plus.
3. Put your games in `Games/GB`, `Games/GBC` and `Games/GBA`, and optional label art (PNG, named like the ROM) in the matching `Labels` folders.
4. Put the BIOS files (`gb_bios.bin`, `gbc_bios.bin`, `gba_bios.bin`) in `BIOS/` if you have them.

ROMs must be unzipped; `.zip` and `.7z` are not read.

---

## Controls

These are slot.'s own controls, unchanged; the full list is on the **Buttons** page in the menu. The essentials:

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
