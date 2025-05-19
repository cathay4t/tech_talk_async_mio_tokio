// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::os::fd::{AsFd, AsRawFd};
use std::sync::atomic::{AtomicUsize, Ordering};

use mio::{
    Events, Interest, Poll, Token,
    net::{TcpListener, TcpStream},
    unix::SourceFd,
};
use nix::sys::{
    time::TimeSpec,
    timerfd::{
        ClockId::CLOCK_BOOTTIME, Expiration, TimerFd, TimerFlags,
        TimerSetTimeFlags,
    },
};

const LISTENER: Token = Token(0);
const CLIENT_CAPACITY: usize = 1024;
static CLIENT_TOKEN: AtomicUsize = AtomicUsize::new(10000);

#[derive(Debug)]
struct CliRequset {
    time_ms: u32,
    token: Token,
    stream: TcpStream,
    time_fd: TimerFd,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TokenType {
    CanRead,
    CanWrite,
    SleepDone,
}

impl CliRequset {
    fn reply(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let reply_str = format!("done-{}ms", self.time_ms);
        // TODO: Keep retry till write all
        // TODO: Client stream might closed or cannot write
        let mut write_len = 0;
        let len = reply_str.as_bytes().len();
        while write_len < len {
            write_len += self.stream.write(reply_str.as_bytes())?;
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "127.0.0.1:1234".parse()?;
    let mut listener = TcpListener::bind(addr)?;

    let mut poll = Poll::new()?;

    poll.registry()
        .register(&mut listener, LISTENER, Interest::READABLE)?;

    let mut events = Events::with_capacity(CLIENT_CAPACITY * 2);

    let mut cli_requests: HashMap<Token, CliRequset> =
        HashMap::with_capacity(CLIENT_CAPACITY);
    let mut read_queue: HashMap<Token, TcpStream> =
        HashMap::with_capacity(CLIENT_CAPACITY);
    let mut token_queue: HashMap<Token, TokenType> =
        HashMap::with_capacity(CLIENT_CAPACITY);

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                LISTENER => loop {
                    let mut stream = match listener.accept() {
                        Ok((stream, _)) => stream,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            // If we get a `WouldBlock` error we know our
                            // listener has no more incoming connections queued,
                            // so we can return to polling and wait for some
                            // more.
                            break;
                        }
                        Err(e) => {
                            eprint!("Error on accepting client {e}");
                            return Err(Box::new(e));
                        }
                    };
                    eprintln!("Connect client {:?}", stream);
                    let read_token = next_token();

                    poll.registry().register(
                        &mut stream,
                        read_token,
                        Interest::READABLE,
                    )?;
                    token_queue.insert(read_token, TokenType::CanRead);
                    read_queue.insert(read_token, stream);
                },
                token => {
                    let token_type = match token_queue.remove(&token) {
                        Some(t) => t,
                        None => continue,
                    };
                    match token_type {
                        TokenType::CanRead => {
                            let tcp_stream = match read_queue.remove(&token) {
                                Some(stream) => stream,
                                None => continue,
                            };
                            let cli_req = handle_client(&poll, tcp_stream)?;
                            token_queue
                                .insert(cli_req.token, TokenType::SleepDone);
                            cli_requests.insert(cli_req.token, cli_req);
                        }
                        TokenType::SleepDone => {
                            let mut cli_req = match cli_requests.remove(&token)
                            {
                                Some(c) => c,
                                None => continue,
                            };
                            poll.registry().deregister(&mut SourceFd(
                                &cli_req.time_fd.as_fd().as_raw_fd(),
                            ))?;

                            let write_token = next_token();
                            poll.registry().register(
                                &mut cli_req.stream,
                                write_token,
                                Interest::WRITABLE,
                            )?;
                            token_queue
                                .insert(write_token, TokenType::CanWrite);
                            cli_requests.insert(write_token, cli_req);
                        }
                        TokenType::CanWrite => {
                            let mut cli_req = match cli_requests.remove(&token)
                            {
                                Some(c) => c,
                                None => continue,
                            };
                            poll.registry().deregister(&mut cli_req.stream)?;
                            cli_req.reply()?;
                        }
                    }
                }
            }
        }
    }
}

fn handle_client(
    poll: &Poll,
    mut stream: TcpStream,
) -> Result<CliRequset, Box<dyn std::error::Error>> {
    let mut buf = [0u8; 4];

    let mut read_count = 0;
    while read_count < 4 {
        read_count += stream.read(&mut buf)?;
    }
    poll.registry().deregister(&mut stream)?;

    let time_ms = u32::from_be_bytes(buf);
    eprintln!("Got client request {}ms", time_ms);

    let fd = TimerFd::new(CLOCK_BOOTTIME, TimerFlags::empty())?;

    fd.set(
        Expiration::OneShot(TimeSpec::from_duration(
            std::time::Duration::from_millis(time_ms.into()),
        )),
        TimerSetTimeFlags::empty(),
    )?;

    let token = next_token();

    poll.registry().register(
        &mut SourceFd(&fd.as_fd().as_raw_fd()),
        token,
        Interest::READABLE,
    )?;

    Ok(CliRequset {
        time_ms,
        token,
        stream,
        time_fd: fd,
    })
}

// TODO: AtomicUsize wrap when overflow, it might cause problem.
//       Better check existing token list, and find next free number
fn next_token() -> Token {
    let token = CLIENT_TOKEN.fetch_add(1, Ordering::SeqCst);
    Token(token)
}
