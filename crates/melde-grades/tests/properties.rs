//! Property tests: averages stay on the scale, and forecasts are self-consistent.

use melde_grades::{
    Assessment, Category, ForecastOutcome, GradeScale, GradingConfig, PlannedAssessment,
    RoundingRule, forecast, subject_summary,
};
use proptest::prelude::*;

fn scale_strategy() -> impl Strategy<Value = GradeScale> {
    prop_oneof![Just(GradeScale::Traditional), Just(GradeScale::Points)]
}

fn rounding_strategy() -> impl Strategy<Value = RoundingRule> {
    prop_oneof![
        Just(RoundingRule::HalfTowardBetter),
        Just(RoundingRule::HalfTowardWorse)
    ]
}

fn category_strategy() -> impl Strategy<Value = Category> {
    prop_oneof![Just(Category::Written), Just(Category::Oral)]
}

/// Grade values on the scale's practical grid: whole points, or quarter steps on 1–6.
fn value_for(scale: GradeScale, raw: u8) -> f64 {
    match scale {
        GradeScale::Points => f64::from(raw % 16),
        GradeScale::Traditional => 1.0 + f64::from(raw % 21) * 0.25,
    }
}

fn config_and_assessments() -> impl Strategy<Value = (GradingConfig, Vec<Assessment>)> {
    (
        scale_strategy(),
        rounding_strategy(),
        1u8..=4,
        1u8..=4,
        prop::collection::vec((any::<u8>(), 1u8..=3, category_strategy()), 0..12),
    )
        .prop_map(|(scale, rounding, ww, ow, raw)| {
            let mut cfg = GradingConfig::new(scale, f64::from(ww), f64::from(ow));
            cfg.rounding = rounding;
            let assessments = raw
                .into_iter()
                .map(|(v, w, category)| Assessment {
                    value: value_for(scale, v),
                    weight: f64::from(w),
                    category,
                })
                .collect();
            (cfg, assessments)
        })
}

proptest! {
    #[test]
    fn average_lies_within_scale((cfg, assessments) in config_and_assessments()) {
        let s = subject_summary(&cfg, &assessments).expect("valid input");
        let (lo, hi) = cfg.scale.bounds();
        if let Some(exact) = s.exact {
            prop_assert!(exact >= lo - 1e-9 && exact <= hi + 1e-9);
        } else {
            prop_assert!(assessments.is_empty());
        }
        if let Some(rounded) = s.rounded {
            prop_assert!(rounded.fract().abs() < f64::EPSILON, "{} is not whole", rounded);
        }
    }

    #[test]
    fn forecast_requirement_actually_reaches_target(
        (cfg, existing) in config_and_assessments(),
        planned_count in 1usize..=3,
        planned_category in category_strategy(),
        target_raw in any::<u8>(),
    ) {
        let target = match cfg.scale {
            GradeScale::Points => f64::from(target_raw % 16),
            GradeScale::Traditional => f64::from(1 + target_raw % 6),
        };
        let planned = vec![PlannedAssessment { weight: 1.0, category: planned_category }; planned_count];
        let f = match forecast(&cfg, &existing, &planned, target) {
            Ok(f) => f,
            // Planned category with zero weight is a legitimate rejection.
            Err(_) => return Ok(()),
        };

        let rounded_with = |x: f64| {
            let mut all = existing.clone();
            all.extend(planned.iter().map(|p| Assessment { value: x, weight: p.weight, category: p.category }));
            subject_summary(&cfg, &all).expect("valid").rounded.expect("non-empty")
        };
        let reaches = |rounded: f64| cfg.scale.is_at_least(rounded, target);

        match f.outcome {
            ForecastOutcome::AlreadySecured => prop_assert!(reaches(rounded_with(cfg.scale.worst()))),
            ForecastOutcome::Unreachable { .. } => prop_assert!(!reaches(rounded_with(cfg.scale.best()))),
            ForecastOutcome::Reachable { required } => {
                prop_assert!(reaches(rounded_with(required)), "required {} does not reach {}", required, target);
                // One grid step worse must no longer reach the target.
                let worse = match cfg.scale {
                    GradeScale::Points => required - 1.0,
                    GradeScale::Traditional => required + 0.01,
                };
                let (lo, hi) = cfg.scale.bounds();
                if worse >= lo && worse <= hi {
                    prop_assert!(!reaches(rounded_with(worse)), "{} would also reach {}", worse, target);
                }
            }
        }
    }
}
