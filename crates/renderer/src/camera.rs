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
        if let Some(c) = center {
            // Capture the world point currently under the cursor BEFORE the scale
            // changes — screen_to_world reads `self.scale`, so it must run while
            // the old scale is still in effect.
            let world = self.screen_to_world(c);
            self.scale = (self.scale * factor).clamp(0.1, 20.0);
            // Recompute the offset so that the same world point stays under the
            // cursor after zooming (previously the offset was unchanged because the
            // world point was computed with the *new* scale, anchoring zoom to the
            // world origin instead of the cursor).
            self.offset = Vec2::new(world.x - c.x / self.scale, world.y - c.y / self.scale);
        } else {
            self.scale = (self.scale * factor).clamp(0.1, 20.0);
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
