# Changelog

## slot. x GRAVY v1.1

Everything since slot. x GRAVY v1.0.2.

### New

- **Deep sleep**: a tap on POWER now really puts the console to sleep (suspend to RAM). The green light blinks out as it falls asleep, POWER wakes it straight back into your game. Overnight it uses about half a percent of battery an hour.
- **NES and SNES**: their own shelves and slots, on FCEUmm and Snes9x, with NTSC and PAL cart moulds. The picture goes through a CRT shader and Blargg's NTSC composite filter. NES colours: Authentic or Trinitron.
- **Super Game Boy**: put the SGB cart in the SNES slot and the camera zooms onto the Game Boy shelf inside it. Games play in SGB colours with their own border, one of the eight real borders read from `SGB1.sfc` / `SGB2.sfc` in `BIOS/`, or the slot. x GRAVY border.
- **Light Sleep** before the deep sleep: Wi-Fi stays on, any button wakes it; after Direct / 5 / 10 / 30 minutes (5 by default) it goes into deep sleep, which only POWER wakes.
- **Slot Color**: Dark, Light or Originals, the colours of the real consoles. The NES Originals slot has the two black blocks beside the lid.
- **Highlight** behind the cart, next to Spotlight, Glow and Star, for Favorites.
- **Label search**: Y in the cartridge studio searches for a label and lets you pick another, Europe first, then USA, then Japan. The on-screen keyboard has accents.
- Your own wallpaper gets black menus, whatever the Slot Color.
- **3:2 screens** (RG34XX SP, 720x480): the shelf and menus span the whole panel, and full screen GB/GBC, GBA, save states and screenshots fill it without black bars. The GBA LCD filter keeps to the panel's own pixels.
- **Home Wi-Fi** (ported from pvaibhav/slot): join your home network from Settings -> General -> Home Wi-Fi. Networks are listed in `Config/wifi.toml`.
- **Network** has its own row in the menu, with Home Wi-Fi on/off and the network you are connected to.
- **Link over the home network**: with Home Wi-Fi connected, two consoles running the same game find each other on the network. Without it the direct link works as before. The link screen shows a HOME WI-FI or DIRECT LINK plate.
- **Clock**: Auto Time sets the clock over Home Wi-Fi (on by default). The time zone is picked as a city, with that city's own summer time. 24-Hour Time off shows 6:00 PM in the status bar, the menu and the clock screen. The clock screen uses the menu's background and ink.
- **Phone-style status bar**: clock on the left, Wi-Fi and battery on the right, set larger and drawn the way a phone draws them.
- **Labels fetched in the background**: missing cart labels are looked up in `Labels/Artwork.pack` or on LaunchBox, only with Home Wi-Fi on or an artwork pack on the card. A label already on the card is never replaced.
- **Labels cached in the background**: the Caching New Labels screen at boot is gone. The shelf is usable right away while a CACHING LABELS bar runs along the top.
- **Cartridge studio**: SELECT on a cart picks its shell colour and finish (solid, clear, glitter). A COLOR and a MARKER tab, and Y fetches the label.
- **Marker ink** (black, white, gold) for blank carts that have no label online.
- Labels have rounded corners and sit in a recess in the cart's own plastic.
- The studio opens and closes like the core picker: the neighbours part, the cart hops up and the rows slide in under it.
- **Cheats fetched automatically**: with Home Wi-Fi on, each game gets its cheat file from the libretro database, so the card doesn't need the whole database.
- **SELECT + B** turns only the screen effects you chose for that system off and back on. With nothing chosen it brings the standard look: Dot Matrix with the DMG palette for GB, the LCD filter for GBC and GBA.
- Carts on the GB shelf always run as a Game Boy in the chosen palette, even when their header says GBC only.
- Screenshots sits above Buttons in the menu, and the Buttons page lists L1 / R1 for full screen on GB and GBC.
- About: the dark back is easier to read, the letters catch light on their edges and keep the housing colour. The credits add pvaibhav's Home Wi-Fi, link and label work, LaunchBox labels, libretro cheats and shaders, and Liberation Sans.

### Fixed

- Favorites and recents are kept when a slot. card first boots into GRAVY.
- Auto Sleep waits while a button is held or fast forward or rewind runs.
- Resetting with START then SELECT no longer lets the game see either button.
- A card without GB games starts on the first shelf that has carts, instead of a bare GB shelf.
- Resetting during the GB / GBC boot logo no longer leaves a game that never starts.
- No dark picture after a Ghosting change, and the screen colour follows a palette switch at once.
- The shelf stays drawn while a held letter jump outruns the scroll.
- Smaller fixes in the labels, the cartridge studio and Wi-Fi: the Wi-Fi status was polled with Home Wi-Fi off, `cart_shell.txt` could get duplicate names, and the caching bar drew over the pickers.

### From upstream slot v1.9.0 (robustness)

- A core that does not answer (stuck in a frame) no longer stalls a save, eject, or shutdown: timeouts are 2 seconds for state/save/thumbnail, 10 seconds for stopping the core.
- A label whose decoder panics no longer stalls the label cache rebuild; the bad label is skipped silently.
- The SD card is synced after each save and after each log write (every second), so long outages don't lose recent logs or game saves.
- **Crash recovery**: if a cart crashes while slot is loading it, the console starts on the shelf without trying to play that cart again, and the crashed state is retired.

### From upstream slot v1.8.0 and v1.8.1

- MBC2 games no longer crash on their first save RAM read. (#137)
- mGBA asks for the state size once per game, and again once the game settles on its save chip. (#126)
- Loading a state while a game writes its 128 KB flash no longer puts the save in the wrong half of the chip. (#104)
- **Run-Ahead is removed**: it scrambled saves in games with 128 KB flash, such as Super Mario Advance 4. Your other settings are kept. (#104)
- The lowest brightness step is as bright as the old second step. (#100)
- Wi-Fi is unloaded before Power Off, as Restart already did. (#117)
- **Safer SD card** through crashes, power offs and failed reads: no full memory dump on a crash (the last 5 crash logs are kept), the card is synced before power off, the resume state is not rewritten when nothing changed, half-written files from a power cut are swept at boot, the boot partition stays read-only unless the boot logo changes, and a save the card failed to read is no longer written over.
- Not taken: the BaseOS startup sound. slot claims the speaker at boot, so it stays silent.


## [1.7.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.7.0)

- The slot, carts and shelf now have a skeuomorphic effect, with light and dark themes and bundled wallpapers. Huge thanks to Markus for this amazing work! (#74, @MarkusMaribu)
- An in-game menu: tap MENU to pause the game behind it. A double tap still goes straight to States.
- Favorites: Y stars a cart onto its own shelf. (#89, @diluno)
- A Recents shelf of the last 10 games, switched on from Settings -> General.
- slot remembers the shelf and cart at power off and boots back onto them. (#53)
- Per-platform bezels. Markus again! (#77, @MarkusMaribu)
- On 4:3 panels the shelf fills the screen. (#81, @diluno)
- Game Titles on the General page fades the shelf name into the selected game's name. (#84)
- A 1 frame run-ahead setting for GBA games on mGBA. (#56, @454135431515)
- Wallpaper Tint darkens your own wallpaper on the shelf.
- GBA, GB and GBC each get their own Shader and Color Correction rows. (#83, @paulocoe-creator)
- LCD3x is offered for GB and GBC games. (#88, @paulocoe-creator)
- Game Boy stretch now properly only widens the picture.
- The backlight steps follow a curve, so the low-end goes dimmer. (#54, @flo333)
- Holding a direction repeats it in menus, the clock and the shelf letter jump, but never in play.
- RG34XX uses its own chords for brightness and the menu display. (#85, @paulocoe-creator)
- The RG28XX picture is turned the way BaseOS says.
- Restart no longer hangs on a black screen: the WiFi driver is unloaded first.
- Muted play keeps its frame rate. (#51, @flo333)
- Fast-forward no longer stutters on frames that take longer than average. (#70)
- gpSP no longer crashes when a crowded boot left its JIT cache unmapped.
- Core options follow the platform that is playing. (#82)
- Card logs moved into a `Logs` folder, and the session log rotates. Card alarms are logged too.

## [1.6.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.6.0)

- Runs on the RG28XX: its panel is mounted portrait, so slot turns the picture onto it. (#35, @GamerAnim)
- Pokémon Pinball: Ruby & Sapphire rumble. mGBA now sees the game's Game Boy Player check. You still need to turn rumble on in game settings. (#25)
- A red alarm shows when the SD card turns read-only or a save fails to write.
- A new shortcut for mute: SELECT + VOL+ / VOL-. The 34XX has a fake volume wheel so VOL+ + VOL- isn't possible. (#39)
- Game Boy cartridge clocks are kept in the .sav. (#22)
- The GBA cartridge clock keeps running on gpSP while the game is off.
- Large libraries now should boot faster, and the first frame shows sooner.
- Games start faster: resting on a cart reads its ROM ahead, so big ROMs no longer wait on the SD card when you insert them.
- Wallpapers are dealt from a shuffled deck, so none repeats until all have shown.
- PNG art is turned the way its EXIF orientation tag says.
- When slot crashes it logs where, and keeps that run's log in `crash/` to attach to a bug report.
- The BaseOS startup sound no longer leaves slot silent. (#20)
- A user defined `Config/bootlogo.bmp` will take precedence over the shipped one. (#30)
- Cart shell, core and video mode files are now `.txt` instead of `.ini`, so Chrome on Windows opens them. slot renames them on first boot. (#15)
- The grid shader's lines stay even on the RG34XX and at fractional scales, and the dot shader no longer darkens a white screen as much. (#26, #36)

## [1.5.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.5.0)

- Rumble regression fixed.
- The boot logo is built to the size of the one BaseOS installed, so it fits every panel instead of only 720x480.
- Runs on the RG35XXSP: the picture fills the 4:3 screen's width and a GBA SP bezel sits underneath while a game plays.
- Screen shaders chosen per platform: Off, LCD3x, Grid or Dot for GBA, and Off, Grid or Simpletex for Game Boy and Game Boy Color.
- SELECT + B / A in game steps back and forth through the screen shaders. A game's own soft reset that holds SELECT with A or B no longer reaches it; hold A on the carousel to start a game fresh instead.
- New settings: auto save on eject, turbo buttons and rewind can each be switched off.
- Settings are grouped into Screen and Gameplay pages, and every value wraps from the last choice to the first.
- Label art is cached on the card, so the shelf no longer stutters building cart faces. New labels are cached once at boot behind a Caching New Labels screen.
- Named palettes for Game Boy games: turn on GB Palettes in the Screen settings, then SELECT + L2 / R2 in game steps back and forth through mGBA's 48 palettes.
- The speaker no longer buzzes while nothing is playing. slot lets go of the audio device after 3 seconds of silence and takes it back when sound starts. Thanks to flo333 for finding it.

## [1.4.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.4.0)

- The volume bar shows a headphones icon while they are plugged in.
- Speaker and headphones keep their own volume and mute, and switch the moment you plug / unplug.
- Settings moved from `System/` to a new `Config/` folder, so replacing `System/` on update no longer resets them. slot moves them on first boot.
- The quick menu and power menu wrap from the last item to the first, and back.

## [1.3.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.3.0)

- The SP boots to a slot logo instead of BaseOS's. slot installs `System/bootlogo.bmp` on first run and keeps the original as `bootlogo.baseos.bmp`.

## [1.2.2](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.2)

- Fix for hitch in games caused by autosave. The autosave now writes to the card in the background.

## [1.2.1](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.1)

- Shell colors from Cart Studio now apply to games with accented names.
- Clear GBA carts no longer show their circuit board above the label.

## [1.2.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.0)

- Hold X for turbo A and Y for turbo B.
- Carts on the shelf are larger, and shrink into the slot as they go in.
- Carts are drawn at their real proportions, so full label scans fit without cropping.
- Jumping a letter with Up or Down shows the letter in the slot.
- Saves now stick for games whose save is smaller than the core first reports, such as Golden Sun on mGBA.
- The seated cart no longer shows through the slot as a game starts.
- SELECT reaches the game as soon as it can no longer be the start of a shortcut.

## [1.1.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.1.0)

- Fast-forward runs several game frames per refresh, so it is properly fast, and rewind no longer records while fast forwarding.
- The About label credits BaseOS.

## [1.0.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.0.0)

- Game Boy and Game Boy Color, each on its own shelf. L1 and R1 switch shelves, and carts are drawn at their own size in their own plastics.
- Two cores, mGBA and gpSP. Press START on a cart to open it and swap the chip.
- Link play between two RG SPs over WiFi: the link cable and the Wireless Adapter for Pokémon trades and Advance Wars, mGBA's link mode for Mario Kart, and Game Boy link for Tetris.
- A quick menu on the shelf for fast forward speed and sound, rumble, color correction, the date and time, and About.
- Game Boy games stretch to fill the screen with L1 and go back with R1.
- The real BIOS boot animation plays when the card has one.
- Per-cart shell colors, set in `cart_shell.ini`.
- Up and Down jump through the shelf a letter at a time.
- Your own folder layout on the card is kept, and slot runs on upstream BaseOS.

## [0.1.1](https://github.com/BrandonKowalski/slot/releases/tag/v0.1.1)

- The game holds still behind the power menu.
- Rewind records at every speed and replays exactly what happened.

## [0.1.0](https://github.com/BrandonKowalski/slot/releases/tag/v0.1.0)

- First release: a GBA-only shelf of cartridges with an insert scrape and clunk.
- The game in the slot resumes on the next boot. Closing the lid sleeps and saves.
- Save states with a switcher, rewind, fast-forward, brightness, blue light and volume.
- A power menu on a held POWER, and an About label.
