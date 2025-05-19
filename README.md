# Tech talk for "Difference between mio and tokio in Rust"

## Task

Write a server:

 * Listen on TCP socket `127.0.0.1:1234`
 * Accepting a u32 native byte order from client.
 * Sleep u32 milliseconds and reply `done-<u32>ms` string to client.

Write a client:
 * Create 10 threads
 * Each thread will connect to `127.0.0.1:1234` and sending a random u32
   integer.
 * Wait server's reply and print it.

Please try to get the server done in two ways:
 * mio(https://github.com/tokio-rs/mio) single thread.
 * tokio(https://github.com/tokio-rs/tokio/) multi-threads.

For client, just choose your favorite way.

## mio Server

```
cargo run --bin mio-srv
```

## tokio Server

```
cargo run --bin tokio-srv
```

## Client

```
cargo run --bin client-srv
```
