use pb_filesystem::filesystem::Filesystem;

#[tokio::main]
async fn main() {
    let filesystem = Filesystem::new_tokio(tokio::runtime::Handle::current(), 100);

    let path = "/Users/parker/Development/pb/Cargo.toml".to_string();
    let result = filesystem.stat(path.to_string()).await;

    println!("{result:?}");
}
