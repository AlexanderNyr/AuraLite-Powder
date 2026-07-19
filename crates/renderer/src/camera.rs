use aura_lite_utils::{Rect, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub offset: Vec2<f32>,
    pub scale: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl Camera {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            offset: Vec2::new(0.0, 0.0),
            scale: 1.0,
            viewport_width,
            viewport_height,
        }
    }

    pub fn world_to_screen(&self, world: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(
            (world.x - self.offset.x) * self.scale,
            (world.y - self.offset.y) * self.scale,
        )
    }

    pub fn screen_to_world(&self, screen: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(
            screen.x / self.scale + self.offset.x,
            screen.y / self.scale + self.offset.y,
        )
    }

    pub fn visible_rect(&self) -> Rect {
        Rect::new(
            self.offset.x,
            self.offset.y,
            self.viewport_width / self.scale,
            self.viewport_height / self.scale,
        )
    }

    pub fn zoom(&mut self, factor: f32, center: Option<Vec2<f32>>) {
        let old_scale = self.scale;
        self.scale = (self.scale * factor).clamp(0.1, 20.0);
        if let Some(c) = center {
            // Adjust offset to keep center stable
            let world_center = self.screen_to_world(c);
            // After scale change, we want same world point under cursor
            // new_offset = world - screen/scale
            self.offset = Vec2::new(
                world_center.x - c.x / self.scale,
                world_center.y - c.y / self.scale,
            );
            let _ = old_scale;
        }
    }

    pub fn pan(&mut self, delta: Vec2<f32>) {
        self.offset.x -= delta.x / self.scale;
        self.offset.y -= delta.y / self.scale;
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.viewport_width = w;
        self.viewport_height = h;
    }
}
