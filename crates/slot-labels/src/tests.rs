use super::*;
use image::{ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

fn cart(stem: &str) -> Cart {
    Cart {
        platform: slot_store::Platform::Gba,
        stem: stem.into(),
        rom: format!("Games/GBA/{stem}.gba").into(),
        label: None,
        title: "".into(),
        code: "".into(),
        shell: None,
        marker: None,
    }
}

fn image_bytes(w: u32, h: u32) -> Vec<u8> {
    // The top and bottom have different colours: catches upside-down/top-centering errors.
    let im = RgbImage::from_fn(w, h, |_, y| {
        if y < h / 2 {
            Rgb([255, 0, 0])
        } else {
            Rgb([0, 0, 255])
        }
    });
    let mut bytes = Cursor::new(Vec::new());
    im.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn no_intro_names_match_database_punctuation() {
    for (stem, db) in [
        (
            "Legend of Zelda, The - A Link to the Past & Four Swords (USA)",
            "The Legend of Zelda: A Link to the Past and Four Swords",
        ),
        (
            "WarioWare, Inc. - Mega Microgame$! (USA)",
            "WarioWare, Inc.: Mega Microgame$!",
        ),
        ("Aladdin (USA) (En,Fr,De,Es)", "Disney's Aladdin"),
        (
            "Pokémon - Emerald Version (USA) [!]\u{200B}",
            "Pokemon: Emerald Version",
        ),
    ] {
        assert_eq!(source::title(stem), source::normalize(db));
    }
}

#[test]
fn title_resolution_requires_unique_exact_gba_match() {
    let item = |id, name, platform| {
        format!("<a href='/games/details/{id}-game'><h3>{name}</h3><p>{platform}</p></a>")
    };
    let correct = item(1, "Mario Kart: Super Circuit", "Nintendo Game Boy Advance");
    let page = format!(
        "{}{}{}",
        correct,
        item(2, "Mario Kart: Super Circuit", "Nintendo DS"),
        item(
            3,
            "Mario Kart: Super Circuit 2",
            "Nintendo Game Boy Advance"
        )
    );
    assert_eq!(
        source::game_id(&page, "mario kart super circuit", Platform::Gba).unwrap(),
        1
    );
    assert!(source::game_id(
        &(correct.clone() + &item(4, "Mario Kart: Super Circuit", "Nintendo Game Boy Advance")),
        "mario kart super circuit",
        Platform::Gba
    )
    .is_err());
    assert_eq!(
        source::game_id(
            &(correct.clone() + &correct),
            "mario kart super circuit",
            Platform::Gba
        )
        .unwrap(),
        1
    );
    assert!(source::game_id(
        "<html>changed layout</html>",
        "mario kart super circuit",
        Platform::Gba
    )
    .is_err());
}

#[test]
fn cart_fronts_rank_own_region_renders_then_photos_then_other_regions_japan_last() {
    let page = r#"
        <a href="https://images.launchbox-app.com/box.png" data-title="Test - Box - Front Image (North America)"></a>
        <a href="https://images.launchbox-app.com/eu.png" data-title="Test - Cart - Front Image (Europe)"></a>
        <a href="https://images.launchbox-app.com/us-fan.png" data-title="Test - Fanart - Cart - Front Image (North America)"></a>
        <a href="https://evil.invalid/us.png" data-title="Test - Cart - Front Image (North America)"></a>
        <a href="https://images.launchbox-app.com/us.png" data-title="Test - Cart - Front Image (North America)"></a>
        <a href="https://images.launchbox-app.com/any.png" data-title="Test - Cart - Front Image"></a>
    "#;
    let urls = |stem| {
        source::image_urls(page, &cart(stem))
            .unwrap()
            .into_iter()
            .map(|u| u.rsplit('/').next().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(urls("Test (USA)"), ["us-fan.png", "us.png", "eu.png"]);
    // Another region's: Europe's, then America's, then any other.
    assert_eq!(urls("Test (Europe)"), ["eu.png", "us-fan.png", "us.png"]);
    assert_eq!(urls("Test (Japan)"), ["eu.png", "us-fan.png", "us.png"]);
    // Japan's come after every other region's, unregioned ones too.
    let japan = r#"
        <a href="https://images.launchbox-app.com/jp-fan.png" data-title="Test - Fanart - Cart - Front Image (Japan)"></a>
        <a href="https://images.launchbox-app.com/any.png" data-title="Test - Cart - Front Image"></a>
    "#;
    assert!(source::image_urls(japan, &cart("Test (Europe)")).unwrap()[0].ends_with("/any.png"));
    // LaunchBox has only a fanart render for most Game Boy and Game Boy Color carts.
    let render = r#"<a href="https://images.launchbox-app.com/fan.png" data-title="Boxxle - Fanart - Cart - Front Image (North America)"></a>"#;
    assert!(
        source::image_urls(render, &cart("Boxxle (USA, Europe) (Rev 1)")).unwrap()[0]
            .ends_with("/fan.png")
    );
    assert!(source::image_urls(
        r#"<a href="https://images.launchbox-app.com/box.png" data-title="Test - Box - Front Image (North America)"></a>"#,
        &cart("Test (USA)")
    )
    .is_err());
    for url in [
        "http://images.launchbox-app.com/x",
        "https://images.launchbox-app.com.evil/x",
        "file:///tmp/x",
    ] {
        assert!(!source::allowed_url(url));
    }
}

#[test]
fn a_game_boy_cart_matches_its_own_platform_first_and_the_other_second() {
    let item = |id, name, platform| {
        format!("<a href='/games/details/{id}-game'><h3>{name}</h3><p>{platform}</p></a>")
    };
    let page = item(1, "Tetris", "Nintendo Game Boy")
        + &item(2, "Tetris", "Nintendo Game Boy Color")
        + &item(3, "Tetris", "Nintendo Game Boy Advance")
        + &item(4, "Wario Land 3", "Nintendo Game Boy Color");
    assert_eq!(source::game_id(&page, "tetris", Platform::Gb).unwrap(), 1);
    assert_eq!(source::game_id(&page, "tetris", Platform::Gbc).unwrap(), 2);
    assert_eq!(source::game_id(&page, "tetris", Platform::Gba).unwrap(), 3);
    assert_eq!(
        source::game_id(&page, "wario land 3", Platform::Gb).unwrap(),
        4
    );
    assert!(source::game_id(&page, "wario land 3", Platform::Gba).is_err());
}

#[test]
fn crop_produces_exact_rgb_png_and_rejects_bad_input() {
    for (platform, sizes) in [
        (
            Platform::Gba,
            &[(1000, 574), (600, 355), (473, 283), (800, 465)][..],
        ),
        (Platform::Gb, &[(796, 906), (674, 759)][..]),
        (Platform::Gbc, &[(800, 916)][..]),
    ] {
        let (lw, lh) = label_size(platform);
        for (w, h) in sizes {
            let png = artwork::prepare(&image_bytes(*w, *h), platform).unwrap();
            assert_eq!(image::guess_format(&png).unwrap(), ImageFormat::Png);
            let im = image::load_from_memory(&png).unwrap().to_rgb8();
            assert_eq!(im.dimensions(), (lw, lh));
            assert_eq!(im.get_pixel(lw / 2, 0), &Rgb([255, 0, 0]));
            assert_eq!(im.get_pixel(lw / 2, lh - 1), &Rgb([0, 0, 255]));
        }
    }
    assert!(artwork::prepare(b"<html>Error</html>", Platform::Gba).is_err());
    assert!(artwork::prepare(&image_bytes(100, 100), Platform::Gba).is_err());
    assert!(artwork::prepare(&image_bytes(5000, 2), Platform::Gba).is_err());
    // A cart scan of the other shape is not this platform's cart.
    assert!(artwork::prepare(&image_bytes(1000, 574), Platform::Gb).is_err());
    assert!(artwork::prepare(&image_bytes(796, 906), Platform::Gba).is_err());
}

/// A grey cart with a coloured sticker at (x, y, w, h), red on top and blue below.
fn tv_scan(size: (u32, u32), sticker: (u32, u32, u32, u32)) -> Vec<u8> {
    let (x0, y0, sw, sh) = sticker;
    let im = RgbImage::from_fn(size.0, size.1, |x, y| {
        if (x0..x0 + sw).contains(&x) && (y0..y0 + sh).contains(&y) {
            if y < y0 + sh / 2 {
                Rgb([220, 30, 30])
            } else {
                Rgb([30, 30, 220])
            }
        } else {
            Rgb([128, 128, 132])
        }
    });
    let mut out = Cursor::new(Vec::new());
    im.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn nes_and_snes_stickers_are_found_on_any_scan() {
    for (platform, size, sticker, want) in [
        (Platform::Nes, (800, 900), (320, 0, 370, 600), (141, 228)),
        (Platform::Snes, (811, 518), (160, 10, 494, 220), (224, 102)),
        (Platform::Snes, (811, 560), (83, 34, 640, 227), (277, 97)),
    ] {
        let png = artwork::prepare(&tv_scan(size, sticker), platform).unwrap();
        let im = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(im.dimensions(), want);
        assert_eq!(im.get_pixel(want.0 / 2, 1), &Rgb([220, 30, 30]));
        assert_eq!(im.get_pixel(want.0 / 2, want.1 - 2), &Rgb([30, 30, 220]));
    }
    // LaunchBox's stock renders keep their sticker where the recess is, however dark the cart.
    let black = RgbImage::from_pixel(811, 560, Rgb([12, 12, 12]));
    let mut out = Cursor::new(Vec::new());
    black.write_to(&mut out, ImageFormat::Png).unwrap();
    let png = artwork::prepare(&out.into_inner(), Platform::Snes).unwrap();
    assert_eq!(
        image::load_from_memory(&png)
            .unwrap()
            .to_rgb8()
            .dimensions(),
        (277, 97)
    );
    // A Famicom cart's wide sticker is no NES label, and a bare shell has none.
    assert!(artwork::prepare(&tv_scan((800, 525), (55, 54, 694, 337)), Platform::Nes).is_err());
    assert!(artwork::prepare(&tv_scan((800, 520), (0, 0, 0, 0)), Platform::Snes).is_err());
}

struct Fake {
    calls: Vec<String>,
    image: Vec<u8>,
}
impl Transport for Fake {
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Error> {
        self.calls.push(url.into());
        if url.contains("/games/images/") {
            Ok(br#"<a href="https://images.launchbox-app.com/cart.png" data-title="Advance Wars - Cart - Front Image (North America)"></a>"#.to_vec())
        } else {
            Ok(self.image.clone())
        }
    }
}

#[test]
fn end_to_end_writes_exact_filename_and_skips_existing_without_network() {
    let dir = tempfile::tempdir().unwrap();
    let cart = cart("Advance Wars (USA) (Rev 1)");
    let mut http = Fake {
        calls: vec![],
        image: image_bytes(600, 355),
    };
    let mut cache = HashMap::new();
    let file = prepare(&mut http, &mut cache, "", dir.path(), &cart, None).unwrap();
    assert_eq!(
        file,
        dir.path().join("Labels/GBA/Advance Wars (USA) (Rev 1).png")
    );
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 264);
    assert_eq!(http.calls.len(), 2);
    prepare(&mut http, &mut cache, "", dir.path(), &cart, None).unwrap();
    assert_eq!(http.calls.len(), 2);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn a_custom_label_wins_a_publish_race_and_no_partial_files_remain() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("label.png");
    std::fs::write(&file, b"custom").unwrap();
    publish(&file, b"download").unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"custom");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn corrupt_download_is_not_published_and_can_be_retried() {
    let dir = tempfile::tempdir().unwrap();
    let cart = cart("Advance Wars (USA)");
    let mut http = Fake {
        calls: vec![],
        image: b"truncated".to_vec(),
    };
    let mut cache = HashMap::new();
    assert!(prepare(&mut http, &mut cache, "", dir.path(), &cart, None).is_err());
    assert!(!dir.path().join("Labels").exists());
    http.image = image_bytes(600, 355);
    assert!(prepare(&mut http, &mut cache, "", dir.path(), &cart, None).is_ok());
    assert_eq!(
        http.calls.len(),
        3,
        "retry should reuse resolved source URL"
    );
}

#[test]
#[ignore = "live public artwork service; run manually"]
fn live_download() {
    let dir = tempfile::tempdir().unwrap();
    let mut downloader = Downloader::default();
    for name in [
        "Advance Wars (USA)",
        "Mario Kart - Super Circuit (USA)",
        "Lara Croft Tomb Raider - Legend (USA)",
        "Lara Croft Tomb Raider - The Prophecy (USA)",
        "Teenage Mutant Ninja Turtles (USA)",
        "Tom and Jerry - The Magic Ring (USA)",
    ] {
        let path = downloader.prepare(dir.path(), &cart(name)).unwrap();
        let image = image::open(path).unwrap();
        assert_eq!((image.width(), image.height()), (264, 136));
    }
}

#[test]
fn legacy_card_publish_is_complete_and_keeps_existing_labels() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("label.png");
    let make = |bytes: &[u8]| {
        let mut tmp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        tmp.write_all(bytes).unwrap();
        tmp.as_file().sync_all().unwrap();
        tmp
    };
    publish_legacy(make(b"first complete image"), &target).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"first complete image");
    publish_legacy(make(b"replacement"), &target).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"first complete image");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn verified_identity_selects_artwork_but_preserves_rom_filename() {
    let dir = tempfile::tempdir().unwrap();
    let cart = cart("Renamed by me (USA)");
    let mut http = Fake {
        calls: vec![],
        image: image_bytes(600, 355),
    };
    let file = prepare(
        &mut http,
        &mut HashMap::new(),
        "",
        dir.path(),
        &cart,
        Some("Advance Wars"),
    )
    .unwrap();
    assert!(http.calls[0].ends_with("/games/images/2367"));
    assert_eq!(file.file_name().unwrap(), "Renamed by me (USA).png");
    assert_eq!(
        source::title("Lara Croft Tomb Raider - Legend (USA)"),
        "tomb raider legend"
    );
    assert_eq!(
        source::title("Lara Croft Tomb Raider - The Prophecy (USA)"),
        "tomb raider the prophecy"
    );
}

#[test]
fn an_indexed_rom_skips_the_search_and_falls_through_unusable_candidates() {
    struct Images(Vec<String>);
    impl Transport for Images {
        fn get(&mut self, url: &str) -> Result<Vec<u8>, Error> {
            self.0.push(url.into());
            match url.rsplit('/').next().unwrap() {
                "gone.png" => Err(Error::Unavailable("http status: 404".into())),
                "box.png" => Ok(image_bytes(500, 500)),
                _ => Ok(image_bytes(438, 503)),
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let rom = dir.path().join("Games/GB/Anything.gb");
    std::fs::create_dir_all(rom.parent().unwrap()).unwrap();
    // CBF43926 is the CRC32 of these bytes.
    std::fs::write(&rom, b"123456789").unwrap();
    let cart = Cart {
        platform: Platform::Gb,
        stem: "Anything".into(),
        rom,
        ..cart("unused")
    };
    let mut http = Images(vec![]);
    let file = prepare(
        &mut http,
        &mut HashMap::new(),
        "CBF43926\tgone.png\tbox.png\tcart.png\n",
        dir.path(),
        &cart,
        None,
    )
    .unwrap();
    assert_eq!(file, dir.path().join("Labels/GB/Anything.png"));
    let label = image::open(file).unwrap();
    assert_eq!((label.width(), label.height()), label_size(Platform::Gb));
    assert_eq!(
        http.0,
        [
            "https://images.launchbox-app.com/gone.png",
            "https://images.launchbox-app.com/box.png",
            "https://images.launchbox-app.com/cart.png"
        ]
    );
}

/// A Game Boy ROM, on the Game Boy Color shelf, whose CRC32 is the standard check value CBF43926.
fn rom_on_gbc_shelf(dir: &Path) -> Cart {
    let rom = dir.join("Games/GBC/Anything.gb");
    std::fs::create_dir_all(rom.parent().unwrap()).unwrap();
    std::fs::write(&rom, b"123456789").unwrap();
    Cart {
        platform: Platform::Gbc,
        stem: "Anything".into(),
        rom,
        ..cart("unused")
    }
}

fn write_pack(root: &Path, entries: Vec<(u32, Vec<u8>)>) {
    let dir = root.join("Labels");
    std::fs::create_dir_all(&dir).unwrap();
    let mut out = Vec::new();
    build::write_pack(&mut out, entries).unwrap();
    std::fs::write(dir.join(build::PACK), out).unwrap();
}

#[test]
fn a_rom_in_the_artwork_pack_is_labelled_without_the_network() {
    struct NoNetwork;
    impl Transport for NoNetwork {
        fn get(&mut self, url: &str) -> Result<Vec<u8>, Error> {
            panic!("asked the network for {url}");
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let jpeg = build::cart_label_jpeg(&image_bytes(438, 503), Platform::Gb).unwrap();
    assert_eq!(image::guess_format(&jpeg).unwrap(), ImageFormat::Jpeg);
    write_pack(dir.path(), vec![(0xCBF4_3926, jpeg)]);
    let file = prepare(
        &mut NoNetwork,
        &mut HashMap::new(),
        "",
        dir.path(),
        &rom_on_gbc_shelf(dir.path()),
        None,
    )
    .unwrap();
    assert_eq!(file, dir.path().join("Labels/GBC/Anything.png"));
    // Made for the Game Boy label, fitted to the Game Boy Color shelf the cart sits on.
    let label = image::open(file).unwrap();
    assert_eq!((label.width(), label.height()), label_size(Platform::Gbc));
}

#[test]
fn a_pack_without_the_rom_or_a_damaged_pack_leaves_it_to_the_network() {
    for damaged in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        if damaged {
            std::fs::create_dir_all(dir.path().join("Labels")).unwrap();
            std::fs::write(dir.path().join("Labels").join(build::PACK), b"SLOTART1junk").unwrap();
        } else {
            write_pack(dir.path(), vec![(0x1234, b"other".to_vec())]);
        }
        let mut http = Fake {
            calls: vec![],
            image: image_bytes(438, 503),
        };
        prepare(
            &mut http,
            &mut HashMap::new(),
            "CBF43926\tcart.png\n",
            dir.path(),
            &rom_on_gbc_shelf(dir.path()),
            None,
        )
        .unwrap();
        assert_eq!(http.calls, ["https://images.launchbox-app.com/cart.png"]);
    }
}

#[test]
fn versions_are_every_cart_front_cut_to_a_label_and_a_picked_one_replaces_the_label() {
    let dir = tempfile::tempdir().unwrap();
    let mut cart = cart("Advance Wars (USA)");
    let mut http = Fake {
        calls: vec![],
        image: image_bytes(600, 355),
    };
    let found = versions(&mut http, &cart).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].render, found[0].region.as_str()),
        (false, "North America")
    );
    let path = save_label(dir.path(), &cart, &found[0].png).unwrap();
    assert_eq!(path, dir.path().join("Labels/GBA/Advance Wars (USA).png"));
    cart.label = Some(path.clone());
    std::fs::write(&path, b"old").unwrap();
    save_label(dir.path(), &cart, &found[0].png).unwrap();
    assert_ne!(
        std::fs::read(&path).unwrap(),
        b"old",
        "a picked label replaces the one worn"
    );
}

#[test]
fn a_hack_sharing_the_title_loses_to_the_game_people_voted_on() {
    let item = |id, name, votes| {
        format!(
            "<a href='/games/details/{id}-game'><h3>{name}</h3><p>Nintendo Entertainment System</p><p>Community Rating: 4.7 Total Votes: {votes}</p></a>"
        )
    };
    let page = item(109, "Super Mario Bros. 3", 1102) + &item(367956, "Super Mario Bros. 3 +", 15);
    assert_eq!(
        source::game_id(&page, "super mario bros 3", Platform::Nes).unwrap(),
        109
    );
    let tie = item(1, "Super Mario Bros. 3", 0) + &item(2, "Super Mario Bros. 3 +", 0);
    assert!(source::game_id(&tie, "super mario bros 3", Platform::Nes).is_err());
}

#[test]
fn a_search_lists_the_shelfs_games_its_own_platform_first() {
    let item = |id, name, platform| {
        format!("<a href='/games/details/{id}-game'><h3> {name} </h3><p>{platform}</p></a>")
    };
    let page = item(5, "Tetris DX", "Nintendo Game Boy Color")
        + &item(6, "Tetris", "Nintendo Entertainment System")
        + &item(7, "Tetris", "Nintendo Game Boy")
        + &item(7, "Tetris", "Nintendo Game Boy");
    let found = source::games(&page, Platform::Gb);
    let names: Vec<_> = found
        .iter()
        .map(|g| (g.id, g.name.as_str(), g.platform.as_str()))
        .collect();
    assert_eq!(
        names,
        [
            (7, "Tetris", "Game Boy"),
            (5, "Tetris DX", "Game Boy Color")
        ]
    );
}

#[test]
fn a_game_picked_from_a_search_gives_its_versions_and_an_empty_search_asks_nothing() {
    let mut http = Fake {
        calls: vec![],
        image: image_bytes(600, 355),
    };
    assert!(source::search(&mut http, "   ", Platform::Gba).is_err());
    assert!(http.calls.is_empty());
    source::search(&mut http, " advance  wars", Platform::Gba).unwrap();
    assert_eq!(
        http.calls,
        ["https://gamesdb.launchbox-app.com/games/results?id=advance%20wars"]
    );
    let fronts = source::fronts_of(&mut http, &cart("Typo (USA)"), 2367).unwrap();
    assert_eq!(fronts.len(), 1);
    assert_eq!(
        http.calls[1],
        "https://gamesdb.launchbox-app.com/games/images/2367"
    );
}
