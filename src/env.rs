use std::sync::Arc;

pub struct Env {
    pub port: u16,
    pub upstreams: Arc<Vec<Arc<str>>>,
    pub workers: usize,
}

pub fn from_env() -> Env {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let upstreams: Arc<Vec<Arc<str>>> = Arc::new(
        std::env::var("UPSTREAMS")
            .expect("UPSTREAMS env var required (comma-separated UDS paths)")
            .split(',')
            .map(|s| Arc::from(s.trim()))
            .filter(|s: &Arc<str>| !s.is_empty())
            .collect(),
    );

    let workers = std::env::var("WORKERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        });

    assert!(
        !upstreams.is_empty(),
        "UPSTREAMS must contain at least one path"
    );

    Env {
        port,
        upstreams,
        workers,
    }
}
