use pb_file_tree::ContinualMetadataTree;
use pb_filesystem::{FileStat, filesystem::Filesystem};
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let filesystem = Filesystem::new(4, 1024);
    let file_tree: ContinualMetadataTree<FileStat> = ContinualMetadataTree::new(
        "/Users/parker/Development/pb/pb/pb-file-tree".into(),
        filesystem,
        None,
        None,
    )
    .await
    .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(20).into()).await;
}
