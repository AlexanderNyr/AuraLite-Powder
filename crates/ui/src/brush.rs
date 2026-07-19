use serde::{Deserialize, Serialize};
use aura_lite_core::{Grid, Particle};
use aura_lite_utils::math::bresenham_line;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BrushTool {
    #[default]
    Brush,
    Line,
    Fill,
    Eraser,
    Rectangle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrushSettings {
    pub tool: BrushTool,
    pub radius: u32,
    pub selected_element: u16,
    pub temperature: u16,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            tool: BrushTool::Brush,
            radius: 3,
            selected_element: 1, // sand
            temperature: 293,
        }
    }
}

impl BrushSettings {
    pub fn apply_brush(&self, grid: &mut Grid, cx: i32, cy: i32) {
        let r = self.radius as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx*dx + dy*dy > r*r { continue; }
                let x = cx + dx;
                let y = cy + dy;
                if !grid.in_bounds(x, y) { continue; }
                match self.tool {
                    BrushTool::Brush => {
                        grid.set(x as u32, y as u32, Particle::new(self.selected_element, self.temperature));
                    }
                    BrushTool::Eraser => {
                        grid.set(x as u32, y as u32, Particle::air());
                    }
                    _ => {
                        grid.set(x as u32, y as u32, Particle::new(self.selected_element, self.temperature));
                    }
                }
            }
        }
    }

    pub fn apply_line(&self, grid: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32) {
        for (x,y) in bresenham_line(x0, y0, x1, y1) {
            self.apply_brush(grid, x, y);
        }
    }

    pub fn apply_rectangle(&self, grid: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, filled: bool) {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        if filled {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    if !grid.in_bounds(x, y) { continue; }
                    grid.set(x as u32, y as u32, Particle::new(self.selected_element, self.temperature));
                }
            }
        } else {
            for x in min_x..=max_x {
                for y in [min_y, max_y] {
                    if !grid.in_bounds(x, y) { continue; }
                    grid.set(x as u32, y as u32, Particle::new(self.selected_element, self.temperature));
                }
            }
            for y in min_y..=max_y {
                for x in [min_x, max_x] {
                    if !grid.in_bounds(x, y) { continue; }
                    grid.set(x as u32, y as u32, Particle::new(self.selected_element, self.temperature));
                }
            }
        }
    }

    pub fn apply_fill(&self, grid: &mut Grid, start_x: i32, start_y: i32) {
        if !grid.in_bounds(start_x, start_y) { return; }
        let target_id = grid.get(start_x as u32, start_y as u32).map(|p| p.element_id).unwrap_or(0);
        if target_id == self.selected_element { return; }
        use std::collections::VecDeque;
        let mut queue = VecDeque::new();
        queue.push_back((start_x, start_y));
        let mut visited = std::collections::HashSet::new();
        let max_fill = 10000; // depth limit
        let mut count = 0;
        while let Some((x,y)) = queue.pop_front() {
            if count > max_fill { break; }
            if !grid.in_bounds(x, y) { continue; }
            if visited.contains(&(x,y)) { continue; }
            visited.insert((x,y));
            let cur_id = grid.get(x as u32, y as u32).map(|p| p.element_id).unwrap_or(0);
            if cur_id != target_id { continue; }
            grid.set(x as u32, y as u32, Particle::new(self.selected_element, self.temperature));
            count += 1;
            queue.push_back((x+1,y));
            queue.push_back((x-1,y));
            queue.push_back((x,y+1));
            queue.push_back((x,y-1));
        }
    }
}
