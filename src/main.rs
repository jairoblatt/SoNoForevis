use std::sync::Arc;

use monoio::buf::{IoBuf, IoBufMut};
use monoio::io::{AsyncReadRent, AsyncWriteRentExt, Splitable};
use monoio::net::{TcpListener, TcpStream, UnixStream};
use socket2::{Domain, Socket, Type};

fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let buf_size: usize = std::env::var("BUF_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(65536);

    let upstreams: Arc<Vec<Arc<str>>> = Arc::new(
        std::env::var("UPSTREAMS")
            .expect("UPSTREAMS env var required (comma-separated UDS paths)")
            .split(',')
            .map(|s| Arc::from(s.trim()))
            .filter(|s: &Arc<str>| !s.is_empty())
            .collect(),
    );
    assert!(!upstreams.is_empty(), "UPSTREAMS must contain at least one path");

    let n = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let handles: Vec<_> = (0..n)
        .map(|_| {
            let upstreams = upstreams.clone();
            std::thread::spawn(move || {
                monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
                    .with_entries(4096)
                    .build()
                    .expect("failed to build IoUring runtime")
                    .block_on(accept_loop(port, buf_size, upstreams))
            })
        })
        .collect();

    for h in handles {
        h.join().expect("worker thread panicked");
    }
}

fn make_listener(port: u16) -> std::net::TcpListener {
    let sock = Socket::new(Domain::IPV4, Type::STREAM, None).expect("socket2::Socket::new");
    sock.set_reuse_address(true).expect("SO_REUSEADDR");
    // Each thread binds independently on the same port; the kernel distributes
    // incoming SYNs across all per-thread queues with no userspace contention.
    sock.set_reuse_port(true).expect("SO_REUSEPORT");
    sock.set_nonblocking(true).expect("O_NONBLOCK");
    let addr: std::net::SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
    sock.bind(&addr.into()).expect("bind");
    sock.listen(1024).expect("listen");
    sock.into()
}

async fn accept_loop(port: u16, buf_size: usize, upstreams: Arc<Vec<Arc<str>>>) {
    let listener = TcpListener::from_std(make_listener(port)).expect("TcpListener::from_std");
    let len = upstreams.len();
    let mut rr: usize = 0;
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                stream.set_nodelay(true).ok();
                let path = upstreams[rr % len].clone();
                rr = rr.wrapping_add(1);
                monoio::spawn(handle_connection(stream, path, buf_size));
            }
            Err(_) => {}
        }
    }
}

async fn handle_connection(tcp: TcpStream, uds_path: Arc<str>, buf_size: usize) {
    let uds = match UnixStream::connect(uds_path.as_ref()).await {
        Ok(s) => s,
        Err(_) => return,
    };
    let (tcp_r, tcp_w) = tcp.into_split();
    let (uds_r, uds_w) = uds.into_split();
    monoio::select! {
        _ = forward(tcp_r, uds_w, buf_size) => {}
        _ = forward(uds_r, tcp_w, buf_size) => {}
    }
}

async fn forward<R, W>(mut reader: R, mut writer: W, buf_size: usize)
where
    R: AsyncReadRent,
    W: AsyncWriteRentExt,
{
    let mut buf: Box<[u8]> = vec![0u8; buf_size].into_boxed_slice();
    loop {
        let (res, slice) = reader.read(buf.slice_mut(..)).await;
        buf = slice.into_inner();
        let n = match res {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        let (res, slice) = writer.write_all(buf.slice(0..n)).await;
        buf = slice.into_inner();
        if res.is_err() {
            return;
        }
    }
}
