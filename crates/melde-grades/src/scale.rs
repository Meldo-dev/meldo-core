//! Grade scales used in German schools.
//!
//! Direction matters: on the traditional 1–6 scale *lower* is better, on the
//! Oberstufe 0–15 point scale *higher* is better. Every comparison goes
//! through [`GradeScale`] so callers never compare raw numbers.

use melde_core::{ValidationError, ensure_range};

/// Tolerance used when comparing averages against rounding boundaries.
pub(crate) const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GradeScale {
    /// Noten 1 (sehr gut) … 6 (ungenügend). Lower is better.
    Traditional,
    /// Notenpunkte 15 … 0 (Oberstufe). Higher is better.
    Points,
}

/// How an exact average is turned into a whole grade when it lies exactly
/// halfway (2,5 or 10,5). Schools differ, so this is configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RoundingRule {
    /// 2,5 → 2 and 10,5 → 11: the half goes to the student's advantage.
    #[default]
    HalfTowardBetter,
    /// 2,5 → 3 and 10,5 → 10.
    HalfTowardWorse,
}

impl GradeScale {
    pub fn best(self) -> f64 {
        match self {
            GradeScale::Traditional => 1.0,
            GradeScale::Points => 15.0,
        }
    }

    pub fn worst(self) -> f64 {
        match self {
            GradeScale::Traditional => 6.0,
            GradeScale::Points => 0.0,
        }
    }

    /// Validates a single grade value on this scale.
    pub fn validate(self, value: f64) -> Result<f64, ValidationError> {
        let (lo, hi) = self.bounds();
        ensure_range("grade", value, lo, hi)
    }

    /// `(min, max)` numeric bounds regardless of direction.
    pub fn bounds(self) -> (f64, f64) {
        let (a, b) = (self.best(), self.worst());
        (a.min(b), a.max(b))
    }

    /// `true` when `a` is strictly better than `b`.
    pub fn is_better(self, a: f64, b: f64) -> bool {
        match self {
            GradeScale::Traditional => a < b - EPSILON,
            GradeScale::Points => a > b + EPSILON,
        }
    }

    /// `true` when `a` is at least as good as `b`.
    pub fn is_at_least(self, a: f64, b: f64) -> bool {
        !self.is_better(b, a)
    }

    /// Rounds an exact average to a whole grade using `rule`.
    pub fn round(self, average: f64, rule: RoundingRule) -> f64 {
        let floor = average.floor();
        let frac = average - floor;
        let rounded = if (frac - 0.5).abs() <= EPSILON {
            // Exactly halfway: pick the better or worse neighbour.
            let (lower, upper) = (floor, floor + 1.0);
            let (better, worse) = if self.is_better(lower, upper) {
                (lower, upper)
            } else {
                (upper, lower)
            };
            match rule {
                RoundingRule::HalfTowardBetter => better,
                RoundingRule::HalfTowardWorse => worse,
            }
        } else {
            average.round()
        };
        let (lo, hi) = self.bounds();
        rounded.clamp(lo, hi)
    }

    /// The exact-average limit that still rounds to `target` (a whole grade).
    ///
    /// Returned as `(limit, inclusive)`: an average reaches the target when it
    /// is better than `limit`, or equal to it if `inclusive`.
    pub fn threshold_for(self, target: f64, rule: RoundingRule) -> (f64, bool) {
        let inclusive = rule == RoundingRule::HalfTowardBetter;
        match self {
            // Target 2 → averages up to 2,5 (inclusive if halves go to the student).
            GradeScale::Traditional => (target + 0.5, inclusive),
            // Target 11 → averages from 10,5.
            GradeScale::Points => (target - 0.5, inclusive),
        }
    }
}

/// Tendency marker on a traditional grade: 2+, 2, 2-.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tendency {
    Plus,
    None,
    Minus,
}

/// Converts Oberstufe points to the traditional grade with tendency,
/// following the KMK table (15/14/13 → 1+/1/1-, …, 0 → 6).
pub fn points_to_traditional(points: u8) -> Result<(u8, Tendency), ValidationError> {
    if points > 15 {
        return Err(ValidationError::OutOfRange {
            field: "points",
            min: 0.0,
            max: 15.0,
            value: f64::from(points),
        });
    }
    if points == 0 {
        return Ok((6, Tendency::None));
    }
    let grade = 5 - (points - 1) / 3;
    let tendency = match (points - 1) % 3 {
        2 => Tendency::Plus,
        1 => Tendency::None,
        _ => Tendency::Minus,
    };
    Ok((grade, tendency))
}

/// Converts a traditional grade with tendency to Oberstufe points.
pub fn traditional_to_points(grade: u8, tendency: Tendency) -> Result<u8, ValidationError> {
    if !(1..=6).contains(&grade) {
        return Err(ValidationError::OutOfRange {
            field: "grade",
            min: 1.0,
            max: 6.0,
            value: f64::from(grade),
        });
    }
    if grade == 6 {
        return match tendency {
            Tendency::Plus | Tendency::None => Ok(0),
            Tendency::Minus => Err(ValidationError::Invalid("6- does not exist".into())),
        };
    }
    let base = 17 - 3 * grade;
    Ok(match tendency {
        Tendency::Plus => base + 1,
        Tendency::None => base,
        Tendency::Minus => base - 1,
    })
}

/// Numeric value of a traditional grade with tendency, e.g. 2- with step 0,25 → 2,25.
/// The step differs between schools (0,25 / 0,3 / 0,33), so it is a parameter.
/// 1+ counts as 1,0 because the scale's best value is 1.
pub fn traditional_value(grade: u8, tendency: Tendency, step: f64) -> Result<f64, ValidationError> {
    ensure_range("tendency step", step, 0.0, 0.5)?;
    if grade == 6 && tendency == Tendency::Minus {
        return Err(ValidationError::Invalid("6- does not exist".into()));
    }
    let base = f64::from(grade);
    let value = match tendency {
        Tendency::Plus => (base - step).max(GradeScale::Traditional.best()),
        Tendency::None => base,
        Tendency::Minus => base + step,
    };
    GradeScale::Traditional.validate(value)
}

/// The grade with tendency closest to an exact average on the 1–6 scale,
/// e.g. 2,25 → 2- with step 0,25 or 0,33. This is how a subject's grade reads;
/// it is still an estimate, not the grade the teacher assigns.
///
/// Two tendencies with the same value (1+ and 1, or 2- and 3+ with step 0,5)
/// resolve to the plain grade, else to the better one. An average exactly
/// between two values follows `rule`.
pub fn nearest_traditional(
    average: f64,
    step: f64,
    rule: RoundingRule,
) -> Result<(u8, Tendency), ValidationError> {
    GradeScale::Traditional.validate(average)?;
    ensure_range("tendency step", step, 0.0, 0.5)?;
    let mut best: Option<((u8, Tendency), f64)> = None;
    // Candidates in order from best (1+) to worst (6): on equal distance the
    // earlier one is the better grade.
    for grade in 1..=6u8 {
        for tendency in [Tendency::Plus, Tendency::None, Tendency::Minus] {
            let Ok(value) = traditional_value(grade, tendency, step) else {
                continue;
            };
            let candidate = ((grade, tendency), value);
            let Some((current, current_value)) = best else {
                best = Some(candidate);
                continue;
            };
            let (distance, current_distance) =
                ((value - average).abs(), (current_value - average).abs());
            let replace = if distance < current_distance - EPSILON {
                true
            } else if distance > current_distance + EPSILON {
                false
            } else if (value - current_value).abs() <= EPSILON {
                // Same value written two ways: the plain grade reads more naturally.
                tendency == Tendency::None && current.1 != Tendency::None
            } else {
                // Exactly halfway between two values: the later candidate is worse.
                rule == RoundingRule::HalfTowardWorse
            };
            if replace {
                best = Some(candidate);
            }
        }
    }
    best.map(|(grade, _)| grade)
        .ok_or_else(|| ValidationError::Invalid("no grade for this average".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_is_respected() {
        assert!(GradeScale::Traditional.is_better(1.0, 2.0));
        assert!(GradeScale::Points.is_better(12.0, 11.0));
        assert!(!GradeScale::Points.is_better(11.0, 11.0));
        assert!(GradeScale::Points.is_at_least(11.0, 11.0));
    }

    #[test]
    fn rounding_halves_follow_rule() {
        use RoundingRule::*;
        let t = GradeScale::Traditional;
        assert_eq!(t.round(2.5, HalfTowardBetter), 2.0);
        assert_eq!(t.round(2.5, HalfTowardWorse), 3.0);
        assert_eq!(t.round(2.49, HalfTowardWorse), 2.0);
        assert_eq!(t.round(2.51, HalfTowardBetter), 3.0);
        let p = GradeScale::Points;
        assert_eq!(p.round(10.5, HalfTowardBetter), 11.0);
        assert_eq!(p.round(10.5, HalfTowardWorse), 10.0);
    }

    #[test]
    fn kmk_point_table_round_trips() {
        assert_eq!(points_to_traditional(15), Ok((1, Tendency::Plus)));
        assert_eq!(points_to_traditional(14), Ok((1, Tendency::None)));
        assert_eq!(points_to_traditional(13), Ok((1, Tendency::Minus)));
        assert_eq!(points_to_traditional(5), Ok((4, Tendency::None)));
        assert_eq!(points_to_traditional(1), Ok((5, Tendency::Minus)));
        assert_eq!(points_to_traditional(0), Ok((6, Tendency::None)));
        for p in 0..=15u8 {
            let (g, t) = points_to_traditional(p).expect("valid points");
            assert_eq!(traditional_to_points(g, t), Ok(p), "points {p}");
        }
        assert!(points_to_traditional(16).is_err());
    }

    #[test]
    fn tendency_values() {
        assert_eq!(traditional_value(2, Tendency::Minus, 0.25), Ok(2.25));
        assert_eq!(traditional_value(2, Tendency::Plus, 0.3), Ok(1.7));
        assert_eq!(traditional_value(1, Tendency::Plus, 0.25), Ok(1.0));
        assert!(traditional_value(6, Tendency::Minus, 0.25).is_err());
    }

    #[test]
    fn nearest_grade_with_tendency() {
        use RoundingRule::*;
        assert_eq!(
            nearest_traditional(2.25, 0.25, HalfTowardBetter),
            Ok((2, Tendency::Minus))
        );
        assert_eq!(
            nearest_traditional(2.2, 0.33, HalfTowardBetter),
            Ok((2, Tendency::Minus))
        );
        assert_eq!(
            nearest_traditional(1.8, 0.33, HalfTowardBetter),
            Ok((2, Tendency::Plus))
        );
        assert_eq!(
            nearest_traditional(2.05, 0.33, HalfTowardBetter),
            Ok((2, Tendency::None))
        );
        assert_eq!(
            nearest_traditional(5.9, 0.25, HalfTowardBetter),
            Ok((6, Tendency::None))
        );
        // 1+ and 1 are both 1,0: the plain grade.
        assert_eq!(
            nearest_traditional(1.0, 0.25, HalfTowardBetter),
            Ok((1, Tendency::None))
        );
        // Halfway between 2 (2,0) and 2- (2,25) follows the rounding rule.
        assert_eq!(
            nearest_traditional(2.125, 0.25, HalfTowardBetter),
            Ok((2, Tendency::None))
        );
        assert_eq!(
            nearest_traditional(2.125, 0.25, HalfTowardWorse),
            Ok((2, Tendency::Minus))
        );
        // With step 0,5, 2- and 3+ are both 2,5: the better one.
        assert_eq!(
            nearest_traditional(2.5, 0.5, HalfTowardWorse),
            Ok((2, Tendency::Minus))
        );
        assert!(nearest_traditional(0.5, 0.25, HalfTowardBetter).is_err());
        assert!(nearest_traditional(2.0, 0.6, HalfTowardBetter).is_err());
    }
}
