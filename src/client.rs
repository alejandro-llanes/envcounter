use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::protocol::{Request, Response};

fn socket_path() -> Result<PathBuf, std::io::Error> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR is not set",
        )
    })?;
    Ok(PathBuf::from(runtime_dir).join("envcounter.sock"))
}

pub async fn send(request: &Request) -> Result<Response, Box<dyn std::error::Error>> {
    let sock_path = socket_path()?;

    let stream = UnixStream::connect(&sock_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::ConnectionRefused
            || e.kind() == std::io::ErrorKind::NotFound
        {
            std::io::Error::new(e.kind(), "cannot connect to envcounterd; is the daemon running?")
        } else {
            e
        }
    })?;

    let (reader, mut writer) = stream.into_split();

    let mut data = serde_json::to_vec(request)?;
    data.push(b'\n');
    writer.write_all(&data).await?;
    writer.shutdown().await?;

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: Response = serde_json::from_str(line.trim())?;
    Ok(response)
}
