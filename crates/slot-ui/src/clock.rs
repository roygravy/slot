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

/// The last cell holds AM or PM on a 12-hour clock and is gone on a 24-hour one.
const CELLS: [u32; 11] = [88, 22, 52, 22, 52, 32, 52, 22, 52, 14, 64];
const FIELD_CELL: [usize; 5] = [0, 2, 4, 6, 8];

const PICKER_H: u32 = 44;
const PICKER_PX: f32 = 30.0;
const PICKER_MIN_PX: f32 = 12.0;
const CARET_H: f32 = 4.0;
const CARET_GAP: f32 = 6.0;

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
    pub const ALL: [Field; 5] = [
        Field::Year,
        Field::Month,
        Field::Day,
        Field::Hour,
        Field::Minute,
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
            self.cell_text(6),
            self.minute,
            match self.h24 {
                true => String::new(),
                false => format!(" {}", self.cell_text(10)),
            }
        )
    }

    pub fn face(&self) -> UndoFace {
        let w = self.picker_w();
        let mut rgba = vec![0u8; (w * PICKER_H * 4) as usize];
        for (i, cell) in self.cells().iter().enumerate() {
            let text = self.cell_text(i);
            if text.is_empty() {
                continue;
            }
            // A 12-hour hour of one digit sits against its colon, as 6:00 does.
            let (cell, x) = match (i, self.h24, text::label_font()) {
                (6, false, Some(font)) => {
                    let tight = (text::line_width(font, &text, PICKER_PX, 0.0).ceil() as u32 + 4).min(*cell);
                    (tight, self.cell_x(i) + cell - tight)
                }
                _ => (*cell, self.cell_x(i)),
            };
            let mut buf = vec![0u8; (cell * PICKER_H * 4) as usize];
            if let Some(font) = text::label_font() {
                let layout = text::fit(font, &text, cell as f32, 1, PICKER_PX, PICKER_MIN_PX);
                text::draw_centred(&mut buf, cell, PICKER_H, &layout, INK);
            }
            blit(&mut rgba, w, &buf, cell, PICKER_H, x, 0);
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
            6 if self.h24 => format!("{:02}", self.hour),
            6 => format!("{}", (self.hour + 11) % 12 + 1),
            7 => ":".into(),
            8 => format!("{:02}", self.minute),
            10 if !self.h24 => if self.hour < 12 { "AM" } else { "PM" }.into(),
            _ => String::new(),
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
        let w = self.picker_w() as f32;
        let x = canvas.centred(w).round();
        let y = ((LEGEND_Y - PICKER_H as f32) / 2.0).round();
        if let Some(tex) = line {
            out.push(Draw::Tint {
                x,
                y,
                w,
                h: PICKER_H as f32,
                tex,
                colour: ink.text,
            });
        }
        let cell = FIELD_CELL[self.cursor];
        out.push(Draw::Rect {
            x: x + self.cell_x(cell) as f32,
            y: y + PICKER_H as f32 + CARET_GAP,
            w: self.cells()[cell] as f32,
            h: CARET_H,
            colour: ink.text,
        });
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

impl ClockPicker {
    fn cells(&self) -> [u32; 11] {
        let mut cells = CELLS;
        if self.h24 {
            (cells[9], cells[10]) = (0, 0);
        }
        cells
    }

    fn picker_w(&self) -> u32 {
        self.cells().iter().sum()
    }

    fn cell_x(&self, cell: usize) -> u32 {
        self.cells()[..cell].iter().sum()
    }
}

fn wrap(v: i64, lo: i64, hi: i64) -> i64 {
    (v - lo).rem_euclid(hi - lo + 1) + lo
}
