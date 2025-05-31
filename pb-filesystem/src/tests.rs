use std::env::temp_dir;

use pb_ore::iter::LendingIterator;

use crate::filesystem::Filesystem;

impl Filesystem {
    fn new_test() -> Filesystem {
        Filesystem::new(2, 32)
    }
}

#[tokio::test]
async fn smoketest_writing() {
    let temp = temp_dir();
    let path = temp.join("test-writing.txt").to_string_lossy().to_string();

    let filesystem = Filesystem::new_test();
    let (mut handle, _stat) = filesystem.open(path).as_file().with_create().await.unwrap();

    let content = "hello world I am writing a file".as_bytes().to_vec();
    handle.write(content.clone(), 0).await.unwrap();

    let data = handle
        .read_with(|mut iterator| {
            let mut buf = Vec::new();
            while let Some(result) = iterator.next() {
                let bytes = result?;
                buf.extend_from_slice(&bytes[..]);
            }
            Ok(buf)
        })
        .await
        .unwrap();
    assert_eq!(data, content);
}

#[tokio::test]
async fn smoketest_mkdir() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("mydir").to_string_lossy().to_string();

    let filesystem = Filesystem::new_test();
    let handle = filesystem
        .open(path)
        .as_directory()
        .with_create()
        .await
        .unwrap();
    let (mut child, _stat) = handle
        .openat("test-file.txt".to_string())
        .as_file()
        .with_create()
        .await
        .unwrap();

    let content = b"i am some data that will get written to disk".to_vec();
    child.write(content.clone(), 0).await.unwrap();

    let rnd_content = child
        .read_with(|mut iterator| {
            let mut buf = Vec::new();
            while let Some(result) = iterator.next() {
                let bytes = result?;
                buf.extend_from_slice(&bytes[..]);
            }
            Ok(buf)
        })
        .await
        .unwrap();

    assert_eq!(content, rnd_content);
}

#[tokio::test]
async fn smoketest_tree() {
    let mut temp = tempfile::TempDir::new().unwrap();
    temp.disable_cleanup(true);
    println!("{:?}", temp.path());
    let path = temp.path().join("tree_1").to_string_lossy().to_string();

    let filesystem = Filesystem::new_test();
    let handle = filesystem
        .open(path)
        .as_directory()
        .with_create()
        .await
        .unwrap();
    let _ = handle
        .openat("test-file.txt".to_string())
        .as_file()
        .with_create()
        .await
        .unwrap();
    let _ = handle
        .openat("nested_dir".to_string())
        .as_directory()
        .with_create()
        .await
        .unwrap();

    let tree = handle.tree().await.unwrap();
    println!("{tree}")
}
