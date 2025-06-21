use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use notify::{FsEventWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::Debouncer;
use pb_filesystem::{
    FileStat, filesystem::Filesystem, handle::internal::ReadIterator, tree::MetadataTree,
};

pub type FileWork<T> = Option<
    Arc<
        dyn for<'d> Fn(&'d FileStat, ReadIterator<'d>) -> Result<T, pb_filesystem::Error>
            + Send
            + Sync
            + 'static,
    >,
>;

/// A [`MetadataTree`] that watches file events for it's root directory and continually updates
/// itself.
pub struct ContinualMetadataTree<T: Clone + Send + 'static> {
    tree: Arc<Mutex<MetadataTree<FileStat>>>,
    filesystem: Filesystem,
    file_work: FileWork<T>,

    watcher2: Debouncer<FsEventWatcher>,
    watcher: std::thread::JoinHandle<()>,
}

impl<T: Clone + Send + 'static> ContinualMetadataTree<T> {
    pub async fn new(
        root_path: PathBuf,
        filesystem: Filesystem,
        ignore: Option<globset::GlobSet>,
        file_work: FileWork<T>,
    ) -> Result<Self, anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer =
            notify_debouncer_mini::new_debouncer(std::time::Duration::from_millis(500), tx)?;
        debouncer
            .watcher()
            .watch(&root_path, RecursiveMode::Recursive)?;

        let root_dir = filesystem
            .open(root_path)
            .as_directory()
            .diagnostics("continual metadata tree")
            .await?;

        let mut tree_builder = root_dir.tree();
        if let Some(ignore) = ignore {
            tree_builder = tree_builder.ignore(ignore);
        }
        // if let Some(work) = file_work.as_ref() {
        //     let work = Arc::clone(work);
        //     tree_builder = tree_builder.with_data(move |stat, iter| (work)(stat, iter));
        // }

        let initial_tree = tree_builder.await?;
        println!("{initial_tree}");

        let tree = Arc::new(Mutex::new(initial_tree));

        let tree_ = Arc::clone(&tree);
        let watcher = std::thread::spawn(move || {
            let tree = tree_;
            loop {
                let events = match rx.recv() {
                    Ok(Ok(events)) => events,
                    Ok(Err(err)) => {
                        tracing::warn!(?err, "got an error, closing file watcher");
                        return;
                    }
                    Err(err) => {
                        tracing::warn!(?err, "closing file watcher");
                        return;
                    }
                };
                tracing::info!(?events, "got events!");
            }
        });

        Ok(ContinualMetadataTree {
            tree,
            filesystem,
            file_work,
            watcher2: debouncer,
            watcher,
        })
    }
}
