use crate::element_trait::ElementDef;
use aura_lite_utils::color::Rgba;
use aura_lite_core::element_id::*;

// Full registry of elements
#[allow(clippy::too_many_arguments)]
fn def(id: u16, name: &'static str, color: Rgba, density: f32, temp: u16, half_life: u64, fissile: bool, moderator: bool, radiation: bool, pen: u32) -> ElementDef {
    ElementDef {
        id,
        name,
        color,
        density,
        temperature: temp,
        half_life_ticks: half_life,
        is_fissile: fissile,
        is_moderator: moderator,
        is_radiation: radiation,
        penetration: pen,
    }
}

pub fn all_definitions() -> Vec<ElementDef> {
    vec![
        def(AIR, "Air", Rgba::new(0,0,0,0), 0.0, 293, 0, false, false, false, 0),
        def(SAND, "Sand", Rgba::rgb(194,178,128), 2.5, 293, 0, false, false, false, 0),
        def(WATER, "Water", Rgba::rgb(64,164,223), 1.0, 293, 0, false, true, false, 0),
        def(STONE, "Stone", Rgba::rgb(120,120,120), 3.0, 293, 0, false, false, false, 0),
        def(U235, "Uranium-235", Rgba::rgb(80, 200, 60), 19.1, 293, 1_000_000, true, false, false, 0),
        def(U238, "Uranium-238", Rgba::rgb(70, 170, 50), 19.1, 293, 2_000_000, true, false, false, 0),
        def(PU239, "Plutonium-239", Rgba::rgb(200, 60, 60), 19.8, 293, 500_000, true, false, false, 0),
        def(PU240, "Plutonium-240", Rgba::rgb(180, 40, 40), 19.8, 293, 400_000, true, false, false, 0),
        def(HEAVY_WATER, "Heavy Water", Rgba::rgb(80, 180, 230), 1.1, 293, 0, false, true, false, 0),
        def(GRAPHITE, "Graphite", Rgba::rgb(50,50,50), 2.2, 293, 0, false, true, false, 0),
        def(LEAD, "Lead", Rgba::rgb(90, 90, 100), 11.3, 293, 0, false, false, false, 0),
        def(CONCRETE, "Concrete", Rgba::rgb(160,160,155), 2.8, 293, 0, false, false, false, 0),
        def(STEEL, "Steel", Rgba::rgb(170,170,180), 7.8, 293, 0, false, false, false, 0),
        def(NEUTRON_THERMAL, "Thermal Neutron", Rgba::rgb(255, 255, 150), 0.001, 350, 0, false, false, true, 8),
        def(NEUTRON_FAST, "Fast Neutron", Rgba::rgb(255, 80, 80), 0.001, 800, 0, false, false, true, 15),
        def(GAMMA, "Gamma Ray", Rgba::rgb(255, 255, 0), 0.0, 1000, 0, false, false, true, 12),
        def(ALPHA, "Alpha", Rgba::rgb(255, 120, 0), 0.01, 400, 0, false, false, true, 2),
        def(BETA, "Beta", Rgba::rgb(80, 200, 255), 0.005, 400, 0, false, false, true, 4),
        def(DEPLETED_URANIUM, "Depleted Uranium", Rgba::rgb(60, 120, 40), 19.1, 293, 0, false, false, false, 0),
        def(FISSION_PRODUCTS, "Fission Products", Rgba::rgb(100, 180, 40), 5.0, 600, 0, false, false, false, 0),
        def(TRITIUM, "Tritium", Rgba::rgb(150, 200, 255), 0.8, 293, 100_000, false, false, false, 0),
        def(DEUTERIUM, "Deuterium", Rgba::rgb(120, 220, 255), 0.8, 293, 0, false, false, false, 0),
        def(TNT, "TNT", Rgba::rgb(220, 50, 50), 1.6, 293, 0, false, false, false, 0),
        def(HYDROGEN, "Hydrogen", Rgba::rgb(200, 230, 255), 0.07, 293, 0, false, false, false, 0),
        def(LITHIUM, "Lithium", Rgba::rgb(200, 200, 200), 0.5, 293, 0, false, false, false, 0),
        def(HELIUM, "Helium", Rgba::rgb(255, 254, 200), 0.1, 293, 0, false, false, false, 0),
        def(MOLTEN_FUEL, "Molten Fuel", Rgba::rgb(255, 100, 0), 10.0, 2500, 0, true, false, false, 0),
        def(FALLOUT, "Fallout", Rgba::rgb(80, 80, 30), 2.0, 400, 0, false, false, false, 0),
        def(BORON, "Boron", Rgba::rgb(100, 90, 80), 2.3, 293, 0, false, false, false, 0),
    ]
}

pub fn get_definition(id: u16) -> Option<ElementDef> {
    all_definitions().into_iter().find(|d| d.id == id)
}

pub fn color_for_id(id: u16) -> [u8;4] {
    if let Some(def) = get_definition(id) {
        [def.color.r, def.color.g, def.color.b, def.color.a]
    } else {
        match id {
            0 => [0,0,0,0],
            1 => [194,178,128,255],
            2 => [64,164,223,255],
            _ => [255,0,255,255],
        }
    }
}

pub fn name_for_id(id: u16) -> &'static str {
    match id {
        AIR => "Air",
        SAND => "Sand",
        WATER => "Water",
        STONE => "Stone",
        U235 => "Uranium-235",
        U238 => "Uranium-238",
        PU239 => "Plutonium-239",
        PU240 => "Plutonium-240",
        HEAVY_WATER => "Heavy Water",
        GRAPHITE => "Graphite",
        LEAD => "Lead",
        CONCRETE => "Concrete",
        STEEL => "Steel",
        NEUTRON_THERMAL => "Thermal Neutron",
        NEUTRON_FAST => "Fast Neutron",
        GAMMA => "Gamma Ray",
        ALPHA => "Alpha",
        BETA => "Beta",
        DEPLETED_URANIUM => "Depleted Uranium",
        FISSION_PRODUCTS => "Fission Products",
        TRITIUM => "Tritium",
        DEUTERIUM => "Deuterium",
        TNT => "TNT",
        HYDROGEN => "Hydrogen",
        LITHIUM => "Lithium",
        HELIUM => "Helium",
        MOLTEN_FUEL => "Molten Fuel",
        FALLOUT => "Fallout",
        BORON => "Boron",
        _ => "Unknown",
    }
}

pub fn density_for_id(id: u16) -> f32 {
    aura_lite_core::element_id::density_for_id(id)
}
