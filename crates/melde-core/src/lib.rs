//! Shared domain primitives used by every Melde Planner crate.
//!
//! All calendar values are *local wall-clock* values supplied by the client.
//! The core never reads the system clock or a timezone database, which keeps
//! every calculation deterministic and testable.

mod time;

pub use time::{LocalDateTime, LocalTime, Weekday};

pub use chrono::NaiveDate as Date;

/// Validation failure for user-supplied data.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("{field} must be between {min} and {max}, got {value}")]
    OutOfRange {
        field: &'static str,
        min: f64,
        max: f64,
        value: f64,
    },
    #[error("{field} must be a finite number")]
    NotFinite { field: &'static str },
    #[error("{0}")]
    Invalid(String),
}

/// Rejects NaN and infinities, which would silently poison averages.
pub fn ensure_finite(field: &'static str, value: f64) -> Result<f64, ValidationError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ValidationError::NotFinite { field })
    }
}

/// Validates that `value` lies in the closed interval `[min, max]`.
pub fn ensure_range(
    field: &'static str,
    value: f64,
    min: f64,
    max: f64,
) -> Result<f64, ValidationError> {
    ensure_finite(field, value)?;
    if value < min || value > max {
        Err(ValidationError::OutOfRange {
            field,
            min,
            max,
            value,
        })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nan_and_infinity() {
        assert!(ensure_finite("x", f64::NAN).is_err());
        assert!(ensure_finite("x", f64::INFINITY).is_err());
        assert_eq!(ensure_finite("x", 2.5), Ok(2.5));
    }

    #[test]
    fn range_is_inclusive() {
        assert!(ensure_range("g", 1.0, 1.0, 6.0).is_ok());
        assert!(ensure_range("g", 6.0, 1.0, 6.0).is_ok());
        assert!(ensure_range("g", 6.01, 1.0, 6.0).is_err());
    }
}
