//! Central Element ID definitions, must stay in sync with elements crate registry

pub const AIR: u16 = 0;
pub const SAND: u16 = 1;
pub const WATER: u16 = 2;
pub const STONE: u16 = 3;
pub const U235: u16 = 4;
pub const U238: u16 = 5;
pub const PU239: u16 = 6;
pub const PU240: u16 = 7;
pub const HEAVY_WATER: u16 = 8; // D2O
pub const GRAPHITE: u16 = 9;
pub const LEAD: u16 = 10;
pub const CONCRETE: u16 = 11;
pub const STEEL: u16 = 12;
pub const NEUTRON_THERMAL: u16 = 13;
pub const NEUTRON_FAST: u16 = 14;
pub const GAMMA: u16 = 15;
pub const ALPHA: u16 = 16;
pub const BETA: u16 = 17;
pub const DEPLETED_URANIUM: u16 = 18;
pub const FISSION_PRODUCTS: u16 = 19;
pub const TRITIUM: u16 = 20;
pub const DEUTERIUM: u16 = 21;
pub const TNT: u16 = 22;
pub const HYDROGEN: u16 = 23;
pub const LITHIUM: u16 = 24;
pub const HELIUM: u16 = 25;
pub const MOLTEN_FUEL: u16 = 26;
pub const FALLOUT: u16 = 27;
pub const BORON: u16 = 28; // neutron absorber

pub const MAX_ELEMENT_ID: u16 = 31;

pub fn is_valid_id(id: u16) -> bool {
    id <= MAX_ELEMENT_ID
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Air,
    Sand,       // granular
    Liquid,
    Solid,
    Gas,
    Radiation,
    Molten,
    Absorber,
}

pub fn kind_for_id(id: u16) -> ElementKind {
    match id {
        AIR => ElementKind::Air,
        SAND => ElementKind::Sand,
        WATER | HEAVY_WATER => ElementKind::Liquid,
        STONE | CONCRETE | STEEL | LEAD | GRAPHITE | BORON => ElementKind::Solid,
        U235 | U238 | PU239 | PU240 | DEPLETED_URANIUM | FISSION_PRODUCTS | TRITIUM | DEUTERIUM | LITHIUM | HELIUM | MOLTEN_FUEL | FALLOUT | TNT => ElementKind::Solid, // some could be considered different but keep solid for physics
        HYDROGEN => ElementKind::Gas,
        NEUTRON_THERMAL | NEUTRON_FAST | GAMMA | ALPHA | BETA => ElementKind::Radiation,
        _ => ElementKind::Air,
    }
}

pub fn density_for_id(id: u16) -> f32 {
    match id {
        AIR => 0.0,
        SAND => 2.5,
        WATER => 1.0,
        HEAVY_WATER => 1.1,
        STONE => 3.0,
        CONCRETE => 2.8,
        STEEL => 7.8,
        LEAD => 11.3,
        GRAPHITE => 2.2,
        BORON => 2.3,
        U235 => 19.1,
        U238 => 19.1,
        PU239 => 19.8,
        PU240 => 19.8,
        DEPLETED_URANIUM => 19.1,
        FISSION_PRODUCTS => 5.0,
        TRITIUM => 0.8,
        DEUTERIUM => 0.8,
        TNT => 1.6,
        HYDROGEN => 0.07,
        LITHIUM => 0.5,
        HELIUM => 0.1,
        MOLTEN_FUEL => 10.0,
        FALLOUT => 2.0,
        NEUTRON_THERMAL => 0.001,
        NEUTRON_FAST => 0.001,
        GAMMA => 0.0,
        ALPHA => 0.01,
        BETA => 0.005,
        _ => 1.0,
    }
}

pub fn is_fissile(id: u16) -> bool {
    matches!(id, U235 | U238 | PU239 | PU240)
}

pub fn is_moderator(id: u16) -> bool {
    matches!(id, HEAVY_WATER | WATER | GRAPHITE)
}

pub fn is_radiation(id: u16) -> bool {
    matches!(id, NEUTRON_THERMAL | NEUTRON_FAST | GAMMA | ALPHA | BETA)
}

pub fn is_liquid(id: u16) -> bool {
    matches!(id, WATER | HEAVY_WATER)
}

pub fn is_gas(id: u16) -> bool {
    matches!(id, HYDROGEN | HELIUM | TRITIUM | DEUTERIUM)
}

pub fn penetration_depth(id: u16) -> u32 {
    // How many cells radiation can penetrate before absorption
    match id {
        GAMMA => 12,
        NEUTRON_FAST => 15,
        NEUTRON_THERMAL => 8,
        BETA => 4,
        ALPHA => 2,
        _ => 0,
    }
}
