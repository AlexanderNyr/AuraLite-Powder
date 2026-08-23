use aura_lite_core::element_id::*;

/// Map element ID to RGBA color. The elements registry is the source of truth.
pub fn color_for_element(id: u16) -> [u8; 4] {
    aura_lite_elements::registry::color_for_id(id)
}

/// Temperature to color overlay (heatmap).
pub fn temperature_to_color(temp: u16) -> [u8; 4] {
    let t = temp as f32;
    let normalized = ((t - 293.0) / (3000.0 - 293.0)).clamp(0.0, 1.0);
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

/// Kept so existing match-style call sites that only have an id still compile
/// if the registry is unavailable (tests without elements still link it).
#[allow(dead_code)]
pub fn fallback_color(id: u16) -> [u8; 4] {
    match id {
        AIR => [0, 0, 0, 0],
        SAND => [194, 178, 128, 255],
        WATER => [64, 164, 223, 255],
        _ => [255, 0, 255, 255],
    }
}
