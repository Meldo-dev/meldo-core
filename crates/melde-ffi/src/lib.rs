//! UniFFI surface of the Melde Planner core.
//!
//! Types here are plain records mirroring the domain crates, so the domain
//! stays free of FFI concerns. Calendar values cross the boundary as plain
//! numbers; Swift converts `Date` to local components before calling in.

uniffi::setup_scaffolding!();

mod grades;
mod participation;
mod tasks;
mod timetable;
mod untis;

use melde_core::{Date, LocalDateTime, LocalTime, ValidationError, Weekday};

/// Every error crossing the FFI boundary. Messages are developer-facing;
/// the app maps them to localized UI text. The field is `detail`, not
/// `message`, because Kotlin exceptions already have a `message` property.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MeldeError {
    #[error("invalid input: {detail}")]
    InvalidInput { detail: String },
    /// WebUntis answered with an error (e.g. code -8504 = wrong user name or password).
    #[error("WebUntis error {code}: {detail}")]
    UntisServer { code: i64, detail: String },
    /// The response was not what WebUntis normally sends (school blocks access, maintenance page).
    #[error("unexpected WebUntis response: {detail}")]
    UntisMalformed { detail: String },
}

impl From<melde_untis::UntisError> for MeldeError {
    fn from(e: melde_untis::UntisError) -> Self {
        match e {
            melde_untis::UntisError::Server { code, message } => MeldeError::UntisServer {
                code,
                detail: message,
            },
            melde_untis::UntisError::Malformed(message) => {
                MeldeError::UntisMalformed { detail: message }
            }
        }
    }
}

impl From<ValidationError> for MeldeError {
    fn from(e: ValidationError) -> Self {
        MeldeError::InvalidInput {
            detail: e.to_string(),
        }
    }
}

fn invalid(message: impl Into<String>) -> MeldeError {
    MeldeError::InvalidInput {
        detail: message.into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiDateTime {
    pub date: FfiDate,
    /// Minutes since local midnight.
    pub minute_of_day: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiWeekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl TryFrom<FfiDate> for Date {
    type Error = MeldeError;

    fn try_from(d: FfiDate) -> Result<Self, Self::Error> {
        Date::from_ymd_opt(d.year, u32::from(d.month), u32::from(d.day)).ok_or_else(|| {
            invalid(format!(
                "{}-{}-{} is not a valid date",
                d.year, d.month, d.day
            ))
        })
    }
}

impl From<Date> for FfiDate {
    fn from(d: Date) -> Self {
        use chrono::Datelike;
        FfiDate {
            year: d.year(),
            // chrono guarantees month 1..=12 and day 1..=31.
            month: d.month() as u8,
            day: d.day() as u8,
        }
    }
}

impl TryFrom<FfiDateTime> for LocalDateTime {
    type Error = MeldeError;

    fn try_from(dt: FfiDateTime) -> Result<Self, Self::Error> {
        Ok(LocalDateTime::new(
            dt.date.try_into()?,
            LocalTime::from_minutes(dt.minute_of_day)?,
        ))
    }
}

impl From<Weekday> for FfiWeekday {
    fn from(w: Weekday) -> Self {
        match w {
            Weekday::Monday => FfiWeekday::Monday,
            Weekday::Tuesday => FfiWeekday::Tuesday,
            Weekday::Wednesday => FfiWeekday::Wednesday,
            Weekday::Thursday => FfiWeekday::Thursday,
            Weekday::Friday => FfiWeekday::Friday,
            Weekday::Saturday => FfiWeekday::Saturday,
            Weekday::Sunday => FfiWeekday::Sunday,
        }
    }
}

impl From<FfiWeekday> for Weekday {
    fn from(w: FfiWeekday) -> Self {
        match w {
            FfiWeekday::Monday => Weekday::Monday,
            FfiWeekday::Tuesday => Weekday::Tuesday,
            FfiWeekday::Wednesday => Weekday::Wednesday,
            FfiWeekday::Thursday => Weekday::Thursday,
            FfiWeekday::Friday => Weekday::Friday,
            FfiWeekday::Saturday => Weekday::Saturday,
            FfiWeekday::Sunday => Weekday::Sunday,
        }
    }
}

/// Version of the Rust core, shown in the app's About screen.
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_round_trip() {
        let ffi = FfiDate {
            year: 2026,
            month: 2,
            day: 28,
        };
        let date: Date = ffi.try_into().expect("valid");
        assert_eq!(FfiDate::from(date), ffi);
    }

    #[test]
    fn rejects_impossible_dates_and_times() {
        let bad = FfiDate {
            year: 2026,
            month: 2,
            day: 30,
        };
        assert!(Date::try_from(bad).is_err());
        let dt = FfiDateTime {
            date: FfiDate {
                year: 2026,
                month: 1,
                day: 1,
            },
            minute_of_day: 1440,
        };
        assert!(LocalDateTime::try_from(dt).is_err());
    }
}
