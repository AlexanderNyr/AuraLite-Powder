//! Utility crate for AuraLite Powder
//! Provides math, chunking, threading helpers

pub mod atomic_f32;
pub mod chunking;
pub mod color;
pub mod math;
pub mod rect;
pub mod thread_pool;
pub mod vec2;

pub use atomic_f32::AtomicF32;
pub use chunking::{ChunkMeta, ChunkPool, CHUNK_SIZE};
pub use color::Rgba;
pub use rect::Rect;
pub use thread_pool::ThreadPool;
pub use vec2::Vec2;
