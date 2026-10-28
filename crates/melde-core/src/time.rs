use chrono::{Datelike, NaiveDate};

use crate::ValidationError;

/// Day of the week, Monday first (German school week convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    pub fn of(date: NaiveDate) -> Weekday {
        Self::ALL[date.weekday().num_days_from_monday() as usize]
    }

    /// 0 for Monday … 6 for Sunday.
    pub fn index(self) -> u32 {
        self as u32
    }
}

/// Minutes since local midnight, `0..1440`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalTime(u16);

impl LocalTime {
    pub const MINUTES_PER_DAY: u16 = 24 * 60;

    pub fn from_minutes(minutes: u16) -> Result<Self, ValidationError> {
        if minutes < Self::MINUTES_PER_DAY {
            Ok(Self(minutes))
        } else {
            Err(ValidationError::Invalid(format!(
                "time must be below 24:00, got {minutes} minutes"
            )))
        }
    }

    pub fn from_hm(hour: u16, minute: u16) -> Result<Self, ValidationError> {
        if minute >= 60 {
            return Err(ValidationError::Invalid(format!(
                "minute must be below 60, got {minute}"
            )));
        }
        Self::from_minutes(hour.saturating_mul(60).saturating_add(minute))
    }

    pub fn minutes(self) -> u16 {
        self.0
    }
}

/// A local date plus wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDateTime {
    pub date: NaiveDate,
    pub time: LocalTime,
}

impl LocalDateTime {
    pub fn new(date: NaiveDate, time: LocalTime) -> Self {
        Self { date, time }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_of_known_dates() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 19).expect("valid date");
        assert_eq!(Weekday::of(monday), Weekday::Monday);
        assert_eq!(Weekday::of(monday + chrono::Days::new(6)), Weekday::Sunday);
    }

    #[test]
    fn local_time_bounds() {
        assert_eq!(LocalTime::from_hm(7, 55).map(LocalTime::minutes), Ok(475));
        assert!(LocalTime::from_hm(24, 0).is_err());
        assert!(LocalTime::from_hm(8, 60).is_err());
    }
}
