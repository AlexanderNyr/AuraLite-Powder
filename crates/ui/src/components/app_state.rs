use crate::brush::BrushSettings;
use aura_lite_core::{Mission, MissionId, Particle, Scenario, SimulationState};
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
    /// Keys 1–0 pick these ten slots.
    pub hotbar: [u16; 10],
    pub favorites: Vec<u16>,
}

impl Default for PalettePanel {
    fn default() -> Self {
        Self {
            selected_id: 1,
            search_query: String::new(),
            hotbar: [1, 2, 4, 13, 21, 20, 32, 33, 34, 40],
            favorites: vec![1, 2, 4, 32, 40],
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
    /// World-space drag rectangle while copy/rect is held.
    pub drag_rect: Option<(i32, i32, i32, i32)>,
    pub recording: bool,
    pub rec_frames: Vec<Vec<u8>>,
    pub rec_w: u32,
    pub rec_h: u32,
    pub mission: Option<Mission>,
    pub tutorial_step: u8,
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
            drag_rect: None,
            recording: false,
            rec_frames: Vec::new(),
            rec_w: 0,
            rec_h: 0,
            mission: None,
            tutorial_step: 0,
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

    pub fn select_hotbar(&mut self, slot: usize) {
        if let Some(&id) = self.palette.hotbar.get(slot) {
            self.set_selected_element(id);
        }
    }

    pub fn cycle_hotbar(&mut self, dir: i32) {
        let cur = self.palette.selected_id;
        let idx = self
            .palette
            .hotbar
            .iter()
            .position(|&id| id == cur)
            .unwrap_or(0);
        let n = self.palette.hotbar.len() as i32;
        let next = ((idx as i32 + dir).rem_euclid(n)) as usize;
        self.select_hotbar(next);
    }

    pub fn toggle_favorite(&mut self, id: u16) {
        if let Some(i) = self.palette.favorites.iter().position(|&x| x == id) {
            self.palette.favorites.remove(i);
        } else {
            self.palette.favorites.push(id);
        }
    }

    pub fn push_rec_frame(&mut self, rgba: &[u8], w: u32, h: u32) {
        if !self.recording {
            return;
        }
        if self.rec_frames.len() >= 90 {
            return;
        }
        let (out, ow, oh) = if w > 320 {
            downsample_rgba(rgba, w, h, 320)
        } else {
            (rgba.to_vec(), w, h)
        };
        self.rec_w = ow;
        self.rec_h = oh;
        self.rec_frames.push(out);
    }

    pub fn start_mission(&mut self, id: MissionId) {
        self.push_undo();
        self.mission = Some(Mission::start(&mut self.simulation, id));
    }

    pub fn resize_grid(&mut self, w: u32, h: u32) {
        self.push_undo();
        self.simulation.resize(w, h);
        self.controller.grid_width = w;
        self.controller.grid_height = h;
        self.simulation.setup_reactor_demo();
    }

    pub fn bump_radius(&mut self, delta: i32) {
        let r = self.tools.brush.radius as i32 + delta;
        self.tools.brush.radius = r.clamp(1, 20) as u32;
    }

    pub fn advance_tutorial(&mut self) {
        if self.tutorial_step < 5 {
            self.tutorial_step += 1;
        } else {
            self.show_tutorial = false;
        }
    }
}

fn downsample_rgba(src: &[u8], w: u32, h: u32, max_w: u32) -> (Vec<u8>, u32, u32) {
    let scale = (w as f32 / max_w as f32).ceil().max(1.0) as u32;
    let nw = (w / scale).max(1);
    let nh = (h / scale).max(1);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let sx = (x * scale).min(w - 1) as usize;
            let sy = (y * scale).min(h - 1) as usize;
            let si = (sy * w as usize + sx) * 4;
            let di = ((y * nw + x) * 4) as usize;
            if si + 3 < src.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    (out, nw, nh)
}
