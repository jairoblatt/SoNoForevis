mod env;
mod fd;
mod tcp;
mod uds;

fn main() {
    let env = env::from_env();

    let handles: Vec<_> = (0..env.workers)
        .map(|_| {
            let upstreams = env.upstreams.clone();
            std::thread::spawn(move || {
                let ctrl_fds: Vec<libc::c_int> = upstreams
                    .iter()
                    .map(|path| uds::connect_ctrl(&format!("{path}.ctrl")))
                    .collect();

                monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
                    .with_entries(4096)
                    .build()
                    .expect("failed to build IoUring runtime")
                    .block_on(tcp::accept_loop(env.port, ctrl_fds))
            })
        })
        .collect();

    for h in handles {
        h.join().expect("worker thread panicked");
    }
}
