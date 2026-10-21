use slot_gfx::{Canvas, Draw, TexId, OUT_H};
use slot_store::{
    civil_from_days, days_from_civil, days_in_month, parse_stamp, zone_named, Zone, ZONES,
};

use crate::plate::{blit, centred_hints, hint_width, UndoFace, HINT_H, LEGEND_GAP};
use crate::text;

const DAY: i64 = 86_400;

const YEAR_MIN: i64 = 2000;
const YEAR_MAX: i64 = 2099;

const INK: [u8; 3] = [0xf6, 0xf4, 0xef];
const BACKDROP: [f32; 4] = [0.06, 0.06, 0.07, 1.0];

const CELLS: [u32; 11] = [88, 22, 52, 22, 52, 32, 52, 22, 52, 28, 196];
const FIELD_CELL: [usize; 6] = [0, 2, 4, 6, 8, 10];

const PICKER_H: u32 = 44;
const PICKER_PX: f32 = 30.0;
const PICKER_MIN_PX: f32 = 12.0;
const CARET_H: f32 = 4.0;
const CARET_GAP: f32 = 6.0;
const HINT_DROP: f32 = 48.0;

const SET_CLOCK_KEY: &str = "A";
const SET_CLOCK_LABEL: &str = "set the clock";

pub fn clock_label(stamp: &str) -> String {
    parse_stamp(stamp).map(hhmm).unwrap_or_default()
}

pub fn hhmm(secs: i64) -> String {
    let rem = secs.rem_euclid(DAY);
    format!("{:02}:{:02}", rem / 3600, rem / 60 % 60)
}

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

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
    Zone,
}

impl Field {
    pub const ALL: [Field; 6] = [
        Field::Year,
        Field::Month,
        Field::Day,
        Field::Hour,
        Field::Minute,
        Field::Zone,
    ];
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
    zone: TimeZone,
    /// Until the date or time is touched, a new zone keeps the moment and
    /// moves the time shown, so a clock set over Wi-Fi stays right.
    typed: bool,
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
            typed: false,
            cursor: 0,
        }
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
            typed: false,
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
        if self.cursor() != Field::Zone {
            self.typed = true;
        }
        match self.cursor() {
            Field::Year => self.year = wrap(self.year + by, YEAR_MIN, YEAR_MAX),
            Field::Month => self.month = wrap(self.month + by, 1, 12),
            Field::Day => self.day = wrap(self.day + by, 1, days_in_month(self.year, self.month)),
            Field::Hour => self.hour = wrap(self.hour + by, 0, 23),
            Field::Minute => self.minute = wrap(self.minute + by, 0, 59),
            Field::Zone => {
                let utc = self.secs();
                self.zone = self.zone.step(by);
                if !self.typed {
                    let keep = ClockPicker::local(utc, self.zone);
                    (self.year, self.month, self.day) = (keep.year, keep.month, keep.day);
                    (self.hour, self.minute) = (keep.hour, keep.minute);
                }
            }
        }
        self.day = self.day.min(days_in_month(self.year, self.month));
    }

    pub fn text(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02} {}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.zone_text()
        )
    }

    pub fn face(&self) -> UndoFace {
        let w = picker_w();
        let mut rgba = vec![0u8; (w * PICKER_H * 4) as usize];
        for (i, cell) in CELLS.iter().enumerate() {
            let text = self.cell_text(i);
            if text.is_empty() {
                continue;
            }
            let mut buf = vec![0u8; (cell * PICKER_H * 4) as usize];
            if let Some(font) = text::label_font() {
                let layout = text::fit(font, &text, *cell as f32, 1, PICKER_PX, PICKER_MIN_PX);
                text::draw_centred(&mut buf, *cell, PICKER_H, &layout, INK);
            }
            blit(&mut rgba, w, &buf, *cell, PICKER_H, cell_x(i), 0);
        }
        UndoFace {
            rgba,
            w,
            h: PICKER_H,
        }
    }

    fn cell_text(&self, cell: usize) -> String {
        match cell {
            0 => format!("{:04}", self.year),
            1 | 3 => "-".into(),
            2 => format!("{:02}", self.month),
            4 => format!("{:02}", self.day),
            6 => format!("{:02}", self.hour),
            7 => ":".into(),
            8 => format!("{:02}", self.minute),
            10 => self.zone_text(),
            _ => String::new(),
        }
    }

    fn zone_text(&self) -> String {
        if let TimeZone::City(zone) = self.zone {
            return zone.name.to_string();
        }
        let offset = self.offset_min();
        let sign = if offset < 0 { '-' } else { '+' };
        let mins = offset.abs();
        format!("UTC{sign}{:02}:{:02}", mins / 60, mins % 60)
    }

    pub fn draw(
        &self,
        canvas: Canvas,
        line: Option<TexId>,
        hint: Option<TexId>,
        back: Option<(TexId, u32)>,
        out: &mut Vec<Draw>,
    ) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: canvas.w,
            h: OUT_H as f32,
            colour: BACKDROP,
        });
        let w = picker_w() as f32;
        let x = canvas.centred(w);
        let y = (OUT_H as f32 - PICKER_H as f32) / 2.0 - HINT_DROP / 2.0;
        if let Some(tex) = line {
            out.push(Draw::Tex {
                x,
                y,
                w,
                h: PICKER_H as f32,
                tex,
                alpha: 1.0,
            });
        }
        let cell = FIELD_CELL[self.cursor];
        out.push(Draw::Rect {
            x: x + cell_x(cell) as f32,
            y: y + PICKER_H as f32 + CARET_GAP,
            w: CELLS[cell] as f32,
            h: CARET_H,
            colour: [
                INK[0] as f32 / 255.0,
                INK[1] as f32 / 255.0,
                INK[2] as f32 / 255.0,
                1.0,
            ],
        });
        let hint_y = y + PICKER_H as f32 + HINT_DROP;
        let hw = hint_width(SET_CLOCK_KEY, SET_CLOCK_LABEL);
        let (Some(back), Some(tex)) = (back, hint) else {
            if let Some(tex) = hint {
                out.push(Draw::Tex {
                    x: canvas.centred(hw as f32),
                    y: hint_y,
                    w: hw as f32,
                    h: HINT_H as f32,
                    tex,
                    alpha: 1.0,
                });
            }
            return;
        };
        for (tex, w, x) in centred_hints(canvas, &[back, (tex, hw)], LEGEND_GAP) {
            out.push(Draw::Tex {
                x,
                y: hint_y,
                w: w as f32,
                h: HINT_H as f32,
                tex,
                alpha: 1.0,
            });
        }
    }
}

pub fn set_clock_hint_face() -> UndoFace {
    crate::plate::hint_face(SET_CLOCK_KEY, SET_CLOCK_LABEL)
}

fn picker_w() -> u32 {
    CELLS.iter().sum()
}

fn cell_x(cell: usize) -> u32 {
    CELLS[..cell].iter().sum()
}

fn wrap(v: i64, lo: i64, hi: i64) -> i64 {
    (v - lo).rem_euclid(hi - lo + 1) + lo
}
