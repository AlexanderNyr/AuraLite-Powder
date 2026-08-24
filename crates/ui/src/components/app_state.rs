use crate::brush::BrushSettings;
use aura_lite_core::{Particle, Scenario, SimulationState};
use aura_lite_renderer::{Camera, OverlayMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationController {
    pub speed: f32,
    pub paused: bool,
    pub tick_rate: u32,
    pub grid_width: u32,
    pub grid_height: u32,
}

impl Default for SimulationController {
    fn default() -> Self {
        Self {
            speed: 1.0,
            paused: false,
            tick_rate: 60,
            grid_width: 256,
            grid_height: 256,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PalettePanel {
    pub selected_id: u16,
    pub search_query: String,
}

impl Default for PalettePanel {
    fn default() -> Self {
        Self {
            selected_id: 1,
            search_query: String::new(),
        }
    }
}

impl PalettePanel {
    pub fn elements_filtered(&self) -> Vec<u16> {
        let all = aura_lite_elements::registry::all_definitions();
        if self.search_query.is_empty() {
            all.into_iter().map(|d| d.id).collect()
        } else {
            let q = self.search_query.to_lowercase();
            all.into_iter()
                .filter(|d| d.name.to_lowercase().contains(&q))
                .map(|d| d.id)
                .collect()
        }
    }
}

#[derive(Default)]
pub struct ToolPanel {
    pub brush: BrushSettings,
    pub line_start: Option<(i32, i32)>,
}

#[derive(Default)]
pub struct PropertyInspector {
    pub hovered_x: Option<u32>,
    pub hovered_y: Option<u32>,
}

impl PropertyInspector {
    pub fn inspect(&self, sim: &SimulationState) -> Option<String> {
        let x = self.hovered_x?;
        let y = self.hovered_y?;
        let p = sim.grid.get(x, y)?;
        let def_name = aura_lite_elements::registry::name_for_id(p.element_id);
        Some(format!(
            "Pos: ({}, {})\nElement: {} (id {})\nTemp: {} K\nFlags: {}\nLifetime: {}\nDensity: {:.2}\nKind: {:?}",
            x, y, def_name, p.element_id, p.temperature, p.flags, p.lifetime,
            aura_lite_core::element_id::density_for_id(p.element_id),
            aura_lite_core::element_id::kind_for_id(p.element_id)
        ))
    }
}

pub struct SaveLoadPanel {
    pub last_save_path: Option<String>,
    pub compression_enabled: bool,
    pub version_label: String,
}

impl Default for SaveLoadPanel {
    fn default() -> Self {
        Self {
            last_save_path: None,
            compression_enabled: false,
            version_label: format!("v{}", aura_lite_core::CORE_VERSION),
        }
    }
}

pub struct InfoPanel {
    pub fps: f32,
    pub tick_rate: f32,
    pub particle_count: usize,
    pub active_reactions: u64,
    pub k_effective: f32,
}

impl Default for InfoPanel {
    fn default() -> Self {
        Self {
            fps: 0.0,
            tick_rate: 0.0,
            particle_count: 0,
            active_reactions: 0,
            k_effective: 0.0,
        }
    }
}

/// Top-level AppState as described in spec
pub struct AppState {
    pub simulation: SimulationState,
    pub controller: SimulationController,
    pub camera: Camera,
    pub grid_view: GridView,
    pub palette: PalettePanel,
    pub tools: ToolPanel,
    pub inspector: PropertyInspector,
    pub save_load: SaveLoadPanel,
    pub info: InfoPanel,
    pub overlay: OverlayMode,
    pub show_tutorial: bool,
    undo: Vec<Vec<Particle>>,
    pub clipboard: Vec<(i32, i32, Particle)>,
}

pub struct GridView {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for GridView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

impl AppState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            simulation: SimulationState::new(width, height, 42),
            controller: SimulationController {
                grid_width: width,
                grid_height: height,
                ..Default::default()
            },
            camera: Camera::new(width as f32, height as f32),
            grid_view: GridView::default(),
            palette: PalettePanel::default(),
            tools: ToolPanel::default(),
            inspector: PropertyInspector::default(),
            save_load: SaveLoadPanel::default(),
            info: InfoPanel::default(),
            overlay: OverlayMode::None,
            show_tutorial: true,
            undo: Vec::new(),
            clipboard: Vec::new(),
        }
    }

    pub fn push_undo(&mut self) {
        self.undo.push(self.simulation.grid.particles.clone());
        if self.undo.len() > 24 {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            if prev.len() == self.simulation.grid.particles.len() {
                self.simulation.grid.particles = prev;
            }
        }
    }

    pub fn apply_scenario(&mut self, scene: Scenario) {
        self.push_undo();
        self.simulation.load_scenario(scene);
    }

    pub fn copy_from(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.clipboard.clear();
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let cx = (min_x + max_x) / 2;
        let cy = (min_y + max_y) / 2;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if !self.simulation.grid.in_bounds(x, y) {
                    continue;
                }
                let p = *self.simulation.grid.get(x as u32, y as u32).unwrap();
                if !p.is_empty() {
                    self.clipboard.push((x - cx, y - cy, p));
                }
            }
        }
    }

    pub fn stamp_at(&mut self, x: i32, y: i32) {
        self.push_undo();
        for &(dx, dy, p) in &self.clipboard {
            let nx = x + dx;
            let ny = y + dy;
            if self.simulation.grid.in_bounds(nx, ny) {
                self.simulation.grid.set(nx as u32, ny as u32, p);
            }
        }
    }

    pub fn update_info(&mut self, fps: f32) {
        self.info.fps = fps;
        self.info.particle_count = self.simulation.grid.count_non_empty();
        self.info.active_reactions = self.simulation.reaction_count;
        self.info.k_effective = self.simulation.k_effective;
        self.info.tick_rate = self.controller.tick_rate as f32 * self.controller.speed;
    }

    pub fn set_selected_element(&mut self, id: u16) {
        self.palette.selected_id = id;
        self.tools.brush.selected_element = id;
    }
}
