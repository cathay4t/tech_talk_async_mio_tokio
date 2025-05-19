// SPDX-License-Identifier: Apache-2.0

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:1234").await?;
    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = [0; 4];
            // Read a u32 from the client
            socket.read_exact(&mut buf).await.unwrap();
            let time_ms = u32::from_be_bytes(buf);
            eprintln!("Received number: {time_ms}");
            let reply = process_client_sleep(time_ms).await.unwrap();
            eprintln!("Replying {reply}");
            socket.write_all(&reply.as_bytes()).await.unwrap();
        });
    }
}

async fn process_client_sleep(
    time_ms: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    tokio::time::sleep(std::time::Duration::from_millis(time_ms.into())).await;
    Ok(format!("done-{time_ms}ms"))
}
