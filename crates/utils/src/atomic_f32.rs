use std::sync::atomic::{AtomicU32, Ordering};

/// Atomic f32 using AtomicU32 internally
#[derive(Debug)]
pub struct AtomicF32 {
    inner: AtomicU32,
}

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self {
            inner: AtomicU32::new(v.to_bits()),
        }
    }
    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.inner.load(order))
    }
    pub fn store(&self, v: f32, order: Ordering) {
        self.inner.store(v.to_bits(), order);
    }
    pub fn fetch_add(&self, v: f32, order: Ordering) -> f32 {
        loop {
            let current = self.inner.load(Ordering::Relaxed);
            let current_f = f32::from_bits(current);
            let new_f = current_f + v;
            let new = new_f.to_bits();
            match self.inner.compare_exchange_weak(current, new, order, Ordering::Relaxed) {
                Ok(_) => return current_f,
                Err(_) => continue,
            }
        }
    }
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}
