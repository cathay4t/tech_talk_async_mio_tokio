// SPDX-License-Identifier: Apache-2.0

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinSet;

const BUFF_SIZE: usize = 4096;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut task_set = JoinSet::new();
    let mut times: Vec<u32> = (1..11).collect();
    times.reverse();

    for x in times {
        task_set.spawn(async move {
            call_srv(x * 200).await.unwrap();
        });
    }

    while !task_set.join_next().await.is_none() {}

    Ok(())
}

async fn call_srv(time_ms: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:1234").await?;

    let mut buff = vec![0u8; BUFF_SIZE];

    println!("Sending request {}ms", time_ms);

    stream.write_all(&time_ms.to_be_bytes()).await?;

    // TODO: Keep retry till end.
    stream.read(buff.as_mut_slice()).await?;

    println!(
        "Request {} ms got reply from server: {}",
        time_ms,
        String::from_utf8(buff)?
    );

    Ok(())
}
