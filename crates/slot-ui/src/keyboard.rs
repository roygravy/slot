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
/// The letter chosen grows up out of its key, as a phone shows the key under the thumb.
const BALLOON_RISE: f32 = 50.0;
const BALLOON_WIDEN: f32 = 10.0;
const RING_OUT: f32 = 4.0;
const LEGEND_Y: f32 = 438.0;
const KEYS_LEGEND_GAP: f32 = 22.0;
const KEY_PX: f32 = 25.0;
const BIG_PX: f32 = 38.0;
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

fn balloon_box(row: usize, cap: &Cap) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = key_box(row, cap);
    (
        x - BALLOON_WIDEN,
        y - BALLOON_RISE,
        w + 2.0 * BALLOON_WIDEN,
        h + BALLOON_RISE,
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
                match cap.key {
                    Key::Char(_) => add(shape_of(balloon_box(r, cap), false)),
                    _ => add(shape_of(ring_box(r, cap), true)),
                }
            }
        }
    }
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
    /// The same, larger, for the balloon over the key chosen.
    pub big: Vec<(char, T)>,
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
    /// While nothing is typed, B closes.
    pub legend_empty: Vec<T>,
    pub legend_results: Vec<T>,
    pub legend_back: Vec<T>,
}

impl KeyboardFaces<UndoFace> {
    pub fn build() -> Self {
        let set: String = key_rows(false)
            .iter()
            .chain(key_rows(true).iter())
            .flatten()
            .filter_map(|c| match c.key {
                Key::Char(c) => Some(c),
                _ => None,
            })
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
            big: all
                .iter()
                .map(|&c| (c, plain(&c.to_string(), BIG_PX, 0.0, false)))
                .collect(),
            words: ["space", "search", "123", "ABC"].map(|w| plain(w, WORD_PX, 0.0, false)),
            shapes: shapes().into_iter().map(|s| (s, rounded(s))).collect(),
            icons: icons(),
            placeholder: plain("Game title", FIELD_PX, 0.0, false),
            searching: plain("Searching", 16.0, 3.0, true),
            try_again: plain("Try fewer or other words", 13.0, 2.0, true),
            legend_typing: vec![
                // X still clears the field; its hint makes room for MENU's.
                hint("B", "Delete"),
                hint("Y", "Space"),
                hint("SELECT", "Shift"),
                hint("START", "Search"),
                hint("MENU", "Exit"),
            ],
            legend_empty: vec![
                hint("B", "Close"),
                hint("Y", "Space"),
                hint("SELECT", "Shift"),
                hint("START", "Search"),
                hint("MENU", "Exit"),
            ],
            legend_results: vec![hint("B", "Back"), hint("A", "Choose"), hint("MENU", "Exit")],
            legend_back: vec![hint("B", "Back"), hint("MENU", "Exit")],
        }
    }

    pub fn map<T>(self, mut up: impl FnMut(UndoFace) -> T) -> KeyboardFaces<T> {
        let mut many = |v: Vec<UndoFace>| v.into_iter().map(&mut up).collect::<Vec<T>>();
        let legend_typing = many(self.legend_typing);
        let legend_empty = many(self.legend_empty);
        let legend_results = many(self.legend_results);
        let legend_back = many(self.legend_back);
        KeyboardFaces {
            glyphs: self.glyphs.into_iter().map(|(c, f)| (c, up(f))).collect(),
            big: self.big.into_iter().map(|(c, f)| (c, up(f))).collect(),
            words: self.words.map(&mut up),
            shapes: self.shapes.into_iter().map(|(s, f)| (s, up(f))).collect(),
            icons: self.icons.map(&mut up),
            placeholder: up(self.placeholder),
            searching: up(self.searching),
            try_again: up(self.try_again),
            legend_typing,
            legend_empty,
            legend_results,
            legend_back,
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

pub fn search_status_face(text: &str) -> UndoFace {
    plain(text, 15.0, 3.0, true)
}

/// A game found: its name, cut short with an ellipsis when it is wider than the list, and
/// its platform under it.
pub fn search_row_faces(name: &str, platform: &str) -> (UndoFace, UndoFace) {
    let (px, tracking) = (19.0, 3.0);
    let fits = |s: &str| {
        text::label_font().is_none_or(|font| {
            text::line_width(font, &s.to_uppercase(), px, tracking) + 4.0 <= ROW_MAX
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
        plain(&shown, px, tracking, true),
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
            let mut balloon = None;
            for (r, caps) in rows.iter().enumerate() {
                for (c, cap) in caps.iter().enumerate() {
                    let b = key_box(r, cap);
                    let chosen = r == row && c == col;
                    let (fill, glyph) = match cap.key {
                        Key::Char(_) => (k.key, k.glyph),
                        Key::Space => (k.key, k.glyph),
                        Key::Search => (faded(v.ink.bar, a), faded(v.ink.on_bar, a)),
                        Key::Shift if shift => (k.balloon, k.glyph),
                        _ => (k.func, k.glyph),
                    };
                    shape(b, false, fill, out);
                    let face = match cap.key {
                        Key::Char(ch) => {
                            let ch = if shift { ch.to_ascii_uppercase() } else { ch };
                            if chosen {
                                balloon = Some((r, *cap, ch));
                            }
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
                    if chosen && !matches!(cap.key, Key::Char(_)) {
                        shape(ring_box(r, cap), true, faded(v.ink.text, a), out);
                    }
                }
            }
            if let Some((r, cap, ch)) = balloon {
                let b = balloon_box(r, &cap);
                // A soft shadow under it, then the balloon and its letter, large.
                shape(b, false, [0.0, 0.0, 0.0, 0.25 * a], out);
                shape((b.0, b.1 - 1.0, b.2, b.3), false, k.balloon, out);
                if let Some((_, f)) = v.faces.big.iter().find(|(g, _)| *g == ch) {
                    centred_in(*f, at((b.0, b.1, b.2, BALLOON_RISE + 6.0)), k.glyph, out);
                }
            }
            let typed = v.texts.typed.is_some();
            legend(
                if typed {
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
            legend(&v.faces.legend_back, LEGEND_GAP, out);
        }
        SearchScreen::Nothing => {
            heading(v.texts.heading, HEADING_Y, faded(v.ink.dim, a), out);
            if let Some(s) = v.texts.status {
                heading(s, 200.0, faded(v.ink.text, a), out);
            }
            heading(v.faces.try_again, 236.0, faded(v.ink.dim, a), out);
            legend(&v.faces.legend_back, LEGEND_GAP, out);
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
            legend(&v.faces.legend_results, LEGEND_GAP, out);
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
