use crate::platform::darwin::path::DarwinFilename;
use crate::platform::darwin::DarwinPath;
use crate::platform::{OpenOptions, Platform, PlatformFilename, PlatformPath};

use super::DarwinPlatform;

#[test]
fn smoketest_xattr() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("test-xattr");

    let path = DarwinPath::try_new(path.to_string_lossy().to_string()).unwrap();
    let file = DarwinPlatform::open(path, OpenOptions::CREATE).unwrap();

    let xattr_name = DarwinFilename::try_new("com.pb.test".to_string()).unwrap();
    let xattr_value = b"123456789";

    // Write the xattr.
    DarwinPlatform::fsetxattr(file, xattr_name.clone(), b"123456789").unwrap();
    // Fsync to ensure the data is flushed to disk.
    DarwinPlatform::fsync(file).unwrap();
    // Read back the xattr value.
    let mut buf = vec![0u8; 10];
    let bytes_read = DarwinPlatform::fgetxattr(file, xattr_name, &mut buf[..]).unwrap();

    assert_eq!(bytes_read, 9);
    assert_eq!(&buf[..9], &xattr_value[..]);
}

#[test]
fn smoketest_getpath() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp
        .path()
        .join("test-getpath")
        .to_string_lossy()
        .to_string();

    let path = DarwinPath::try_new(path).unwrap();
    let file = DarwinPlatform::open(path.clone(), OpenOptions::CREATE).unwrap();
    let rnd_path = DarwinPlatform::fgetpath(file).unwrap();

    let is_suffix = rnd_path
        .into_inner()
        .as_str()
        .strip_suffix(&path.into_inner())
        .is_some();
    assert!(is_suffix);
}
