use crate::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn from_min_max(min: Vec2<f32>, max: Vec2<f32>) -> Self {
        Self {
            x: min.x,
            y: min.y,
            w: max.x - min.x,
            h: max.y - min.y,
        }
    }
    pub fn contains(&self, p: Vec2<f32>) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
    pub fn min(&self) -> Vec2<f32> {
        Vec2::new(self.x, self.y)
    }
    pub fn max(&self) -> Vec2<f32> {
        Vec2::new(self.x + self.w, self.y + self.h)
    }
}
