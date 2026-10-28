//! Stundenplan exports.

use melde_core::{Date, LocalDateTime, LocalTime};
use melde_scheduling as s;

use crate::{FfiDate, FfiDateTime, FfiWeekday, MeldeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiWeekParity {
    Every,
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiLesson {
    pub id: String,
    pub subject_id: String,
    pub weekday: FfiWeekday,
    pub start_minute: u16,
    pub end_minute: u16,
    pub parity: FfiWeekParity,
    pub room: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiHoliday {
    pub name: String,
    pub first_day: FfiDate,
    pub last_day: FfiDate,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FfiChangeKind {
    Cancelled,
    Substituted {
        subject_id: Option<String>,
        room: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiTimetableChange {
    pub lesson_id: String,
    pub date: FfiDate,
    pub kind: FfiChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct FfiTimetable {
    pub lessons: Vec<FfiLesson>,
    pub holidays: Vec<FfiHoliday>,
    pub changes: Vec<FfiTimetableChange>,
    pub week_a_reference: Option<FfiDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiOccurrence {
    pub lesson_id: String,
    pub subject_id: String,
    pub date: FfiDate,
    pub start_minute: u16,
    pub end_minute: u16,
    pub room: Option<String>,
    pub cancelled: bool,
    pub substituted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTimetableIssue {
    EmptyLesson { lesson_id: String },
    Overlap { first: String, second: String },
    InvertedHoliday { name: String },
    UnknownLesson { lesson_id: String },
}

impl TryFrom<FfiTimetable> for s::Timetable {
    type Error = MeldeError;

    fn try_from(t: FfiTimetable) -> Result<Self, Self::Error> {
        let lessons = t
            .lessons
            .into_iter()
            .map(|l| -> Result<s::Lesson, MeldeError> {
                Ok(s::Lesson {
                    id: l.id,
                    subject_id: l.subject_id,
                    weekday: l.weekday.into(),
                    start: LocalTime::from_minutes(l.start_minute)?,
                    end: LocalTime::from_minutes(l.end_minute)?,
                    parity: match l.parity {
                        FfiWeekParity::Every => s::WeekParity::Every,
                        FfiWeekParity::A => s::WeekParity::A,
                        FfiWeekParity::B => s::WeekParity::B,
                    },
                    room: l.room,
                })
            })
            .collect::<Result<_, _>>()?;
        let holidays = t
            .holidays
            .into_iter()
            .map(|h| -> Result<s::Holiday, MeldeError> {
                Ok(s::Holiday {
                    name: h.name,
                    first_day: h.first_day.try_into()?,
                    last_day: h.last_day.try_into()?,
                })
            })
            .collect::<Result<_, _>>()?;
        let changes = t
            .changes
            .into_iter()
            .map(|c| -> Result<s::TimetableChange, MeldeError> {
                Ok(s::TimetableChange {
                    lesson_id: c.lesson_id,
                    date: c.date.try_into()?,
                    kind: match c.kind {
                        FfiChangeKind::Cancelled => s::ChangeKind::Cancelled,
                        FfiChangeKind::Substituted { subject_id, room } => {
                            s::ChangeKind::Substituted { subject_id, room }
                        }
                    },
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(s::Timetable {
            lessons,
            holidays,
            changes,
            week_a_reference: t.week_a_reference.map(Date::try_from).transpose()?,
        })
    }
}

impl From<s::Occurrence> for FfiOccurrence {
    fn from(o: s::Occurrence) -> Self {
        FfiOccurrence {
            lesson_id: o.lesson_id,
            subject_id: o.subject_id,
            date: o.date.into(),
            start_minute: o.start.minutes(),
            end_minute: o.end.minutes(),
            room: o.room,
            cancelled: o.cancelled,
            substituted: o.substituted,
        }
    }
}

#[uniffi::export]
pub fn timetable_validate(timetable: FfiTimetable) -> Result<Vec<FfiTimetableIssue>, MeldeError> {
    let tt: s::Timetable = timetable.try_into()?;
    Ok(tt
        .validate()
        .into_iter()
        .map(|i| match i {
            s::Issue::EmptyLesson { lesson_id } => FfiTimetableIssue::EmptyLesson { lesson_id },
            s::Issue::Overlap { first, second } => FfiTimetableIssue::Overlap { first, second },
            s::Issue::InvertedHoliday { name } => FfiTimetableIssue::InvertedHoliday { name },
            s::Issue::UnknownLesson { lesson_id } => FfiTimetableIssue::UnknownLesson { lesson_id },
        })
        .collect())
}

#[uniffi::export]
pub fn timetable_occurrences_on(
    timetable: FfiTimetable,
    date: FfiDate,
) -> Result<Vec<FfiOccurrence>, MeldeError> {
    let tt: s::Timetable = timetable.try_into()?;
    Ok(tt
        .occurrences_on(date.try_into()?)
        .into_iter()
        .map(Into::into)
        .collect())
}

#[uniffi::export]
pub fn timetable_current(
    timetable: FfiTimetable,
    at: FfiDateTime,
) -> Result<Option<FfiOccurrence>, MeldeError> {
    let tt: s::Timetable = timetable.try_into()?;
    Ok(tt.current(LocalDateTime::try_from(at)?).map(Into::into))
}

#[uniffi::export]
pub fn timetable_next(
    timetable: FfiTimetable,
    at: FfiDateTime,
    horizon_days: u32,
) -> Result<Option<FfiOccurrence>, MeldeError> {
    let tt: s::Timetable = timetable.try_into()?;
    Ok(tt
        .next(LocalDateTime::try_from(at)?, horizon_days)
        .map(Into::into))
}

/// The lesson a Meldung at `at` should be attached to, if it can be determined.
#[uniffi::export]
pub fn timetable_participation_lesson(
    timetable: FfiTimetable,
    at: FfiDateTime,
    grace_minutes: u16,
) -> Result<Option<FfiOccurrence>, MeldeError> {
    let tt: s::Timetable = timetable.try_into()?;
    Ok(tt
        .subject_for_participation(LocalDateTime::try_from(at)?, grace_minutes)
        .map(Into::into))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monday() -> FfiDate {
        FfiDate {
            year: 2026,
            month: 10,
            day: 19,
        }
    }

    fn mathe() -> FfiLesson {
        FfiLesson {
            id: "l1".into(),
            subject_id: "mathe".into(),
            weekday: FfiWeekday::Monday,
            start_minute: 8 * 60,
            end_minute: 8 * 60 + 45,
            parity: FfiWeekParity::Every,
            room: Some("A1".into()),
        }
    }

    #[test]
    fn current_lesson_round_trip() {
        let tt = FfiTimetable {
            lessons: vec![mathe()],
            ..Default::default()
        };
        let at = FfiDateTime {
            date: monday(),
            minute_of_day: 8 * 60 + 10,
        };
        let current = timetable_current(tt, at).expect("valid").expect("lesson");
        assert_eq!(current.subject_id, "mathe");
        assert_eq!(current.date, monday());
        assert_eq!(current.room.as_deref(), Some("A1"));
    }

    #[test]
    fn invalid_minutes_are_rejected() {
        let mut lesson = mathe();
        lesson.end_minute = 2000;
        let tt = FfiTimetable {
            lessons: vec![lesson],
            ..Default::default()
        };
        assert!(timetable_validate(tt).is_err());
    }
}
