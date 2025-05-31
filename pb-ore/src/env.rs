//! Utilities for reading environment variables.

use std::ffi::OsStr;

/// Returns true if the environmentd variable is set, and is _not_ one of the following:
/// `'0', '', 'no', 'false'`.
pub fn is_truthy<K: AsRef<OsStr>>(var: K) -> bool {
    static CANDIDATES: &[&str] = &["0", "", "no", "false"];

    // Return early if the value is not set.
    let Some(mut value) = std::env::var_os(var) else {
        return false;
    };

    // Check if our value matches any of our "falsey" candidates.
    OsStr::make_ascii_lowercase(&mut value);
    let is_falsey = CANDIDATES.iter().any(|falsey| value == *falsey);

    !is_falsey
}
