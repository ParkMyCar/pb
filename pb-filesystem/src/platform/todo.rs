//! Placeholder Platform that uses `todo!(...)` for all implementations.

use crate::platform::Platform;

pub struct TodoPlatform;

impl Platform for TodoPlatform {
    fn stat(_path: String) -> Result<crate::FileMetadata, crate::Error> {
        todo!("stat")
    }
}
