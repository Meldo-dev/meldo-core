//! Stundenplan: a repeating weekly schedule with optional A/B weeks,
//! holidays and per-date changes (Vertretung / Ausfall).

use chrono::Days;
use melde_core::{Date, LocalDateTime, LocalTime, Weekday};

/// Which weeks a lesson takes place in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WeekParity {
    #[default]
    Every,
    /// Only in A weeks (requires [`Timetable::week_a_reference`]).
    A,
    /// Only in B weeks.
    B,
}

/// A recurring slot in the weekly timetable.
#[derive(Debug, Clone, PartialEq)]
pub struct Lesson {
    pub id: String,
    pub subject_id: String,
    pub weekday: Weekday,
    pub start: LocalTime,
    pub end: LocalTime,
    pub parity: WeekParity,
    pub room: Option<String>,
}

/// Holidays (Ferien) or single days off, both dates inclusive.
#[derive(Debug, Clone, PartialEq)]
pub struct Holiday {
    pub name: String,
    pub first_day: Date,
    pub last_day: Date,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Cancelled,
    /// Vertretung: another subject and/or room replaces the lesson.
    Substituted {
        subject_id: Option<String>,
        room: Option<String>,
    },
}

/// A change to one lesson on one date.
#[derive(Debug, Clone, PartialEq)]
pub struct TimetableChange {
    pub lesson_id: String,
    pub date: Date,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timetable {
    pub lessons: Vec<Lesson>,
    pub holidays: Vec<Holiday>,
    pub changes: Vec<TimetableChange>,
    /// Any date inside a week that counts as an A week. Without it, A/B
    /// lessons are treated as weekly.
    pub week_a_reference: Option<Date>,
}

/// A lesson on a concrete date, with changes applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Occurrence {
    pub lesson_id: String,
    pub subject_id: String,
    pub date: Date,
    pub start: LocalTime,
    pub end: LocalTime,
    pub room: Option<String>,
    pub cancelled: bool,
    pub substituted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    /// End time is not after start time.
    EmptyLesson { lesson_id: String },
    /// Two lessons overlap on the same weekday in at least one week.
    Overlap { first: String, second: String },
    /// Holiday ends before it starts.
    InvertedHoliday { name: String },
    /// A change refers to a lesson id that does not exist.
    UnknownLesson { lesson_id: String },
}

impl Timetable {
    /// Lists every consistency problem; an empty list means the timetable is valid.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        for l in &self.lessons {
            if l.end <= l.start {
                issues.push(Issue::EmptyLesson {
                    lesson_id: l.id.clone(),
                });
            }
        }
        for (i, a) in self.lessons.iter().enumerate() {
            for b in &self.lessons[i + 1..] {
                let same_weeks = parities_overlap(a.parity, b.parity);
                if a.weekday == b.weekday && same_weeks && a.start < b.end && b.start < a.end {
                    issues.push(Issue::Overlap {
                        first: a.id.clone(),
                        second: b.id.clone(),
                    });
                }
            }
        }
        for h in &self.holidays {
            if h.last_day < h.first_day {
                issues.push(Issue::InvertedHoliday {
                    name: h.name.clone(),
                });
            }
        }
        for c in &self.changes {
            if !self.lessons.iter().any(|l| l.id == c.lesson_id) {
                issues.push(Issue::UnknownLesson {
                    lesson_id: c.lesson_id.clone(),
                });
            }
        }
        issues
    }

    pub fn is_holiday(&self, date: Date) -> bool {
        self.holidays
            .iter()
            .any(|h| h.first_day <= date && date <= h.last_day)
    }

    /// `Some(true)` for A weeks, `Some(false)` for B weeks, `None` without a reference.
    pub fn is_week_a(&self, date: Date) -> Option<bool> {
        let reference = self.week_a_reference?;
        let weeks = (monday_of(date) - monday_of(reference))
            .num_days()
            .div_euclid(7);
        Some(weeks.rem_euclid(2) == 0)
    }

    /// All lessons on `date` in start order, with changes applied. Holidays have none.
    pub fn occurrences_on(&self, date: Date) -> Vec<Occurrence> {
        if self.is_holiday(date) {
            return Vec::new();
        }
        let weekday = Weekday::of(date);
        let week_a = self.is_week_a(date);
        let mut result: Vec<Occurrence> = self
            .lessons
            .iter()
            .filter(|l| l.weekday == weekday && l.end > l.start)
            .filter(|l| match (l.parity, week_a) {
                (WeekParity::Every, _) | (_, None) => true,
                (WeekParity::A, Some(a)) => a,
                (WeekParity::B, Some(a)) => !a,
            })
            .map(|l| self.apply_changes(l, date))
            .collect();
        result.sort_by_key(|o| (o.start, o.end));
        result
    }

    fn apply_changes(&self, lesson: &Lesson, date: Date) -> Occurrence {
        let mut o = Occurrence {
            lesson_id: lesson.id.clone(),
            subject_id: lesson.subject_id.clone(),
            date,
            start: lesson.start,
            end: lesson.end,
            room: lesson.room.clone(),
            cancelled: false,
            substituted: false,
        };
        for change in self
            .changes
            .iter()
            .filter(|c| c.lesson_id == lesson.id && c.date == date)
        {
            match &change.kind {
                ChangeKind::Cancelled => o.cancelled = true,
                ChangeKind::Substituted { subject_id, room } => {
                    o.substituted = true;
                    if let Some(s) = subject_id {
                        o.subject_id = s.clone();
                    }
                    if room.is_some() {
                        o.room = room.clone();
                    }
                }
            }
        }
        o
    }

    /// The lesson taking place at `at` (start inclusive, end exclusive), if not cancelled.
    pub fn current(&self, at: LocalDateTime) -> Option<Occurrence> {
        self.occurrences_on(at.date)
            .into_iter()
            .find(|o| !o.cancelled && o.start <= at.time && at.time < o.end)
    }

    /// The next non-cancelled lesson starting after `at`, looking up to `horizon_days` ahead.
    pub fn next(&self, at: LocalDateTime, horizon_days: u32) -> Option<Occurrence> {
        (0..=u64::from(horizon_days))
            .filter_map(|offset| at.date.checked_add_days(Days::new(offset)))
            .flat_map(|date| self.occurrences_on(date))
            .find(|o| !o.cancelled && (o.date > at.date || o.start > at.time))
    }

    /// The subject a Meldung recorded at `at` belongs to: the current lesson,
    /// or one that ended at most `grace_minutes` ago (students often tap
    /// right after the bell). Manual selection is needed when this is `None`.
    pub fn subject_for_participation(
        &self,
        at: LocalDateTime,
        grace_minutes: u16,
    ) -> Option<Occurrence> {
        if let Some(current) = self.current(at) {
            return Some(current);
        }
        let now = at.time.minutes();
        self.occurrences_on(at.date)
            .into_iter()
            .filter(|o| !o.cancelled && o.end.minutes() <= now)
            .filter(|o| now - o.end.minutes() <= grace_minutes)
            .max_by_key(|o| o.end)
    }
}

fn parities_overlap(a: WeekParity, b: WeekParity) -> bool {
    !matches!(
        (a, b),
        (WeekParity::A, WeekParity::B) | (WeekParity::B, WeekParity::A)
    )
}

fn monday_of(date: Date) -> Date {
    date - Days::new(u64::from(Weekday::of(date).index()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn t(h: u16, m: u16) -> LocalTime {
        LocalTime::from_hm(h, m).expect("valid time")
    }

    fn lesson(
        id: &str,
        subject: &str,
        weekday: Weekday,
        start: LocalTime,
        end: LocalTime,
    ) -> Lesson {
        Lesson {
            id: id.into(),
            subject_id: subject.into(),
            weekday,
            start,
            end,
            parity: WeekParity::Every,
            room: None,
        }
    }

    /// Monday 2026-10-19: Mathe 08:00–08:45, Deutsch 08:50–09:35.
    fn sample() -> Timetable {
        Timetable {
            lessons: vec![
                lesson("l2", "deutsch", Weekday::Monday, t(8, 50), t(9, 35)),
                lesson("l1", "mathe", Weekday::Monday, t(8, 0), t(8, 45)),
                lesson("l3", "englisch", Weekday::Tuesday, t(8, 0), t(8, 45)),
            ],
            ..Timetable::default()
        }
    }

    const MONDAY: (i32, u32, u32) = (2026, 10, 19);

    fn monday_at(h: u16, m: u16) -> LocalDateTime {
        LocalDateTime::new(d(MONDAY.0, MONDAY.1, MONDAY.2), t(h, m))
    }

    #[test]
    fn occurrences_are_sorted() {
        let o = sample().occurrences_on(d(2026, 10, 19));
        let ids: Vec<_> = o.iter().map(|o| o.lesson_id.as_str()).collect();
        assert_eq!(ids, ["l1", "l2"]);
    }

    #[test]
    fn current_and_next_lesson() {
        let tt = sample();
        assert_eq!(
            tt.current(monday_at(8, 10)).map(|o| o.subject_id),
            Some("mathe".into())
        );
        // End is exclusive: 08:45 is the break.
        assert_eq!(tt.current(monday_at(8, 45)), None);
        assert_eq!(
            tt.next(monday_at(8, 45), 7).map(|o| o.subject_id),
            Some("deutsch".into())
        );
        // After Monday's last lesson, the next one is Tuesday.
        let next = tt.next(monday_at(10, 0), 7).expect("tuesday lesson");
        assert_eq!(
            (next.subject_id.as_str(), next.date),
            ("englisch", d(2026, 10, 20))
        );
    }

    #[test]
    fn participation_uses_grace_period_after_bell() {
        let tt = sample();
        let at = monday_at(8, 47);
        assert_eq!(
            tt.subject_for_participation(at, 5).map(|o| o.subject_id),
            Some("mathe".into())
        );
        assert_eq!(tt.subject_for_participation(at, 1), None);
    }

    #[test]
    fn holidays_remove_lessons() {
        let mut tt = sample();
        tt.holidays.push(Holiday {
            name: "Herbstferien".into(),
            first_day: d(2026, 10, 19),
            last_day: d(2026, 10, 30),
        });
        assert!(tt.occurrences_on(d(2026, 10, 19)).is_empty());
        let next = tt.next(monday_at(7, 0), 21).expect("lesson after holidays");
        assert_eq!(next.date, d(2026, 11, 2));
    }

    #[test]
    fn cancelled_and_substituted_lessons() {
        let mut tt = sample();
        tt.changes.push(TimetableChange {
            lesson_id: "l1".into(),
            date: d(2026, 10, 19),
            kind: ChangeKind::Cancelled,
        });
        tt.changes.push(TimetableChange {
            lesson_id: "l2".into(),
            date: d(2026, 10, 19),
            kind: ChangeKind::Substituted {
                subject_id: Some("bio".into()),
                room: Some("B12".into()),
            },
        });
        assert_eq!(tt.current(monday_at(8, 10)), None);
        let sub = tt.current(monday_at(9, 0)).expect("substitution");
        assert!(sub.substituted);
        assert_eq!(
            (sub.subject_id.as_str(), sub.room.as_deref()),
            ("bio", Some("B12"))
        );
        assert_eq!(
            tt.next(monday_at(7, 0), 0).map(|o| o.lesson_id),
            Some("l2".into())
        );
    }

    #[test]
    fn ab_weeks_alternate() {
        let mut tt = Timetable {
            week_a_reference: Some(d(2026, 10, 21)), // Wednesday of an A week
            ..Timetable::default()
        };
        let mut a = lesson("a", "physik", Weekday::Monday, t(10, 0), t(10, 45));
        a.parity = WeekParity::A;
        let mut b = lesson("b", "chemie", Weekday::Monday, t(10, 0), t(10, 45));
        b.parity = WeekParity::B;
        tt.lessons = vec![a, b];
        assert!(tt.validate().is_empty(), "A and B lessons may share a slot");
        let subject_on = |date| tt.occurrences_on(date)[0].subject_id.clone();
        assert_eq!(subject_on(d(2026, 10, 19)), "physik");
        assert_eq!(subject_on(d(2026, 10, 26)), "chemie");
        assert_eq!(subject_on(d(2026, 10, 12)), "chemie");
    }

    #[test]
    fn validation_finds_problems() {
        let mut tt = sample();
        tt.lessons
            .push(lesson("x", "kunst", Weekday::Monday, t(8, 30), t(9, 0)));
        tt.lessons
            .push(lesson("y", "musik", Weekday::Friday, t(9, 0), t(9, 0)));
        tt.changes.push(TimetableChange {
            lesson_id: "missing".into(),
            date: d(2026, 10, 19),
            kind: ChangeKind::Cancelled,
        });
        let issues = tt.validate();
        assert!(issues.contains(&Issue::EmptyLesson {
            lesson_id: "y".into()
        }));
        assert!(issues.contains(&Issue::Overlap {
            first: "l2".into(),
            second: "x".into()
        }));
        assert!(issues.contains(&Issue::Overlap {
            first: "l1".into(),
            second: "x".into()
        }));
        assert!(issues.contains(&Issue::UnknownLesson {
            lesson_id: "missing".into()
        }));
        assert!(sample().validate().is_empty());
    }
}
