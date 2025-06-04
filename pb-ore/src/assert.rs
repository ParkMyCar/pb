//! Utilities for `assert!`s.

/// Asserts that the provided expression, that returns an `Option`, is `None`.
#[macro_export]
macro_rules! assert_none {
    ($val:expr, $($msg:tt)+) => {{
        if let Some(y) = &$val {
            panic!("assertion failed: expected None found Some({y:?}), {}", format!($($msg)+));
        }
    }};
    ($val:expr) => {{
        if let Some(y) = &$val {
            panic!("assertion failed: expected None found Some({y:?})");
        }
    }}
}
