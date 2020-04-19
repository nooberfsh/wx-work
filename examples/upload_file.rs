use wx_work::client::Client;
use wx_work::media::FileType;

#[tokio::main]
async fn main() {
    let corp_id = "";
    let secret_id = "";

    let cli = Client::new(corp_id, secret_id).unwrap();
    cli.upload_file(FileType::Video, "path/to/file")
        .await
        .unwrap();
}
