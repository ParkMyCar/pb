use pb_filesystem::filesystem::Filesystem;

#[tokio::main]
async fn main() {
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

    let cargo_toml = handle
        .openat("Cargo.toml".to_string())
        .await
        .expect("failed to open Cargo.toml");
    let stat = cargo_toml.stat().await.expect("failed to stat Cargo.toml");
    println!("{stat:?}");
}
