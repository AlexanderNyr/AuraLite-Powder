use aura_lite_utils::color::Rgba;

/// RGBA alias for trait
pub type RgbaColor = Rgba;

/// Result of element update
#[derive(Clone, Debug)]
pub enum UpdateResult {
    NoChange,
    Transform { new_id: u16 },
    Spawn { x: i32, y: i32, id: u16 },
    Changed,
}

/// Context for element update
pub struct UpdateCtx {
    pub x: u32,
    pub y: u32,
    pub temperature: u16,
    pub tick: u64,
    pub rng_seed: u64,
}

/// Context for reaction checks
pub struct ReactionCtx {
    pub temperature: u16,
    pub neighbor_temp: u16,
    pub tick: u64,
}

#[derive(Clone, Debug)]
pub enum ReactionEvent {
    Fission { products: Vec<u16>, neutrons: u32, energy: f32 },
    Fusion { product: u16, energy: f32 },
    Decay { daughter: u16, radiation: u16 },
    NoReaction,
}

/// Definition of an element (data-driven)
#[derive(Clone, Debug)]
pub struct ElementDef {
    pub id: u16,
    pub name: &'static str,
    pub color: Rgba,
    pub density: f32,
    pub temperature: u16,
    pub half_life_ticks: u64,
    pub is_fissile: bool,
    pub is_moderator: bool,
    pub is_radiation: bool,
    pub penetration: u32,
}

/// Trait for custom element behavior
pub trait Element: Send + Sync + 'static {
    fn id(&self) -> u16;
    fn name(&self) -> &'static str;
    fn color(&self) -> Rgba;
    fn density(&self) -> f32;
    fn temperature(&self) -> u16 {
        293
    }
    fn update(&self, ctx: &mut UpdateCtx) -> UpdateResult {
        let _ = ctx;
        UpdateResult::NoChange
    }
    fn react(&self, neighbor: &ElementDef, ctx: &ReactionCtx) -> Option<ReactionEvent> {
        let _ = (neighbor, ctx);
        None
    }
}
