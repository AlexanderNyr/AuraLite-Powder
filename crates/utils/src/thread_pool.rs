use rayon::ThreadPool as RayonThreadPool;
use rayon::ThreadPoolBuilder;

/// Wrapper around rayon thread pool with reservation for render/UI threads
pub struct ThreadPool {
    inner: Option<RayonThreadPool>,
    pub num_threads: usize,
}

impl ThreadPool {
    pub fn new() -> Self {
        let cpus = num_cpus::get().saturating_sub(2).max(1);
        Self::with_threads(cpus)
    }
    pub fn with_threads(n: usize) -> Self {
        let pool = ThreadPoolBuilder::new()
            .num_threads(n)
            .build()
            .ok();
        Self { inner: pool, num_threads: n }
    }
    pub fn install<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        if let Some(pool) = &self.inner {
            pool.install(f)
        } else {
            f()
        }
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to check num_cpus without extra dependency, fallback to 4
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}
