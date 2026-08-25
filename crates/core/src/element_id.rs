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
pub const STEAM: u16 = 29;
pub const ICE: u16 = 30;
pub const XENON: u16 = 31;
pub const FIRE: u16 = 32;
pub const WOOD: u16 = 33;
pub const ACID: u16 = 34;
pub const WIRE: u16 = 35;
pub const HEATER: u16 = 36;
pub const PUMP: u16 = 37;
pub const PIPE: u16 = 38;
pub const SENSOR: u16 = 39;
pub const CONTROL_ROD: u16 = 40;
pub const SLAG: u16 = 41;
pub const COAL: u16 = 42;
pub const SPARK: u16 = 43;
pub const FILTER: u16 = 44;
pub const IODINE: u16 = 45;
pub const PIPE_WATER: u16 = 46;
pub const PIPE_STEAM: u16 = 47;
pub const OIL: u16 = 48; // flammable liquid, floats on water
pub const MERCURY: u16 = 49; // very dense liquid, sinks through water

pub const MAX_ELEMENT_ID: u16 = 49;

pub fn is_valid_id(id: u16) -> bool {
    id <= MAX_ELEMENT_ID
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Air,
    Sand, // granular
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
        WATER | HEAVY_WATER | ACID | OIL | MERCURY => ElementKind::Liquid,
        STONE | CONCRETE | STEEL | LEAD | GRAPHITE | BORON | ICE | WOOD | WIRE | HEATER | PUMP
        | PIPE | PIPE_WATER | PIPE_STEAM | SENSOR | CONTROL_ROD | FILTER => ElementKind::Solid,
        U235 | U238 | PU239 | PU240 | DEPLETED_URANIUM | FISSION_PRODUCTS | LITHIUM | FALLOUT
        | TNT | SLAG | COAL => ElementKind::Sand,
        TRITIUM | DEUTERIUM | HELIUM | HYDROGEN | STEAM | XENON | IODINE => ElementKind::Gas,
        MOLTEN_FUEL => ElementKind::Molten,
        FIRE | SPARK => ElementKind::Radiation,
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
        STEAM => 0.05,
        ICE => 0.92,
        XENON => 0.06,
        FIRE => 0.02,
        WOOD => 0.7,
        ACID => 1.2,
        OIL => 0.85,
        MERCURY => 13.5,
        WIRE => 8.0,
        HEATER => 7.5,
        PUMP => 6.0,
        PIPE => 5.5,
        SENSOR => 4.0,
        CONTROL_ROD => 2.5,
        SLAG => 3.2,
        COAL => 1.3,
        SPARK => 0.001,
        FILTER => 4.5,
        IODINE => 0.05,
        PIPE_WATER => 5.6,
        PIPE_STEAM => 5.4,
        NEUTRON_THERMAL => 0.001,
        NEUTRON_FAST => 0.001,
        GAMMA => 0.0,
        ALPHA => 0.01,
        BETA => 0.005,
        _ => 1.0,
    }
}

pub fn is_fissile(id: u16) -> bool {
    matches!(id, U235 | U238 | PU239 | PU240 | MOLTEN_FUEL)
}

pub fn is_moderator(id: u16) -> bool {
    matches!(id, HEAVY_WATER | WATER | GRAPHITE | PIPE_WATER)
}

pub fn is_radiation(id: u16) -> bool {
    matches!(
        id,
        NEUTRON_THERMAL | NEUTRON_FAST | GAMMA | ALPHA | BETA | SPARK
    )
}

pub fn is_liquid(id: u16) -> bool {
    matches!(id, WATER | HEAVY_WATER | MOLTEN_FUEL | ACID | OIL | MERCURY)
}

pub fn is_gas(id: u16) -> bool {
    matches!(
        id,
        HYDROGEN | HELIUM | TRITIUM | DEUTERIUM | STEAM | XENON | FIRE | IODINE
    )
}

/// Immovable structural material (walls, shielding, devices).
pub fn is_static_solid(id: u16) -> bool {
    matches!(
        id,
        STONE
            | CONCRETE
            | STEEL
            | LEAD
            | GRAPHITE
            | BORON
            | ICE
            | WOOD
            | WIRE
            | HEATER
            | PUMP
            | PIPE
            | PIPE_WATER
            | PIPE_STEAM
            | SENSOR
            | CONTROL_ROD
            | FILTER
    )
}

pub fn is_pipe(id: u16) -> bool {
    matches!(id, PIPE | PIPE_WATER | PIPE_STEAM)
}

pub fn pipe_payload(id: u16) -> Option<u16> {
    match id {
        PIPE_WATER => Some(WATER),
        PIPE_STEAM => Some(STEAM),
        _ => None,
    }
}

pub fn pipe_with(payload: u16) -> u16 {
    match payload {
        STEAM | HYDROGEN | HELIUM | XENON | IODINE => PIPE_STEAM,
        _ => PIPE_WATER,
    }
}

/// Granular material that piles with an angle of repose.
pub fn is_powder(id: u16) -> bool {
    matches!(
        id,
        SAND | FALLOUT
            | FISSION_PRODUCTS
            | LITHIUM
            | TNT
            | U235
            | U238
            | PU239
            | PU240
            | DEPLETED_URANIUM
            | SLAG
            | COAL
    )
}

pub fn is_flammable(id: u16) -> bool {
    matches!(id, WOOD | COAL | TNT | HYDROGEN | OIL)
}

pub fn is_conductive(id: u16) -> bool {
    matches!(id, WIRE | HEATER | SENSOR | STEEL | SPARK | PUMP)
}

/// The elements `devices::step_devices` actually acts on (heaters, pumps,
/// fire and its fuels, the electrical family, steam). Used by P2c's
/// classify-once gating: when none of these exist, the whole device pass
/// (including its full-grid snapshots) is skipped.
pub fn is_device_element(id: u16) -> bool {
    matches!(
        id,
        HEATER
            | PUMP
            | FIRE
            | ACID
            | WOOD
            | COAL
            | HYDROGEN
            | SPARK
            | WIRE
            | SENSOR
            | FILTER
            | CONTROL_ROD
            | STEAM
    )
}

pub fn is_device(id: u16) -> bool {
    matches!(
        id,
        HEATER | PUMP | PIPE | PIPE_WATER | PIPE_STEAM | SENSOR | WIRE | CONTROL_ROD
    )
}

pub fn is_fluid(id: u16) -> bool {
    is_liquid(id) || is_gas(id)
}

/// How many extra horizontal cells a liquid may travel in one tick (low = viscous).
pub fn flow_steps(id: u16) -> u32 {
    match id {
        WATER | ACID => 4,
        HEAVY_WATER => 3,
        OIL => 3,
        MERCURY => 2,
        MOLTEN_FUEL => 1,
        STEAM | HYDROGEN | HELIUM | FIRE | XENON | IODINE => 3,
        TRITIUM | DEUTERIUM => 2,
        _ => 0,
    }
}

/// Probability a powder slides down a diagonal this tick (angle of repose).
pub fn repose_slide(id: u16) -> f32 {
    match id {
        SAND | FALLOUT => 0.62,
        FISSION_PRODUCTS | LITHIUM | TNT => 0.45,
        U235 | U238 | PU239 | PU240 | DEPLETED_URANIUM => 0.28,
        _ => 0.4,
    }
}

/// Thermal conductivity used by the heat solver (0..1).
pub fn conductivity(id: u16) -> f32 {
    match id {
        AIR => 0.015,
        STEAM | HYDROGEN | HELIUM | TRITIUM | DEUTERIUM => 0.03,
        WATER | HEAVY_WATER => 0.14,
        ICE => 0.18,
        SAND | FALLOUT | FISSION_PRODUCTS | SLAG | COAL => 0.06,
        CONCRETE | STONE | GRAPHITE | WOOD => 0.10,
        BORON | LITHIUM | TNT | CONTROL_ROD => 0.08,
        STEEL | WIRE | HEATER => 0.42,
        LEAD => 0.30,
        PIPE | PUMP | SENSOR | FILTER => 0.18,
        ACID => 0.12,
        OIL => 0.10,
        MERCURY => 0.60,
        FIRE | XENON | IODINE => 0.04,
        U235 | U238 | PU239 | PU240 | DEPLETED_URANIUM | MOLTEN_FUEL => 0.16,
        _ => 0.05,
    }
}

/// Terminal fall speed in cells / tick.
pub fn max_fall_speed(id: u16) -> i8 {
    match id {
        WATER | HEAVY_WATER | OIL | MERCURY => 2,
        MOLTEN_FUEL => 2,
        SAND | FALLOUT | FISSION_PRODUCTS | TNT | LITHIUM => 3,
        U235 | U238 | PU239 | PU240 | DEPLETED_URANIUM => 3,
        _ => 1,
    }
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
