use aura_lite_core::element_id::*;

/// Map element ID to RGBA color [r,g,b,a]
pub fn color_for_element(id: u16) -> [u8; 4] {
    match id {
        AIR => [0, 0, 0, 0],
        SAND => [194, 178, 128, 255],
        WATER => [64, 164, 223, 255],
        STONE => [120, 120, 120, 255],
        U235 => [80, 200, 60, 255],
        U238 => [70, 170, 50, 255],
        PU239 => [200, 60, 60, 255],
        PU240 => [180, 40, 40, 255],
        HEAVY_WATER => [80, 180, 230, 255],
        GRAPHITE => [50, 50, 50, 255],
        LEAD => [90, 90, 100, 255],
        CONCRETE => [160, 160, 155, 255],
        STEEL => [170, 170, 180, 255],
        NEUTRON_THERMAL => [255, 255, 150, 255],
        NEUTRON_FAST => [255, 80, 80, 255],
        GAMMA => [255, 255, 0, 255],
        ALPHA => [255, 120, 0, 255],
        BETA => [80, 200, 255, 255],
        DEPLETED_URANIUM => [60, 120, 40, 255],
        FISSION_PRODUCTS => [100, 180, 40, 255],
        TRITIUM => [150, 200, 255, 255],
        DEUTERIUM => [120, 220, 255, 255],
        TNT => [220, 50, 50, 255],
        HYDROGEN => [200, 230, 255, 200],
        LITHIUM => [200, 200, 200, 255],
        HELIUM => [255, 254, 200, 200],
        MOLTEN_FUEL => [255, 100, 0, 255],
        FALLOUT => [80, 80, 30, 255],
        BORON => [100, 90, 80, 255],
        _ => [255, 0, 255, 255],
    }
}

/// Temperature to color overlay (heatmap)
pub fn temperature_to_color(temp: u16) -> [u8; 4] {
    let t = temp as f32;
    // baseline 293, max 3000
    let normalized = ((t - 293.0) / (3000.0 - 293.0)).clamp(0.0, 1.0);
    // glow: black -> red -> yellow -> white
    if normalized < 0.33 {
        let f = normalized / 0.33;
        [(f * 255.0) as u8, 0, 0, (f * 200.0) as u8]
    } else if normalized < 0.66 {
        let f = (normalized - 0.33) / 0.33;
        [255, (f * 255.0) as u8, 0, (100.0 + f * 100.0) as u8]
    } else {
        let f = (normalized - 0.66) / 0.34;
        [255, 255, (f * 255.0) as u8, 200]
    }
}
