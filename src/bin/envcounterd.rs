#[tokio::main]
async fn main() {
    if let Err(e) = envcounter::server::run().await {
        eprintln!("envcounterd: {}", e);
        std::process::exit(1);
    }
}
