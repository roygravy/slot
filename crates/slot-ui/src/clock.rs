use slot_gfx::{Canvas, Draw, TexId};

use crate::quick_menu::{draw_menu_ground, LEGEND_Y};
use crate::theme::Ink;
use slot_store::{
    civil_from_days, days_from_civil, days_in_month, parse_stamp, zone_named, Zone, ZONES,
};

use crate::plate::{blit, centred_hints, hint_width, UndoFace, HINT_H, LEGEND_GAP};
use crate::text;

const DAY: i64 = 86_400;

const YEAR_MIN: i64 = 2000;
const YEAR_MAX: i64 = 2099;

/// White, so the menu's ink can tint it.
const INK: [u8; 3] = [0xff, 0xff, 0xff];

/// The wheels: the time's on one page, the date's on the next, as a phone's are. Each wheel
/// shows its value large, with the one before over it and the one after under it, faint.
const WHEEL_W: u32 = 560;
const WHEEL_H: u32 = 250;
const ROW_Y: [u32; 3] = [48, 118, 188];
const BIG_PX: f32 = 60.0;
const SMALL_PX: f32 = 36.0;
const FAINT: u32 = 70;
const UNDER_Y: u32 = 236;
const UNDER_PX: f32 = 18.0;
/// The ring round the wheel being turned.
const RING_H: f32 = 74.0;
const RING_W: f32 = 3.0;
const WEEKDAYS: [&str; 7] = ["THU", "FRI", "SAT", "SUN", "MON", "TUE", "WED"];

const SET_CLOCK_KEY: &str = "A";
const SET_CLOCK_LABEL: &str = "set the clock";

pub fn clock_label(stamp: &str) -> String {
    parse_stamp(stamp).map(hhmm).unwrap_or_default()
}

/// The time of day as the status bar shows it: 18:05, or 6:05 PM on a 12-hour clock.
pub fn clock_text(secs: i64, h24: bool) -> String {
    if h24 {
        return hhmm(secs);
    }
    let rem = secs.rem_euclid(DAY);
    let (hour, minute) = (rem / 3600, rem / 60 % 60);
    let half = if hour < 12 { "AM" } else { "PM" };
    format!("{}:{minute:02} {half}", (hour + 11) % 12 + 1)
}

pub fn hhmm(secs: i64) -> String {
    let rem = secs.rem_euclid(DAY);
    format!("{:02}:{:02}", rem / 3600, rem / 60 % 60)
}

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

pub fn date_time_text_in(secs: i64, h24: bool) -> String {
    let (_, month, day) = civil_from_days(secs.div_euclid(DAY));
    let name = MONTHS
        .get((month - 1) as usize)
        .copied()
        .unwrap_or_default();
    format!("{name} {day} {}", clock_text(secs, h24))
}

pub fn date_time_text(secs: i64) -> String {
    let (_, month, day) = civil_from_days(secs.div_euclid(DAY));
    let name = MONTHS
        .get((month - 1) as usize)
        .copied()
        .unwrap_or_default();
    format!("{name} {day} {}", hhmm(secs))
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Field {
    Year,
    Month,
    Day,
    Hour,
    Minute,
}

impl Field {
    /// The time first, then the date: left and right walk across both pages.
    pub const ALL: [Field; 5] = [
        Field::Hour,
        Field::Minute,
        Field::Day,
        Field::Month,
        Field::Year,
    ];

    fn on_time_page(self) -> bool {
        matches!(self, Field::Hour | Field::Minute)
    }
}

/// Where the clock is: a city that keeps its own summer time, or a bare
/// offset from before cities could be picked.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TimeZone {
    Fixed(i64),
    City(&'static Zone),
}

impl TimeZone {
    pub fn from_state(name: Option<&str>, offset_min: i16) -> Self {
        name.and_then(zone_named)
            .map_or(TimeZone::Fixed(i64::from(offset_min)), TimeZone::City)
    }

    pub fn offset_at(&self, utc: i64) -> i64 {
        match self {
            TimeZone::Fixed(min) => *min,
            TimeZone::City(zone) => zone.offset_at(utc),
        }
    }

    fn utc_of_local(&self, local: i64) -> i64 {
        match self {
            TimeZone::Fixed(min) => local - min * 60,
            TimeZone::City(zone) => zone.utc_of_local(local),
        }
    }

    pub fn city(&self) -> Option<&'static str> {
        match self {
            TimeZone::Fixed(_) => None,
            TimeZone::City(zone) => Some(zone.name),
        }
    }

    fn standard_min(&self) -> i64 {
        match self {
            TimeZone::Fixed(min) => *min,
            TimeZone::City(zone) => i64::from(zone.standard_min),
        }
    }

    /// The next city east (`right`) or west.
    pub fn next(self, right: bool) -> Self {
        self.step(if right { 1 } else { -1 })
    }

    fn step(self, by: i64) -> Self {
        let at = match self {
            TimeZone::City(zone) => ZONES.iter().position(|z| z == zone),
            TimeZone::Fixed(_) => None,
        };
        let next = match at {
            Some(i) => (i as i64 + by).rem_euclid(ZONES.len() as i64) as usize,
            // From a bare offset, the first city past it in that direction.
            None if by > 0 => ZONES
                .iter()
                .position(|z| i64::from(z.standard_min) > self.standard_min())
                .unwrap_or(0),
            None => ZONES
                .iter()
                .rposition(|z| i64::from(z.standard_min) < self.standard_min())
                .unwrap_or(ZONES.len() - 1),
        };
        TimeZone::City(&ZONES[next])
    }
}

#[derive(Clone, Debug)]
pub struct ClockPicker {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    /// Picked in General, not here; the picker only turns what it shows into UTC.
    zone: TimeZone,
    h24: bool,
    cursor: usize,
}

impl ClockPicker {
    pub fn from_secs(secs: i64) -> Self {
        let (year, month, day) = civil_from_days(secs.div_euclid(DAY));
        let rem = secs.rem_euclid(DAY);
        ClockPicker {
            year: year.clamp(YEAR_MIN, YEAR_MAX),
            month,
            day,
            hour: rem / 3600,
            minute: rem / 60 % 60,
            zone: TimeZone::Fixed(0),
            h24: true,
            cursor: 0,
        }
    }

    /// Shows the hour on a 12-hour clock, with AM or PM after the minute, when `h24` is off.
    pub fn hours24(mut self, h24: bool) -> Self {
        self.h24 = h24;
        self
    }

    pub fn local(utc: i64, zone: TimeZone) -> Self {
        ClockPicker {
            zone,
            ..ClockPicker::from_secs(utc + zone.offset_at(utc) * 60)
        }
    }

    pub fn from_ymd(year: i64, month: i64, day: i64) -> Self {
        ClockPicker {
            year: year.clamp(YEAR_MIN, YEAR_MAX),
            month,
            day,
            hour: 0,
            minute: 0,
            zone: TimeZone::Fixed(0),
            h24: true,
            cursor: 0,
        }
    }

    fn wall(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * DAY + self.hour * 3600 + self.minute * 60
    }

    pub fn secs(&self) -> i64 {
        self.zone.utc_of_local(self.wall())
    }

    pub fn zone(&self) -> TimeZone {
        self.zone
    }

    /// The offset in force at the time shown.
    pub fn offset_min(&self) -> i64 {
        self.zone.offset_at(self.secs())
    }

    pub fn month(&self) -> i64 {
        self.month
    }

    pub fn day(&self) -> i64 {
        self.day
    }

    pub fn cursor(&self) -> Field {
        Field::ALL[self.cursor]
    }

    pub fn field(&mut self, field: Field) {
        self.cursor = Field::ALL.iter().position(|f| *f == field).unwrap_or(0);
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(Field::ALL.len() - 1);
    }

    pub fn up(&mut self) {
        self.step(1);
    }

    pub fn down(&mut self) {
        self.step(-1);
    }

    fn step(&mut self, by: i64) {
        match self.cursor() {
            Field::Year => self.year = wrap(self.year + by, YEAR_MIN, YEAR_MAX),
            Field::Month => self.month = wrap(self.month + by, 1, 12),
            Field::Day => self.day = wrap(self.day + by, 1, days_in_month(self.year, self.month)),
            Field::Hour => self.hour = wrap(self.hour + by, 0, 23),
            Field::Minute => self.minute = wrap(self.minute + by, 0, 59),
        }
        self.day = self.day.min(days_in_month(self.year, self.month));
    }

    pub fn text(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {}:{:02}{}",
            self.year,
            self.month,
            self.day,
            self.value_text(Field::Hour),
            self.minute,
            match self.h24 {
                true => String::new(),
                false => format!(" {}", if self.hour < 12 { "AM" } else { "PM" }),
            }
        )
    }

    /// One wheel's value as it reads: the month by name, the hour on a 12-hour clock.
    fn value_text(&self, field: Field) -> String {
        match field {
            Field::Year => format!("{:04}", self.year),
            Field::Month => MONTHS[(self.month - 1) as usize].into(),
            Field::Day => format!("{}", self.day),
            Field::Hour if self.h24 => format!("{:02}", self.hour),
            Field::Hour => format!("{}", (self.hour + 11) % 12 + 1),
            Field::Minute => format!("{:02}", self.minute),
        }
    }

    /// This wheel turned one step either way, for the faint values over and under it.
    fn turned(&self, field: Field, by: i64) -> String {
        let mut c = self.clone();
        c.field(field);
        c.step(by);
        c.value_text(field)
    }

    /// The page's wheels and their widths, AM or PM a wheel of its own on a 12-hour clock.
    fn wheels(&self) -> Vec<(Option<Field>, u32)> {
        if self.cursor().on_time_page() {
            let mut w = vec![
                (Some(Field::Hour), 120),
                (None, 40),
                (Some(Field::Minute), 120),
            ];
            if !self.h24 {
                w.push((None, 110));
            }
            w
        } else {
            vec![
                (Some(Field::Day), 110),
                (Some(Field::Month), 150),
                (Some(Field::Year), 170),
            ]
        }
    }

    fn wheel_x(&self) -> Vec<u32> {
        let wheels = self.wheels();
        let total: u32 = wheels.iter().map(|w| w.1).sum();
        let mut x = (WHEEL_W - total) / 2;
        wheels
            .iter()
            .map(|w| {
                let at = x;
                x += w.1;
                at
            })
            .collect()
    }

    /// What the other page holds, in a line under the wheels.
    fn under_text(&self) -> String {
        if self.cursor().on_time_page() {
            let days = days_from_civil(self.year, self.month, self.day);
            format!(
                "{} {} {} {}",
                WEEKDAYS[days.rem_euclid(7) as usize],
                self.day,
                MONTHS[(self.month - 1) as usize],
                self.year
            )
        } else {
            clock_text(self.hour * 3600 + self.minute * 60, self.h24)
        }
    }

    pub fn face(&self) -> UndoFace {
        let mut rgba = vec![0u8; (WHEEL_W * WHEEL_H * 4) as usize];
        let Some(font) = text::label_font() else {
            return UndoFace {
                rgba,
                w: WHEEL_W,
                h: WHEEL_H,
            };
        };
        let mut put = |text: &str, x: u32, w: u32, mid: u32, px: f32, alpha: u32| {
            let h = (px * 1.5) as u32;
            let mut buf = vec![0u8; (w * h * 4) as usize];
            let layout = text::fit(font, text, w as f32, 1, px, px * 0.5);
            text::draw_centred(&mut buf, w, h, &layout, INK);
            for a in buf.iter_mut().skip(3).step_by(4) {
                *a = (*a as u32 * alpha / 255) as u8;
            }
            blit(&mut rgba, WHEEL_W, &buf, w, h, x, mid - h / 2);
        };
        for ((field, w), x) in self.wheels().into_iter().zip(self.wheel_x()) {
            match field {
                Some(f) => {
                    put(&self.turned(f, -1), x, w, ROW_Y[0], SMALL_PX, FAINT);
                    put(&self.value_text(f), x, w, ROW_Y[1], BIG_PX, 255);
                    put(&self.turned(f, 1), x, w, ROW_Y[2], SMALL_PX, FAINT);
                }
                // The colon, or AM and PM, which turn with the hour.
                None if w == 40 => put(":", x, w, ROW_Y[1] - 4, BIG_PX, 255),
                None => {
                    let half = if self.hour < 12 { "AM" } else { "PM" };
                    put(half, x, w, ROW_Y[1], SMALL_PX, 255);
                }
            }
        }
        put(&self.under_text(), 0, WHEEL_W, UNDER_Y, UNDER_PX, 150);
        UndoFace {
            rgba,
            w: WHEEL_W,
            h: WHEEL_H,
        }
    }

    /// Drawn on the menu's own ground and in its ink, with the legend where the menu keeps it.
    pub fn draw(
        &self,
        canvas: Canvas,
        ground: Option<TexId>,
        ink: Ink,
        line: Option<TexId>,
        hint: Option<TexId>,
        back: Option<(TexId, u32)>,
        out: &mut Vec<Draw>,
    ) {
        draw_menu_ground(canvas, ground, ink, out);
        let w = WHEEL_W as f32;
        let x = canvas.centred(w).round();
        let y = ((LEGEND_Y - WHEEL_H as f32) / 2.0).round();
        if let Some(tex) = line {
            out.push(Draw::Tint {
                x,
                y,
                w,
                h: WHEEL_H as f32,
                tex,
                colour: ink.text,
            });
        }
        let at = self
            .wheels()
            .iter()
            .position(|(f, _)| *f == Some(self.cursor()))
            .unwrap_or(0);
        let (rx, rw) = (x + self.wheel_x()[at] as f32 + 8.0, self.wheels()[at].1 as f32 - 16.0);
        let ry = y + ROW_Y[1] as f32 - RING_H / 2.0;
        for (bx, by, bw, bh) in [
            (rx, ry, rw, RING_W),
            (rx, ry + RING_H - RING_W, rw, RING_W),
            (rx, ry, RING_W, RING_H),
            (rx + rw - RING_W, ry, RING_W, RING_H),
        ] {
            out.push(Draw::Rect {
                x: bx,
                y: by,
                w: bw,
                h: bh,
                colour: ink.text,
            });
        }
        let hw = hint_width(SET_CLOCK_KEY, SET_CLOCK_LABEL);
        let hints: Vec<(TexId, u32)> = back.into_iter().chain(hint.map(|t| (t, hw))).collect();
        for (tex, w, x) in centred_hints(canvas, &hints, LEGEND_GAP) {
            out.push(Draw::Tint {
                x: x.round(),
                y: LEGEND_Y,
                w: w as f32,
                h: HINT_H as f32,
                tex,
                colour: ink.text,
            });
        }
    }
}

pub fn set_clock_hint_face() -> UndoFace {
    crate::plate::tint_mask(crate::plate::hint_face(SET_CLOCK_KEY, SET_CLOCK_LABEL))
}

fn wrap(v: i64, lo: i64, hi: i64) -> i64 {
    (v - lo).rem_euclid(hi - lo + 1) + lo
}
