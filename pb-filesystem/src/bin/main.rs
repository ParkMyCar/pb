use std::{
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
    time::Instant,
};

use notify::{RecursiveMode, Watcher};
use pb_filesystem::filesystem::Filesystem;
use pb_ore::iter::LendingIterator;

use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let filesystem = Filesystem::new(8, 1024);
    let root = filesystem
        .open("/Users/parker/Development")
        .as_directory()
        .await
        .unwrap();

    let mut ignore_set = globset::GlobSetBuilder::new();
    ignore_set.add(globset::Glob::new("**/target").unwrap());
    let ignore_set = ignore_set.build().unwrap();

    let start = Instant::now();
    let num_files = Arc::new(AtomicUsize::new(0));
    let num_files_ = Arc::clone(&num_files);
    let tree = root
        .tree()
        .ignore(ignore_set)
        .with_data(move |_stat, mut reader| {
            num_files_.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut hasher = xxhash_rust::xxh3::Xxh3Default::new();
            while let Some(read) = reader.next() {
                let data = read?;
                hasher.update(data);
            }
            Ok(hasher.digest())
        })
        .await
        .unwrap();
    let elapsed = start.elapsed();
    root.close().await.unwrap();
    let num_files = num_files.load(std::sync::atomic::Ordering::Relaxed);

    println!("{elapsed:?} for {num_files} files");
}

#[tokio::main(flavor = "current_thread")]
async fn main2() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let path = "/Users/parker/Development/pb";
    let filesystem = Filesystem::new(4, 1024);
    let root = filesystem
        .open(path.to_string())
        .as_directory()
        .await
        .unwrap();

    let mut ignore_set = globset::GlobSetBuilder::new();
    ignore_set.add(globset::Glob::new("**/target/**").unwrap());
    let ignore_set = ignore_set.build().unwrap();

    let tree = root.tree().ignore(ignore_set).await?;
    println!("{tree}");

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(
        Path::new("/Users/parker/Development/pb"),
        RecursiveMode::Recursive,
    )?;

    for res in rx {
        let Ok(event) = res else {
            continue;
        };
        let ignored = event.paths.iter().all(|path| tree.ignored(path));
        if ignored {
            continue;
        }

        tracing::info!(?event, "got FS event");
    }

    Ok(())
}
