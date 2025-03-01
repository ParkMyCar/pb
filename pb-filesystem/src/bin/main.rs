#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use pb_filesystem::filesystem::Filesystem;
use pb_filesystem::platform::{FilesystemPlatform, Platform};
use pb_filesystem::FileType;
use pb_ore::iter::LendingIterator;

use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let result = FilesystemPlatform::file_handle_max();
    println!("{result:?}");

    let filesystem = Filesystem::new_tokio(tokio::runtime::Handle::current(), 1024);

    let root = filesystem
        .open("/Users/parker/Development".to_string())
        .as_directory()
        .await
        .unwrap();
    let filename = "rust".to_string();

    let mut directories2 = VecDeque::new();
    directories2.push_front((Arc::new(root), filename));

    let (tx, rx) = futures::channel::mpsc::unbounded();
    let count = Arc::new(AtomicUsize::new(0));

    while let Some((parent, filename)) = directories2.pop_front() {
        let processed = count.load(std::sync::atomic::Ordering::Relaxed);
        if processed % 1000 == 0 {
            println!("processed {processed}");
        }

        let directory = parent.openat(filename).as_directory().await.unwrap();
        // Drop the parent as soon as possible to re-claim it's handle.
        drop(parent);

        let directory = Arc::new(directory);
        let entries = directory.list().await.unwrap();

        for entry in entries {
            if entry.name.inner == "." || entry.name.inner == ".." {
                continue;
            }

            match entry.kind {
                FileType::Directory => {
                    let directory_ = Arc::clone(&directory);
                    directories2.push_front((directory_, entry.name.inner));
                }
                FileType::File => {
                    if !entry.name.inner.ends_with(".rs") {
                        continue;
                    }

                    let directory_ = Arc::clone(&directory);
                    let tx_ = tx.clone();
                    let count_ = Arc::clone(&count);
                    tokio::task::spawn(async move {
                        let (file, _stat) =
                            directory_.openat(entry.name.inner).as_file().await.unwrap();
                        drop(directory_);

                        let mut buf: Vec<u8> = Vec::new();
                        let _result = file
                            .read_with(move |mut bytes| {
                                while let Some(byte_result) = bytes.next() {
                                    let bytes = byte_result?;
                                    buf.extend(bytes);
                                }
                                Ok(buf)
                            })
                            .await;
                        count_.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        file.close().await.expect("failed to close file");
                        tx_.unbounded_send(()).expect("receiver went away");
                    });
                }
                FileType::Symlink => (),
            }
        }
    }
    drop(tx);

    let results: Vec<_> = rx.collect().await;
    println!("saw files {}", results.len());
}

#[tokio::main]
async fn main2() {
    let filesystem = Filesystem::new_tokio(tokio::runtime::Handle::current(), 100);

    let path = "/Users/parker.timmerman/Development/pt_forks/pb/pb-filesystem/src".to_string();
    let parent = filesystem
        .open(path.to_string())
        .as_directory()
        .await
        .expect("failed to open");
    let start = Instant::now();
    for _ in 0..100_000 {
        let (handle, _stat) = parent
            .openat("filesystem.rs".to_string())
            .as_file()
            .await
            .expect("failed to open");
        let hash = handle
            .read_with(|mut bytes| {
                let mut hasher = blake3::Hasher::new();
                while let Some(byte_result) = bytes.next() {
                    let bytes = byte_result?;
                    hasher.update(bytes);
                }
                Ok(hasher.finalize())
            })
            .await
            .expect("failed to read and hash file");
        handle.close().await.unwrap();
        std::hint::black_box(hash);
    }
    let total = start.elapsed();
    println!("{total:?}");

    let path = "/Users/parker.timmerman/Development/pt_forks/pb/pb-filesystem/src/filesystem.rs"
        .to_string();
    let start = Instant::now();
    for _ in 0..100_000 {
        let (handle, _stat) = filesystem
            .open(path.to_string())
            .as_file()
            .await
            .expect("failed to open");
        let hash = handle
            .read_with(|mut bytes| {
                let mut hasher = blake3::Hasher::new();
                while let Some(byte_result) = bytes.next() {
                    let bytes = byte_result?;
                    hasher.update(bytes);
                }
                Ok(hasher.finalize())
            })
            .await
            .expect("failed to read and hash file");
        handle.close().await.unwrap();
        std::hint::black_box(hash);
    }
    let total = start.elapsed();
    println!("{total:?}");

    let path = "/Users/parker.timmerman/Development/pt_forks/pb/pb-filesystem/src/filesystem.rs"
        .to_string();
    let start = Instant::now();
    let mut buf = Vec::new();
    for _ in 0..100_000 {
        let mut file = tokio::fs::File::open(&path).await.unwrap();
        buf.clear();
        file.read_to_end(&mut buf).await.unwrap();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&buf[..]);
        let hash = hasher.finalize();
        std::hint::black_box(hash);
    }
    let total = start.elapsed();
    println!("{total:?}");
}

#[tokio::main]
async fn main5() {
    let filesystem = Filesystem::new_tokio(tokio::runtime::Handle::current(), 100);

    let path = "/Users/parker.timmerman/Development/pt_forks/pb/pb-filesystem".to_string();
    let handle = filesystem
        .open(path.to_string())
        .as_directory()
        .await
        .expect("failed to open");
    let stat = handle.stat().await.expect("failed to stat");
    println!("{stat:?}");

    let filenames = handle.list().await.expect("failed to list dir");
    println!("{filenames:?}");

    let stat2 = handle.stat().await.expect("failed to stat a 2nd time");
    println!("{stat2:?}");

    let (cargo_toml, _stat) = handle
        .openat("Cargo.toml".to_string())
        .as_file()
        .await
        .expect("failed to open Cargo.toml");
    let stat = cargo_toml.stat().await.expect("failed to stat Cargo.toml");
    println!("{stat:?}");

    let hash = cargo_toml
        .read_with(|mut bytes| {
            let mut hasher = blake3::Hasher::new();
            while let Some(byte_result) = bytes.next() {
                let bytes = byte_result?;
                hasher.update(bytes);
            }
            Ok(hasher.finalize())
        })
        .await
        .expect("failed to read and hash file");
    println!("{hash:?}");

    let path = "/Users/parker.timmerman/Development/pt_forks/pb/pb-filesystem/src/filesystem.rs"
        .to_string();
    let (handle, _stat) = filesystem
        .open(path.to_string())
        .as_file()
        .await
        .expect("failed to open");
    let hash = handle
        .read_with(|mut bytes| {
            let mut hasher = blake3::Hasher::new();
            while let Some(byte_result) = bytes.next() {
                let bytes = byte_result?;
                hasher.update(bytes);
            }
            Ok(hasher.finalize())
        })
        .await
        .expect("failed to read and hash file");
    println!("{hash:?}");

    let file = std::fs::File::open(
        "/Users/parker.timmerman/Development/pt_forks/pb/pb-filesystem/src/filesystem.rs",
    )
    .unwrap();
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file).expect("failed to hash file");
    let hash = hasher.finalize();
    println!("{hash:?}");
}
