//! Notenprognose: "what do I need on the next exam(s) to reach grade X?"
//!
//! The subject average is an affine function of the value `x` given to the
//! planned assessments (all planned assessments get the same `x`), because
//! each category average is `(S + w·x) / (W + w)` with fixed sums and weights.
//! So the average at the scale's worst and best values determines the line,
//! and the required value follows by solving one linear equation.

use melde_core::{ValidationError, ensure_range};

use crate::average::{Assessment, Category, GradingConfig, subject_summary, summarize_unchecked};
use crate::scale::{EPSILON, GradeScale};

/// An upcoming assessment whose result is unknown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlannedAssessment {
    pub weight: f64,
    pub category: Category,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForecastOutcome {
    /// The target is reached even with the worst possible result.
    AlreadySecured,
    /// Each planned assessment needs `required` or better. For the point
    /// scale this is a whole number of points; for 1–6 it is the worst
    /// acceptable value rounded to two decimals in the student's disfavour.
    Reachable { required: f64 },
    /// Not reachable even with the best possible result.
    Unreachable { best_possible_average: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Forecast {
    pub outcome: ForecastOutcome,
    /// Average if every planned assessment gets the scale's worst value.
    pub worst_case_average: f64,
    /// Average if every planned assessment gets the scale's best value.
    pub best_case_average: f64,
}

/// Computes what the planned assessments need for the subject to round to
/// `target_grade` (a whole grade on the config's scale).
pub fn forecast(
    config: &GradingConfig,
    existing: &[Assessment],
    planned: &[PlannedAssessment],
    target_grade: f64,
) -> Result<Forecast, ValidationError> {
    // Validates the config and existing assessments.
    subject_summary(config, existing)?;
    let scale = config.scale;
    scale.validate(target_grade)?;
    if target_grade.fract() != 0.0 {
        return Err(ValidationError::Invalid(
            "target grade must be a whole grade".into(),
        ));
    }
    if planned.is_empty() {
        return Err(ValidationError::Invalid(
            "at least one planned assessment is required".into(),
        ));
    }
    for p in planned {
        ensure_range("planned weight", p.weight, 0.0, 1000.0)?;
        if p.weight == 0.0 {
            return Err(ValidationError::Invalid(
                "planned weight must be positive".into(),
            ));
        }
    }

    let average_with = |x: f64| -> f64 {
        let mut all: Vec<Assessment> = existing.to_vec();
        all.extend(planned.iter().map(|p| Assessment {
            value: x,
            weight: p.weight,
            category: p.category,
        }));
        summarize_unchecked(config, &all).exact.unwrap_or(f64::NAN)
    };

    let (worst, best) = (scale.worst(), scale.best());
    let worst_avg = average_with(worst);
    let best_avg = average_with(best);
    if !worst_avg.is_finite() || !best_avg.is_finite() {
        // Planned assessments only in a category whose weight is zero.
        return Err(ValidationError::Invalid(
            "planned assessments do not influence the average (category weight is 0)".into(),
        ));
    }

    let (limit, inclusive) = scale.threshold_for(target_grade, config.rounding);
    let meets = |avg: f64| {
        if inclusive {
            scale.is_at_least(avg, limit)
        } else {
            scale.is_better(avg, limit)
        }
    };

    let outcome = if meets(worst_avg) {
        ForecastOutcome::AlreadySecured
    } else if !meets(best_avg) {
        ForecastOutcome::Unreachable {
            best_possible_average: best_avg,
        }
    } else {
        let slope = (best_avg - worst_avg) / (best - worst);
        let exact = worst + (limit - worst_avg) / slope;
        ForecastOutcome::Reachable {
            required: worst_acceptable(scale, exact, &meets, &average_with),
        }
    };

    Ok(Forecast {
        outcome,
        worst_case_average: worst_avg,
        best_case_average: best_avg,
    })
}

/// Snaps the exact break-even value to the scale's practical grid, moving
/// toward "better" until the target is actually met.
fn worst_acceptable(
    scale: GradeScale,
    exact: f64,
    meets: &impl Fn(f64) -> bool,
    average_with: &impl Fn(f64) -> f64,
) -> f64 {
    let (step, mut candidate) = match scale {
        // Whole points; start at the first integer that is not worse than exact.
        GradeScale::Points => (1.0, (exact - EPSILON).ceil()),
        // Hundredths; start at the first value not worse than exact.
        GradeScale::Traditional => (-0.01, ((exact + EPSILON) * 100.0).floor() / 100.0),
    };
    let (lo, hi) = scale.bounds();
    candidate = candidate.clamp(lo, hi);
    // Bounded: at most 1 500 hundredth-steps before reaching the best grade.
    while !meets(average_with(candidate)) && scale.is_better(scale.best(), candidate) {
        candidate = (candidate + step).clamp(lo, hi);
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::RoundingRule;

    fn written(value: f64) -> Assessment {
        Assessment {
            value,
            weight: 1.0,
            category: Category::Written,
        }
    }

    fn next_exam() -> PlannedAssessment {
        PlannedAssessment {
            weight: 1.0,
            category: Category::Written,
        }
    }

    #[test]
    fn needs_specific_points_for_target() {
        // 9 and 10 points; what is needed for 11 (avg ≥ 10,5)? (9+10+x)/3 ≥ 10,5 → x ≥ 12,5 → 13
        let cfg = GradingConfig::new(GradeScale::Points, 1.0, 1.0);
        let f = forecast(&cfg, &[written(9.0), written(10.0)], &[next_exam()], 11.0).expect("ok");
        assert_eq!(f.outcome, ForecastOutcome::Reachable { required: 13.0 });
    }

    #[test]
    fn half_rule_changes_requirement() {
        // (2 + 3 + x)/3 ≤ 2,5 → x ≤ 2,5 with halves to the student.
        let mut cfg = GradingConfig::new(GradeScale::Traditional, 1.0, 1.0);
        let existing = [written(2.0), written(3.0)];
        let f = forecast(&cfg, &existing, &[next_exam()], 2.0).expect("ok");
        assert_eq!(f.outcome, ForecastOutcome::Reachable { required: 2.5 });

        // Halves go against the student: need strictly below 2,5.
        cfg.rounding = RoundingRule::HalfTowardWorse;
        let f = forecast(&cfg, &existing, &[next_exam()], 2.0).expect("ok");
        assert_eq!(f.outcome, ForecastOutcome::Reachable { required: 2.49 });
    }

    #[test]
    fn detects_unreachable_target() {
        let cfg = GradingConfig::new(GradeScale::Traditional, 1.0, 1.0);
        let existing = [written(5.0), written(5.0), written(5.0)];
        let f = forecast(&cfg, &existing, &[next_exam()], 1.0).expect("ok");
        assert!(
            matches!(f.outcome, ForecastOutcome::Unreachable { best_possible_average } if (best_possible_average - 4.0).abs() < 1e-9)
        );
    }

    #[test]
    fn detects_already_secured_target() {
        let cfg = GradingConfig::new(GradeScale::Points, 1.0, 1.0);
        let existing = [written(15.0), written(15.0), written(15.0), written(15.0)];
        // Worst case (15*4+0)/5 = 12 → rounds to 12, so target 10 is secured.
        let f = forecast(&cfg, &existing, &[next_exam()], 10.0).expect("ok");
        assert_eq!(f.outcome, ForecastOutcome::AlreadySecured);
    }

    #[test]
    fn oral_category_weights_are_respected() {
        // Written avg 3 (weight 1), next oral grade (weight 1): (3 + x)/2 ≤ 2,5 → x ≤ 2
        let cfg = GradingConfig::new(GradeScale::Traditional, 1.0, 1.0);
        let planned = [PlannedAssessment {
            weight: 1.0,
            category: Category::Oral,
        }];
        let f = forecast(&cfg, &[written(3.0)], &planned, 2.0).expect("ok");
        assert_eq!(f.outcome, ForecastOutcome::Reachable { required: 2.0 });
    }

    #[test]
    fn rejects_bad_input() {
        let cfg = GradingConfig::new(GradeScale::Points, 1.0, 1.0);
        assert!(forecast(&cfg, &[], &[], 10.0).is_err());
        assert!(forecast(&cfg, &[], &[next_exam()], 10.5).is_err());
        assert!(forecast(&cfg, &[], &[next_exam()], 16.0).is_err());
        let cfg = GradingConfig::new(GradeScale::Points, 1.0, 0.0);
        let oral = [PlannedAssessment {
            weight: 1.0,
            category: Category::Oral,
        }];
        assert!(forecast(&cfg, &[], &oral, 10.0).is_err());
    }

    #[test]
    fn no_existing_grades_needs_the_target_itself() {
        let cfg = GradingConfig::new(GradeScale::Points, 1.0, 1.0);
        let f = forecast(&cfg, &[], &[next_exam()], 11.0).expect("ok");
        // x ≥ 10,5 → 11 whole points.
        assert_eq!(f.outcome, ForecastOutcome::Reachable { required: 11.0 });
    }
}
