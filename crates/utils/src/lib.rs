//! Utility crate for AuraLite Powder
//! Provides math, chunking, threading helpers

pub mod vec2;
pub mod rect;
pub mod atomic_f32;
pub mod chunking;
pub mod thread_pool;
pub mod math;
pub mod color;

pub use vec2::Vec2;
pub use rect::Rect;
pub use atomic_f32::AtomicF32;
pub use chunking::{ChunkPool, ChunkMeta, CHUNK_SIZE};
pub use thread_pool::ThreadPool;
pub use color::Rgba;
