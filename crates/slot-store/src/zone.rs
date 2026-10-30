//! Time zones that follow their own summer time, so a clock set once stays
//! right through the year. Rules are kept in the device's own code because
//! the handheld has no zone database.

use crate::stamp::{civil_from_days, days_from_civil, days_in_month};

const DAY: i64 = 86_400;
const HOUR: i64 = 3_600;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Summer {
    None,
    /// Last Sunday of March to last Sunday of October, at 01:00 UTC.
    Europe,
    /// Second Sunday of March to first Sunday of November, at 02:00 local.
    America,
    /// First Sunday of October to first Sunday of April, at 02:00 standard.
    Australia,
    /// Last Sunday of September to first Sunday of April, at 02:00 standard.
    NewZealand,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Zone {
    pub name: &'static str,
    /// Winter offset from UTC, in minutes.
    pub standard_min: i16,
    pub summer: Summer,
}

const fn zone(name: &'static str, standard_min: i16, summer: Summer) -> Zone {
    Zone {
        name,
        standard_min,
        summer,
    }
}

/// West to east, so up and down on the picker walk the globe.
pub const ZONES: [Zone; 28] = [
    zone("Honolulu", -600, Summer::None),
    zone("Anchorage", -540, Summer::America),
    zone("Los Angeles", -480, Summer::America),
    zone("Denver", -420, Summer::America),
    zone("Chicago", -360, Summer::America),
    zone("Mexico City", -360, Summer::None),
    zone("New York", -300, Summer::America),
    zone("Halifax", -240, Summer::America),
    zone("Sao Paulo", -180, Summer::None),
    zone("UTC", 0, Summer::None),
    zone("London", 0, Summer::Europe),
    zone("Lisbon", 0, Summer::Europe),
    zone("Amsterdam", 60, Summer::Europe),
    zone("Berlin", 60, Summer::Europe),
    zone("Paris", 60, Summer::Europe),
    zone("Athens", 120, Summer::Europe),
    zone("Istanbul", 180, Summer::None),
    zone("Moscow", 180, Summer::None),
    zone("Dubai", 240, Summer::None),
    zone("Karachi", 300, Summer::None),
    zone("Delhi", 330, Summer::None),
    zone("Bangkok", 420, Summer::None),
    zone("Beijing", 480, Summer::None),
    zone("Singapore", 480, Summer::None),
    zone("Tokyo", 540, Summer::None),
    zone("Brisbane", 600, Summer::None),
    zone("Sydney", 600, Summer::Australia),
    zone("Auckland", 720, Summer::NewZealand),
];

pub fn zone_named(name: &str) -> Option<&'static Zone> {
    ZONES.iter().find(|z| z.name.eq_ignore_ascii_case(name.trim()))
}

impl Zone {
    /// Offset from UTC at this moment, in minutes.
    pub fn offset_at(&self, utc: i64) -> i64 {
        let standard = i64::from(self.standard_min);
        let summer = match self.summer {
            Summer::None => false,
            Summer::Europe => {
                let year = year_of(utc);
                let start = last_sunday(year, 3) * DAY + HOUR;
                let end = last_sunday(year, 10) * DAY + HOUR;
                (start..end).contains(&utc)
            }
            Summer::America => {
                let year = year_of(utc + standard * 60);
                let start = nth_sunday(year, 3, 2) * DAY + 2 * HOUR - standard * 60;
                let end = nth_sunday(year, 11, 1) * DAY + 2 * HOUR - (standard + 60) * 60;
                (start..end).contains(&utc)
            }
            Summer::Australia => {
                let year = year_of(utc + standard * 60);
                let end = nth_sunday(year, 4, 1) * DAY + 2 * HOUR - standard * 60;
                let start = nth_sunday(year, 10, 1) * DAY + 2 * HOUR - standard * 60;
                utc < end || utc >= start
            }
            Summer::NewZealand => {
                let year = year_of(utc + standard * 60);
                let end = nth_sunday(year, 4, 1) * DAY + 2 * HOUR - standard * 60;
                let start = last_sunday(year, 9) * DAY + 2 * HOUR - standard * 60;
                utc < end || utc >= start
            }
        };
        standard + if summer { 60 } else { 0 }
    }

    /// The UTC moment a local wall time stands for. In the hour that repeats
    /// when summer time ends, the summer reading wins.
    pub fn utc_of_local(&self, local: i64) -> i64 {
        let summer = local - (i64::from(self.standard_min) + 60) * 60;
        if self.offset_at(summer) > i64::from(self.standard_min) {
            summer
        } else {
            local - i64::from(self.standard_min) * 60
        }
    }
}

fn year_of(secs: i64) -> i64 {
    civil_from_days(secs.div_euclid(DAY)).0
}

/// 0 is Sunday; 1970-01-01 was a Thursday.
fn weekday(days: i64) -> i64 {
    (days + 4).rem_euclid(7)
}

fn nth_sunday(year: i64, month: i64, n: i64) -> i64 {
    let first = days_from_civil(year, month, 1);
    first + (7 - weekday(first)) % 7 + (n - 1) * 7
}

fn last_sunday(year: i64, month: i64) -> i64 {
    let last = days_from_civil(year, month, days_in_month(year, month));
    last - weekday(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stamp::parse_stamp;

    fn at(stamp: &str) -> i64 {
        parse_stamp(stamp).expect("a stamp")
    }

    #[test]
    fn amsterdam_is_two_hours_ahead_in_october_and_one_after_the_change() {
        let ams = zone_named("amsterdam").unwrap();
        assert_eq!(ams.offset_at(at("2026-10-21_18-22-00")), 120);
        assert_eq!(ams.offset_at(at("2026-10-25_00-59-59")), 120);
        assert_eq!(ams.offset_at(at("2026-10-25_01-00-00")), 60);
        assert_eq!(ams.offset_at(at("2027-03-28_00-59-59")), 60);
        assert_eq!(ams.offset_at(at("2027-03-28_01-00-00")), 120);
    }

    #[test]
    fn new_york_changes_at_two_in_the_morning_local() {
        let ny = zone_named("New York").unwrap();
        assert_eq!(ny.offset_at(at("2026-03-08_06-59-59")), -300);
        assert_eq!(ny.offset_at(at("2026-03-08_07-00-00")), -240);
        assert_eq!(ny.offset_at(at("2026-11-01_05-59-59")), -240);
        assert_eq!(ny.offset_at(at("2026-11-01_06-00-00")), -300);
    }

    #[test]
    fn sydney_has_summer_over_new_year() {
        let syd = zone_named("Sydney").unwrap();
        assert_eq!(syd.offset_at(at("2026-01-10_00-00-00")), 660);
        assert_eq!(syd.offset_at(at("2026-07-10_00-00-00")), 600);
        assert_eq!(syd.offset_at(at("2026-10-03_15-59-59")), 600);
        assert_eq!(syd.offset_at(at("2026-10-03_16-00-00")), 660);
    }

    #[test]
    fn a_local_time_turns_back_into_the_same_utc() {
        let ams = zone_named("Amsterdam").unwrap();
        let utc = at("2026-10-21_18-22-00");
        assert_eq!(ams.utc_of_local(utc + 120 * 60), utc);
        let winter = at("2026-12-01_12-00-00");
        assert_eq!(ams.utc_of_local(winter + 60 * 60), winter);
    }
}
