//! Searching for a label by name: a keyboard laid out as a phone's, a field over it with what
//! is typed, and the games the search found as a list to pick from.
use crate::draw::{Canvas, Draw, TexId, OUT_W};
use crate::plate::{centred_hints, hint_face, tint_mask, UndoFace, HINT_H, LEGEND_GAP};
use crate::text;
use crate::theme::Ink;

/// The keyboard is ten letter keys wide.
pub const UNITS: f32 = 10.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
    Shift,
    Delete,
    /// Between the letters and the numbers.
    Layer,
    Space,
    Search,
}

/// A key and where it sits in its row, in letter keys from the left.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Cap {
    pub key: Key,
    pub x: f32,
    pub w: f32,
}

fn chars(keys: &str, from: f32, w: f32) -> Vec<Cap> {
    keys.chars()
        .enumerate()
        .map(|(i, c)| Cap {
            key: Key::Char(c),
            x: from + i as f32 * w,
            w,
        })
        .collect()
}

/// The rows, top to bottom: the letters as a phone lays them out, or its numbers and marks.
pub fn key_rows(numbers: bool) -> [Vec<Cap>; 4] {
    let cap = |key, x, w| Cap { key, x, w };
    let bottom = vec![
        cap(Key::Layer, 0.0, 2.5),
        cap(Key::Space, 2.5, 5.0),
        cap(Key::Search, 7.5, 2.5),
    ];
    if numbers {
        let mut third = chars(".,?!'", 1.5, 1.4);
        third.push(cap(Key::Delete, 8.5, 1.5));
        [
            chars("1234567890", 0.0, 1.0),
            chars("-/:;()$&@\"", 0.0, 1.0),
            third,
            bottom,
        ]
    } else {
        let mut third = vec![cap(Key::Shift, 0.0, 1.5)];
        third.extend(chars("zxcvbnm", 1.5, 1.0));
        third.push(cap(Key::Delete, 8.5, 1.5));
        [
            chars("qwertyuiop", 0.0, 1.0),
            chars("asdfghjkl", 0.5, 1.0),
            third,
            bottom,
        ]
    }
}

/// The letters a key also types, as a phone offers them when the key is held: X shows them.
pub fn accents(c: char) -> &'static [char] {
    match c {
        'a' => &['à', 'á', 'â', 'ä', 'ã', 'å', 'æ'],
        'c' => &['ç'],
        'e' => &['è', 'é', 'ê', 'ë'],
        'i' => &['ì', 'í', 'î', 'ï'],
        'n' => &['ñ'],
        'o' => &['ò', 'ó', 'ô', 'ö', 'õ', 'ø', 'œ'],
        's' => &['ß'],
        'u' => &['ù', 'ú', 'û', 'ü'],
        'y' => &['ý', 'ÿ'],
        _ => &[],
    }
}

// ---- where it all goes on the 640 x 480 screen ----
const HEADING_Y: f32 = 18.0;
const FIELD: (f32, f32, f32, f32) = (40.0, 56.0, 560.0, 46.0);
const PANEL_Y: f32 = 150.0;
const PANEL_H: f32 = 268.0;
const ROW_Y: f32 = 162.0;
const ROW_PITCH: f32 = 62.0;
const KEY_H: f32 = 50.0;
const MARGIN: f32 = 6.0;
const GAP: f32 = 7.0;
const RADIUS: f32 = 8.0;
const RING_OUT: f32 = 4.0;
/// The accents of the key chosen, in a row of keys between the field and the keyboard.
const STRIP_Y: f32 = 108.0;
const STRIP_KEY: (f32, f32) = (48.0, 36.0);
const STRIP_GAP: f32 = 6.0;
const LEGEND_Y: f32 = 438.0;
const KEYS_LEGEND_GAP: f32 = 22.0;
const KEY_PX: f32 = 25.0;
const WORD_PX: f32 = 19.0;
const FIELD_PX: f32 = 23.0;
const ICON: u32 = 26;
/// The results' rows.
const LIST_Y: f32 = 66.0;
const LIST_PITCH: f32 = 58.0;
const LIST_SHOWN: usize = 6;
const LIST_X: f32 = 32.0;

fn unit() -> f32 {
    (OUT_W as f32 - 2.0 * MARGIN) / UNITS
}

/// A key's box on the screen, before the keyboard is centred on a wider one.
fn key_box(row: usize, cap: &Cap) -> (f32, f32, f32, f32) {
    let u = unit();
    let x = (MARGIN + cap.x * u + GAP / 2.0).round();
    let w = (cap.w * u - GAP).round();
    (x, ROW_Y + row as f32 * ROW_PITCH, w, KEY_H)
}

/// The accents' keys, `n` of them, centred under the key they belong to and kept on screen.
fn strip_boxes(row: usize, cap: &Cap, n: usize) -> Vec<(f32, f32, f32, f32)> {
    let (kx, _, kw, _) = key_box(row, cap);
    let (w, h) = STRIP_KEY;
    let total = n as f32 * w + (n as f32 - 1.0) * STRIP_GAP;
    let x0 = (kx + kw / 2.0 - total / 2.0)
        .clamp(MARGIN, OUT_W as f32 - MARGIN - total)
        .round();
    (0..n)
        .map(|i| (x0 + i as f32 * (w + STRIP_GAP), STRIP_Y, w, h))
        .collect()
}

fn strip_ring(b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (
        b.0 - RING_OUT,
        b.1 - RING_OUT,
        b.2 + 2.0 * RING_OUT,
        b.3 + 2.0 * RING_OUT,
    )
}

fn ring_box(row: usize, cap: &Cap) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = key_box(row, cap);
    (
        x - RING_OUT,
        y - RING_OUT,
        w + 2.0 * RING_OUT,
        h + 2.0 * RING_OUT,
    )
}

/// The rounded shapes, by size: filled, or a ring.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Shape {
    pub w: u32,
    pub h: u32,
    pub ring: bool,
}

fn shape_of(b: (f32, f32, f32, f32), ring: bool) -> Shape {
    Shape {
        w: b.2 as u32,
        h: b.3 as u32,
        ring,
    }
}

/// Every shape the keyboard draws.
fn shapes() -> Vec<Shape> {
    let mut all: Vec<Shape> = Vec::new();
    let mut add = |s: Shape| {
        if !all.contains(&s) {
            all.push(s);
        }
    };
    for numbers in [false, true] {
        for (r, row) in key_rows(numbers).iter().enumerate() {
            for cap in row {
                add(shape_of(key_box(r, cap), false));
                add(shape_of(ring_box(r, cap), true));
            }
        }
    }
    let strip = (0.0, 0.0, STRIP_KEY.0, STRIP_KEY.1);
    add(shape_of(strip, false));
    add(shape_of(strip_ring(strip), true));
    add(Shape {
        w: FIELD.2 as u32,
        h: FIELD.3 as u32,
        ring: false,
    });
    all
}

fn rounded(s: Shape) -> UndoFace {
    let r = if s.h as f32 >= FIELD.3 && !s.ring && s.w as f32 >= FIELD.2 {
        FIELD.3 / 2.0
    } else {
        RADIUS + if s.ring { RING_OUT } else { 0.0 }
    };
    let (hw, hh) = (s.w as f32 / 2.0, s.h as f32 / 2.0);
    let mut rgba = vec![0u8; (s.w * s.h * 4) as usize];
    for y in 0..s.h {
        for x in 0..s.w {
            let (px, py) = (x as f32 + 0.5 - hw, y as f32 + 0.5 - hh);
            let (qx, qy) = (px.abs() - hw + r, py.abs() - hh + r);
            let d = (qx.max(0.0).hypot(qy.max(0.0))) + qx.max(qy).min(0.0) - r;
            let a = if s.ring {
                (0.5 - d).clamp(0.0, 1.0) * (0.5 + d + 2.0).clamp(0.0, 1.0)
            } else {
                (0.5 - d).clamp(0.0, 1.0)
            };
            let at = ((y * s.w + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    UndoFace {
        rgba,
        w: s.w,
        h: s.h,
    }
}

/// White text on nothing, set as it is typed (no capitals forced), for `Draw::Tint`.
fn plain(s: &str, px: f32, tracking: f32, upper: bool) -> UndoFace {
    let line = if upper {
        s.to_uppercase()
    } else {
        s.to_owned()
    };
    let h = (px * 1.6).ceil() as u32;
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: vec![0; 4],
            w: 1,
            h: 1,
        };
    };
    let w = (text::line_width(font, &line, px, tracking).ceil() as u32 + 4).max(1);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let layout = text::Layout {
        lines: vec![line],
        px,
        tracking,
    };
    text::draw_centred(&mut rgba, w, h, &layout, [255, 255, 255]);
    UndoFace { rgba, w, h }
}

/// Distance from a point to a segment.
fn seg(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

fn inside(p: (f32, f32), pts: &[(f32, f32)]) -> bool {
    let mut odd = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// An icon drawn by a coverage function over its square, centred on the origin, with four
/// samples a pixel.
fn icon(cover: impl Fn(f32, f32) -> bool) -> UndoFace {
    let s = ICON;
    let c = s as f32 / 2.0;
    let mut rgba = vec![0u8; (s * s * 4) as usize];
    for y in 0..s {
        for x in 0..s {
            let mut a = 0.0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let (px, py) = (
                        x as f32 + (sx as f32 + 0.5) / 4.0 - c,
                        y as f32 + (sy as f32 + 0.5) / 4.0 - c,
                    );
                    a += f32::from(u8::from(cover(px, py)));
                }
            }
            let at = ((y * s + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, (a / 16.0 * 255.0) as u8]);
        }
    }
    UndoFace { rgba, w: s, h: s }
}

fn outline(pts: &[(f32, f32)], stroke: f32, fill: bool) -> impl Fn(f32, f32) -> bool + '_ {
    move |x, y| {
        let p = (x, y);
        let edge = (0..pts.len()).any(|i| seg(p, pts[i], pts[(i + 1) % pts.len()]) <= stroke / 2.0);
        edge || (fill && inside(p, pts))
    }
}

const SHIFT: [(f32, f32); 7] = [
    (0.0, -9.0),
    (9.0, 0.5),
    (4.0, 0.5),
    (4.0, 8.0),
    (-4.0, 8.0),
    (-4.0, 0.5),
    (-9.0, 0.5),
];
const DELETE: [(f32, f32); 5] = [
    (-11.0, 0.0),
    (-5.0, -8.0),
    (11.0, -8.0),
    (11.0, 8.0),
    (-5.0, 8.0),
];

/// Shift, shift held, delete, the magnifier and the clear button.
fn icons() -> [UndoFace; 5] {
    let stroke = 1.9;
    let cross = |x: f32, y: f32, cx: f32, arm: f32| {
        (x - cx).abs() <= arm
            && y.abs() <= arm
            && ((x - cx - y).abs() <= stroke * 0.75 || (x - cx + y).abs() <= stroke * 0.75)
    };
    [
        icon(outline(&SHIFT, stroke, false)),
        icon(outline(&SHIFT, stroke, true)),
        icon(move |x, y| outline(&DELETE, stroke, false)(x, y) || cross(x, y, 2.5, 3.6)),
        icon(move |x, y| {
            let d = (x + 2.0).hypot(y + 2.0);
            (d <= 7.5 && d >= 7.5 - 2.3) || seg((x, y), (3.6, 3.6), (9.0, 9.0)) <= 1.4
        }),
        // A disc with the cross knocked out of it.
        icon(move |x, y| x.hypot(y) <= 9.0 && !cross(x, y, 0.0, 3.8)),
    ]
}

/// A face, its texture and size, once uploaded.
pub type Up = (TexId, u32, u32);

/// The keyboard's faces. Built once, as `UndoFace`s, then uploaded in place with `map`.
pub struct KeyboardFaces<T> {
    /// Each key's letter, lower and upper case, and its numbers and marks.
    pub glyphs: Vec<(char, T)>,
    /// space, search, 123, ABC.
    pub words: [T; 4],
    pub shapes: Vec<(Shape, T)>,
    /// Shift, shift held, delete, magnifier, clear.
    pub icons: [T; 5],
    /// The field's hint while nothing is typed.
    pub placeholder: T,
    pub searching: T,
    /// Under "nothing found": what to try.
    pub try_again: T,
    pub legend_typing: Vec<T>,
    /// On a letter with accents: X shows them.
    pub legend_accents: Vec<T>,
    /// While the accents show.
    pub legend_strip: Vec<T>,
    /// While nothing is typed, B closes.
    pub legend_empty: Vec<T>,
    pub legend_results: Vec<T>,
    pub legend_back: Vec<T>,
    /// Legends of the screens built on this one, picked by the caller (see `WifiLegend`).
    pub extra: Vec<Vec<T>>,
}

/// What the keyboard is for: the words on its keys and legends follow.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Purpose {
    Search,
    Password,
}

/// The Wi-Fi pages' own legends, in `KeyboardFaces::extra` of the password faces.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WifiLegend {
    Nearby = 0,
    Saved = 1,
    Confirm = 2,
    Wps = 3,
    Zones = 4,
    Back = 5,
}

impl KeyboardFaces<UndoFace> {
    pub fn build() -> Self {
        Self::build_for(Purpose::Search)
    }

    pub fn build_for(purpose: Purpose) -> Self {
        let pw = purpose == Purpose::Password;
        let go = if pw { "Join" } else { "Search" };
        let set: String = key_rows(false)
            .iter()
            .chain(key_rows(true).iter())
            .flatten()
            .filter_map(|c| match c.key {
                Key::Char(c) => Some(c),
                _ => None,
            })
            .flat_map(|c| std::iter::once(c).chain(accents(c).iter().copied()))
            .collect();
        let all: Vec<char> = set
            .chars()
            .chain(
                set.chars()
                    .filter(|c| c.is_alphabetic())
                    .flat_map(char::to_uppercase),
            )
            .collect();
        let hint = |k: &str, l: &str| tint_mask(hint_face(k, l));
        KeyboardFaces {
            glyphs: all
                .iter()
                .map(|&c| (c, plain(&c.to_string(), KEY_PX, 0.0, false)))
                .collect(),
            words: ["space", if pw { "join" } else { "search" }, "123", "ABC"]
                .map(|w| plain(w, WORD_PX, 0.0, false)),
            shapes: shapes().into_iter().map(|s| (s, rounded(s))).collect(),
            icons: icons(),
            placeholder: plain(if pw { "Password" } else { "Game title" }, FIELD_PX, 0.0, false),
            searching: plain(if pw { "Scanning" } else { "Searching" }, 16.0, 3.0, true),
            try_again: plain(
                if pw {
                    "Move closer to the router"
                } else {
                    "Try fewer or other words"
                },
                13.0,
                2.0,
                true,
            ),
            legend_typing: vec![
                // X still clears the field; its hint makes room for MENU's.
                hint("B", "Delete"),
                hint("Y", "Space"),
                hint("SELECT", "Shift"),
                hint("START", go),
                hint("MENU", "Exit"),
            ],
            legend_accents: vec![
                hint("B", "Delete"),
                // START's hint makes room; the search key still shows it.
                hint("X", "Accents"),
                hint("SELECT", "Shift"),
                hint("MENU", "Exit"),
            ],
            legend_strip: vec![hint("A", "Type"), hint("B", "Close"), hint("MENU", "Exit")],
            legend_empty: vec![
                hint("B", "Close"),
                hint("Y", "Space"),
                hint("SELECT", "Shift"),
                hint("START", go),
                hint("MENU", "Exit"),
            ],
            legend_results: vec![hint("B", "Back"), hint("A", "Choose"), hint("MENU", "Exit")],
            legend_back: vec![hint("B", "Back"), hint("MENU", "Exit")],
            extra: if pw {
                vec![
                    vec![hint("B", "Back"), hint("A", "Join"), hint("Y", "Scan Again")],
                    vec![hint("B", "Back"), hint("Y", "Forget")],
                    vec![hint("B", "Cancel"), hint("A", "Forget")],
                    vec![hint("B", "Cancel")],
                    vec![hint("B", "Back"), hint("A", "Choose")],
                    vec![hint("B", "Back")],
                ]
            } else {
                Vec::new()
            },
        }
    }

    pub fn map<T>(self, mut up: impl FnMut(UndoFace) -> T) -> KeyboardFaces<T> {
        let mut many = |v: Vec<UndoFace>| v.into_iter().map(&mut up).collect::<Vec<T>>();
        let legend_typing = many(self.legend_typing);
        let legend_accents = many(self.legend_accents);
        let legend_strip = many(self.legend_strip);
        let legend_empty = many(self.legend_empty);
        let legend_results = many(self.legend_results);
        let legend_back = many(self.legend_back);
        let extra = self.extra.into_iter().map(&mut many).collect();
        KeyboardFaces {
            glyphs: self.glyphs.into_iter().map(|(c, f)| (c, up(f))).collect(),
            words: self.words.map(&mut up),
            shapes: self.shapes.into_iter().map(|(s, f)| (s, up(f))).collect(),
            icons: self.icons.map(&mut up),
            placeholder: up(self.placeholder),
            searching: up(self.searching),
            try_again: up(self.try_again),
            legend_typing,
            legend_accents,
            legend_strip,
            legend_empty,
            legend_results,
            legend_back,
            extra,
        }
    }
}

/// The search's own words, made again as they change.
pub struct SearchTexts<T> {
    /// "Search for a label · Game Boy".
    pub heading: T,
    /// What is typed; None while nothing is.
    pub typed: Option<T>,
    /// Over the results: how many were found, or that none were.
    pub status: Option<T>,
    /// Each game found: its name, and its platform under it.
    pub rows: Vec<(T, T)>,
    /// Under the status on the message screen, in place of "try fewer words".
    pub note: Option<T>,
    /// Large over the status on the message screen: the ROM Manager's PIN.
    pub big: Option<T>,
}

pub fn search_heading_face(text: &str) -> UndoFace {
    plain(text, 13.0, 3.0, true)
}

/// How wide the typed words can show in the field, between the magnifier and the clear button.
const TYPED_MAX: u32 = FIELD.2 as u32 - 100;
/// How far in the first letters shown fade, when the start of a long name is cut off.
const TYPED_FADE: u32 = 24;
/// How wide a game's name can show in the list.
const ROW_MAX: f32 = OUT_W as f32 - 2.0 * LIST_X;

/// What is typed. A name longer than the field shows its end, where the caret is, the
/// letters before it fading out at the field's left, as a phone's search field does.
pub fn search_typed_face(text: &str) -> UndoFace {
    let face = plain(text, FIELD_PX, 0.0, false);
    if face.w <= TYPED_MAX {
        return face;
    }
    let (w, h) = (TYPED_MAX, face.h);
    let skip = face.w - w;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let from = ((y * face.w + x + skip) * 4) as usize;
            let to = ((y * w + x) * 4) as usize;
            rgba[to..to + 4].copy_from_slice(&face.rgba[from..from + 4]);
            if x < TYPED_FADE {
                let a = rgba[to + 3] as u32 * x / TYPED_FADE;
                rgba[to + 3] = a as u8;
            }
        }
    }
    UndoFace { rgba, w, h }
}

/// The ROM Manager's PIN, big enough to read off the console from arm's length.
pub fn search_big_face(text: &str) -> UndoFace {
    plain(text, 64.0, 18.0, false)
}

pub fn search_status_face(text: &str) -> UndoFace {
    plain(text, 15.0, 3.0, true)
}

/// A game found: its name, cut short with an ellipsis when it is wider than the list, and
/// its platform under it.
pub fn search_row_faces(name: &str, platform: &str) -> (UndoFace, UndoFace) {
    list_row_faces(name, platform, true)
}

/// A row of a list on this screen; `upper` false keeps a network's name as it is written.
pub fn list_row_faces(name: &str, platform: &str, upper: bool) -> (UndoFace, UndoFace) {
    let (px, tracking) = (19.0, 3.0);
    let fits = |s: &str| {
        text::label_font().is_none_or(|font| {
            let s = if upper { s.to_uppercase() } else { s.to_owned() };
            text::line_width(font, &s, px, tracking) + 4.0 <= ROW_MAX
        })
    };
    let mut shown = name.trim().to_owned();
    if !fits(&shown) {
        let mut chars: Vec<char> = shown.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let cut: String = chars.iter().collect();
            let cut = format!("{}…", cut.trim_end());
            if fits(&cut) {
                shown = cut;
                break;
            }
        }
    }
    (
        plain(&shown, px, if upper { tracking } else { 1.0 }, upper),
        plain(platform, 11.0, 2.5, true),
    )
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SearchScreen {
    Keyboard {
        row: usize,
        col: usize,
        numbers: bool,
        shift: bool,
        /// The caret, which blinks.
        caret: bool,
        /// The chosen key's accents are showing, this one of them chosen.
        accent: Option<usize>,
    },
    Searching,
    Results {
        at: usize,
    },
    /// Nothing was found, or the search could not be made.
    Nothing,
}

pub struct SearchView<'a> {
    pub faces: &'a KeyboardFaces<Up>,
    pub texts: &'a SearchTexts<Up>,
    pub ink: Ink,
    pub ground: [f32; 4],
    pub light: bool,
    pub screen: SearchScreen,
    /// How far it has faded in.
    pub openness: f32,
    /// In place of the search's own legend, on every screen but the keyboard.
    pub legend: Option<&'a [Up]>,
}

fn rgb(c: [u8; 3], a: f32) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        a,
    ]
}

/// The keyboard's colours, as a phone's in its dark or light look: the panel, a letter key, a
/// key that does something else, the balloon and the field.
struct Keys {
    panel: [f32; 4],
    key: [f32; 4],
    func: [f32; 4],
    balloon: [f32; 4],
    field: [f32; 4],
    glyph: [f32; 4],
    faint: [f32; 4],
}

fn keys(light: bool, a: f32) -> Keys {
    if light {
        Keys {
            panel: rgb([0xd3, 0xd2, 0xcd], a),
            key: rgb([0xfb, 0xfb, 0xf9], a),
            func: rgb([0xaa, 0xab, 0xa8], a),
            balloon: rgb([0xff, 0xff, 0xff], a),
            field: rgb([0xd8, 0xd6, 0xd0], a),
            glyph: rgb([0x1c, 0x1c, 0x1e], a),
            faint: rgb([0x80, 0x80, 0x84], a),
        }
    } else {
        Keys {
            panel: rgb([0x2b, 0x2b, 0x2e], a),
            key: rgb([0x6b, 0x6b, 0x6e], a),
            func: rgb([0x46, 0x46, 0x4a], a),
            balloon: rgb([0x96, 0x96, 0x9a], a),
            field: rgb([0x3a, 0x3a, 0x3e], a),
            glyph: rgb([0xff, 0xff, 0xff], a),
            faint: rgb([0x9c, 0x9c, 0xa0], a),
        }
    }
}

fn faded(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

fn tint(tex: TexId, x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) -> Draw {
    Draw::Tint {
        x: x.round(),
        y: y.round(),
        w,
        h,
        tex,
        colour,
    }
}

/// A face centred in a box.
fn centred_in(f: Up, b: (f32, f32, f32, f32), colour: [f32; 4], out: &mut Vec<Draw>) {
    out.push(tint(
        f.0,
        b.0 + (b.2 - f.1 as f32) / 2.0,
        b.1 + (b.3 - f.2 as f32) / 2.0,
        f.1 as f32,
        f.2 as f32,
        colour,
    ));
}

pub fn draw_label_search(canvas: Canvas, v: &SearchView, out: &mut Vec<Draw>) {
    let a = v.openness.clamp(0.0, 1.0);
    let k = keys(v.light, a);
    let ox = canvas.centred(OUT_W as f32).round();
    let at = |b: (f32, f32, f32, f32)| (b.0 + ox, b.1, b.2, b.3);
    let shape = |b: (f32, f32, f32, f32), ring: bool, colour: [f32; 4], out: &mut Vec<Draw>| {
        let want = shape_of(b, ring);
        if let Some((_, f)) = v.faces.shapes.iter().find(|(s, _)| *s == want) {
            out.push(tint(f.0, b.0 + ox, b.1, f.1 as f32, f.2 as f32, colour));
        }
    };
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: canvas.h,
        colour: faded(v.ground, a),
    });
    let legend = |faces: &[Up], gap: f32, out: &mut Vec<Draw>| {
        let pairs: Vec<(TexId, u32)> = faces.iter().map(|f| (f.0, f.1)).collect();
        for (tex, w, x) in centred_hints(canvas, &pairs, gap) {
            out.push(tint(
                tex,
                x,
                LEGEND_Y,
                w as f32,
                HINT_H as f32,
                faded(v.ink.text, a),
            ));
        }
    };
    let heading = |f: Up, y: f32, colour: [f32; 4], out: &mut Vec<Draw>| {
        out.push(tint(
            f.0,
            canvas.centred(f.1 as f32),
            y,
            f.1 as f32,
            f.2 as f32,
            colour,
        ));
    };

    match v.screen {
        SearchScreen::Keyboard {
            row,
            col,
            numbers,
            shift,
            caret,
            accent,
        } => {
            heading(v.texts.heading, HEADING_Y, faded(v.ink.dim, a), out);
            // The field: a pill with the magnifier, what is typed and its caret, and the clear
            // button once there is something to clear.
            shape(FIELD, false, k.field, out);
            let (fx, fy, fw, fh) = at(FIELD);
            let ic = v.faces.icons[3];
            out.push(tint(
                ic.0,
                fx + 14.0,
                fy + (fh - ic.2 as f32) / 2.0,
                ic.1 as f32,
                ic.2 as f32,
                k.faint,
            ));
            let tx = fx + 48.0;
            let typed_w = match v.texts.typed {
                Some(f) => {
                    out.push(tint(
                        f.0,
                        tx,
                        fy + (fh - f.2 as f32) / 2.0,
                        f.1 as f32,
                        f.2 as f32,
                        k.glyph,
                    ));
                    let clear = v.faces.icons[4];
                    out.push(tint(
                        clear.0,
                        fx + fw - 14.0 - clear.1 as f32,
                        fy + (fh - clear.2 as f32) / 2.0,
                        clear.1 as f32,
                        clear.2 as f32,
                        k.faint,
                    ));
                    f.1 as f32 - 2.0
                }
                None => {
                    let p = v.faces.placeholder;
                    out.push(tint(
                        p.0,
                        tx,
                        fy + (fh - p.2 as f32) / 2.0,
                        p.1 as f32,
                        p.2 as f32,
                        k.faint,
                    ));
                    0.0
                }
            };
            if caret {
                out.push(Draw::Rect {
                    x: (tx + typed_w + 1.0).round(),
                    y: fy + 11.0,
                    w: 2.0,
                    h: fh - 22.0,
                    colour: faded(v.ink.text, a),
                });
            }

            out.push(Draw::Rect {
                x: 0.0,
                y: PANEL_Y,
                w: canvas.w,
                h: PANEL_H,
                colour: k.panel,
            });
            let rows = key_rows(numbers);
            let mut strip = None;
            for (r, caps) in rows.iter().enumerate() {
                for (c, cap) in caps.iter().enumerate() {
                    let b = key_box(r, cap);
                    let chosen = r == row && c == col;
                    let (fill, glyph) = match cap.key {
                        // The key chosen lights up where it is: nothing floats over the others.
                        Key::Char(_) if chosen => (k.balloon, k.glyph),
                        Key::Char(_) => (k.key, k.glyph),
                        Key::Space => (k.key, k.glyph),
                        Key::Search => (faded(v.ink.bar, a), faded(v.ink.on_bar, a)),
                        Key::Shift if shift => (k.balloon, k.glyph),
                        _ => (k.func, k.glyph),
                    };
                    shape(b, false, fill, out);
                    let face = match cap.key {
                        Key::Char(ch) => {
                            if chosen {
                                strip = accent.map(|at| (r, *cap, ch, at));
                            }
                            let ch = if shift { ch.to_ascii_uppercase() } else { ch };
                            v.faces
                                .glyphs
                                .iter()
                                .find(|(g, _)| *g == ch)
                                .map(|(_, f)| *f)
                        }
                        Key::Shift => Some(v.faces.icons[usize::from(shift)]),
                        Key::Delete => Some(v.faces.icons[2]),
                        Key::Space => Some(v.faces.words[0]),
                        Key::Search => Some(v.faces.words[1]),
                        Key::Layer => Some(v.faces.words[if numbers { 3 } else { 2 }]),
                    };
                    if let Some(f) = face {
                        centred_in(f, at(b), glyph, out);
                    }
                    if chosen {
                        let ring = if accent.is_some() {
                            v.ink.dim
                        } else {
                            v.ink.text
                        };
                        shape(ring_box(r, cap), true, faded(ring, a), out);
                    }
                }
            }
            if let Some((r, cap, ch, chosen)) = strip {
                let marks = accents(ch);
                for (i, b) in strip_boxes(r, &cap, marks.len()).into_iter().enumerate() {
                    let on = i == chosen;
                    shape(b, false, if on { k.balloon } else { k.key }, out);
                    let mark = if shift {
                        marks[i].to_uppercase().next().unwrap_or(marks[i])
                    } else {
                        marks[i]
                    };
                    if let Some((_, f)) = v.faces.glyphs.iter().find(|(g, _)| *g == mark) {
                        centred_in(*f, at(b), k.glyph, out);
                    }
                    if on {
                        shape(strip_ring(b), true, faded(v.ink.text, a), out);
                    }
                }
            }
            let on_accents = match rows[row].get(col).map(|c| c.key) {
                Some(Key::Char(c)) => !accents(c).is_empty(),
                _ => false,
            };
            legend(
                if accent.is_some() {
                    &v.faces.legend_strip
                } else if on_accents {
                    &v.faces.legend_accents
                } else if v.texts.typed.is_some() {
                    &v.faces.legend_typing
                } else {
                    &v.faces.legend_empty
                },
                KEYS_LEGEND_GAP,
                out,
            );
        }
        SearchScreen::Searching => {
            heading(v.texts.heading, HEADING_Y, faded(v.ink.dim, a), out);
            heading(v.faces.searching, 220.0, faded(v.ink.text, a), out);
            legend(v.legend.unwrap_or(&v.faces.legend_back), LEGEND_GAP, out);
        }
        SearchScreen::Nothing => {
            heading(v.texts.heading, HEADING_Y, faded(v.ink.dim, a), out);
            // With a big line the status and note move down under it.
            let low = match v.texts.big {
                Some(b) => {
                    heading(b, 150.0, faded(v.ink.text, a), out);
                    70.0
                }
                None => 0.0,
            };
            if let Some(s) = v.texts.status {
                heading(s, 200.0 + low, faded(v.ink.text, a), out);
            }
            heading(
                v.texts.note.unwrap_or(v.faces.try_again),
                236.0 + low,
                faded(v.ink.dim, a),
                out,
            );
            legend(v.legend.unwrap_or(&v.faces.legend_back), LEGEND_GAP, out);
        }
        SearchScreen::Results { at: chosen } => {
            if let Some(s) = v.texts.status {
                heading(s, HEADING_Y + 6.0, faded(v.ink.dim, a), out);
            }
            let n = v.texts.rows.len();
            let first = chosen
                .saturating_sub(LIST_SHOWN - 1)
                .min(n.saturating_sub(LIST_SHOWN));
            for (i, (name, platform)) in
                v.texts.rows.iter().enumerate().skip(first).take(LIST_SHOWN)
            {
                let y = LIST_Y + (i - first) as f32 * LIST_PITCH;
                let on = i == chosen;
                if on {
                    out.push(Draw::Rect {
                        x: 0.0,
                        y,
                        w: canvas.w,
                        h: LIST_PITCH - 4.0,
                        colour: faded(v.ink.bar, a),
                    });
                }
                let (big, small) = if on {
                    (faded(v.ink.on_bar, a), faded(v.ink.on_bar, 0.6 * a))
                } else {
                    (faded(v.ink.text, a), faded(v.ink.dim, a))
                };
                let x = ox + LIST_X;
                out.push(tint(name.0, x, y + 6.0, name.1 as f32, name.2 as f32, big));
                out.push(tint(
                    platform.0,
                    x,
                    y + 34.0,
                    platform.1 as f32,
                    platform.2 as f32,
                    small,
                ));
            }
            legend(v.legend.unwrap_or(&v.faces.legend_results), LEGEND_GAP, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_fills_the_keyboard_and_no_keys_overlap() {
        for numbers in [false, true] {
            for row in key_rows(numbers) {
                for pair in row.windows(2) {
                    assert!(pair[0].x + pair[0].w <= pair[1].x + 1e-4);
                }
                let end = row.last().map(|c| c.x + c.w).unwrap();
                assert!(end <= UNITS + 1e-4);
            }
        }
    }

    #[test]
    fn every_key_shape_is_made() {
        let all = shapes();
        for numbers in [false, true] {
            for (r, row) in key_rows(numbers).iter().enumerate() {
                for cap in row {
                    assert!(all.contains(&shape_of(key_box(r, cap), false)));
                }
            }
        }
    }
}
