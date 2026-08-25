//! Shared nuclear-reaction parameters.
//!
//! Single source of truth used by the simulation kernel and (via re-exports)
//! the elements crate, so `ReactionTable` probabilities cannot drift from
//! the runtime behaviour.

use crate::element_id::*;
use serde::{Deserialize, Serialize};

/// Neutron kinetic-energy bin used by fission / moderation / absorption.
///
/// P4 adds the epithermal (resonance) group between fast and thermal. It is a
/// *queue-transient* state: moderation steps a neutron down one group per
/// moderator collision (fast → epithermal → thermal), while particles on the
/// grid remain fast or thermal (epithermal events spawn as fast particles).
/// Variant order is load-bearing for save compatibility: Thermal=0 and Fast=1
/// match the pre-P4 bincode encoding, so old saves decode unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeutronEnergy {
    Thermal,
    Fast,
    Epithermal,
}

/// One moderation collision steps a neutron down a single energy group.
/// Thermal neutrons have nothing to downscatter to.
pub fn moderator_downscatter(energy: NeutronEnergy) -> Option<NeutronEnergy> {
    match energy {
        NeutronEnergy::Fast => Some(NeutronEnergy::Epithermal),
        NeutronEnergy::Epithermal => Some(NeutronEnergy::Thermal),
        NeutronEnergy::Thermal => None,
    }
}

pub const FUSION_THRESHOLD: u16 = 1500;
pub const FUSION_PROBABILITY: f32 = 0.05;
pub const SPONTANEOUS_FISSION_PROB: f32 = 0.00001;
pub const MELTDOWN_TEMP: u16 = 2000;
pub const MELTDOWN_PROB: f32 = 0.01;
pub const BOIL_TEMP: u16 = 2500;
pub const BOIL_PROB: f32 = 0.05;
pub const TNT_IGNITE_TEMP: u16 = 500;
pub const LITHIUM_BREED_CHANCE: f32 = 0.40;
pub const AMBIENT_TEMP: u16 = 293;
pub const FISSION_SELF_HEAT: u16 = 500;
pub const FUSION_RADIUS_HEAT: u16 = 800;

/// Base fission probability before the temperature modifier.
/// Epithermal sits between fast and thermal — the resonance region, where
/// U-238's threshold behaviour is already visible but the fissile isotopes are
/// not yet at their thermal peaks.
pub fn fission_base_probability(element_id: u16, energy: NeutronEnergy) -> f32 {
    match element_id {
        U235 => match energy {
            NeutronEnergy::Thermal => 0.85,
            NeutronEnergy::Epithermal => 0.55,
            NeutronEnergy::Fast => 0.35,
        },
        PU239 => match energy {
            NeutronEnergy::Thermal => 0.90,
            NeutronEnergy::Epithermal => 0.60,
            NeutronEnergy::Fast => 0.40,
        },
        U238 => match energy {
            NeutronEnergy::Thermal => 0.02,
            NeutronEnergy::Epithermal => 0.12,
            NeutronEnergy::Fast => 0.25,
        },
        PU240 => match energy {
            NeutronEnergy::Thermal => 0.10,
            NeutronEnergy::Epithermal => 0.18,
            NeutronEnergy::Fast => 0.30,
        },
        MOLTEN_FUEL => match energy {
            NeutronEnergy::Thermal => 0.50,
            NeutronEnergy::Epithermal => 0.38,
            NeutronEnergy::Fast => 0.30,
        },
        _ => 0.0,
    }
}

/// U-238 neutron-capture (breeding) chance, applied when an incident neutron
/// fails to fission the nucleus: U-238 + n → Pu-239. This is the real path to
/// plutonium in thermal reactors, and it makes a breeder cycle possible in the
/// toy: U-238 breeds Pu-239, Pu-239 is fissile (0.90 thermal), fissions, and
/// its neutrons keep the cycle going.
pub fn u238_capture_chance(energy: NeutronEnergy) -> f32 {
    match energy {
        NeutronEnergy::Thermal => 0.25,
        NeutronEnergy::Epithermal => 0.20,
        NeutronEnergy::Fast => 0.15,
    }
}

/// Doppler temperature coefficient (per Kelvin above ambient). Negative for
/// fissile isotopes: a hotter fuel lattice broadens resonance absorption, so
/// reactivity falls — the feedback real reactors rely on to stay critical.
/// Only consulted under the `thermal-pde` feature (P3).
pub fn temperature_coefficient(element_id: u16) -> f32 {
    match element_id {
        U235 => -0.0008,
        U238 => -0.0006,
        PU239 => -0.0005,
        PU240 => -0.0007,
        MOLTEN_FUEL => -0.0004,
        _ => 0.0,
    }
}

/// Temperature-adjusted fission probability, clamped to `[0, 0.95]`.
pub fn fission_probability(element_id: u16, energy: NeutronEnergy, temp: u16) -> f32 {
    let base = fission_base_probability(element_id, energy);
    #[cfg(not(feature = "thermal-pde"))]
    {
        // MVP model: a mild *positive* temperature coefficient (hotter = slightly
        // more reactive). Kept as the default so replay and the golden corpus hold.
        let temp_factor = 1.0 + ((temp as f32 - AMBIENT_TEMP as f32) / 1000.0).clamp(-0.5, 1.0);
        (base * temp_factor).clamp(0.0, 0.95)
    }
    #[cfg(feature = "thermal-pde")]
    {
        // P3: Doppler feedback dominates — reactivity falls as fuel temperature
        // rises, so a chain reaction self-limits instead of running to meltdown.
        let coeff = temperature_coefficient(element_id);
        let excess = (temp as f32 - AMBIENT_TEMP as f32).max(0.0);
        (base * (1.0 + coeff * excess)).clamp(0.0, 0.95)
    }
}

pub fn neutron_count(element_id: u16, rng: &mut fastrand::Rng) -> u32 {
    match element_id {
        PU239 => rng.u32(2..=4),
        _ => rng.u32(2..=3),
    }
}

pub fn energy_released_mev(element_id: u16) -> f32 {
    match element_id {
        U235 => 202.5,
        U238 => 205.0,
        PU239 => 207.0,
        PU240 => 200.0,
        MOLTEN_FUEL => 180.0,
        _ => 0.0,
    }
}

pub fn half_life_ticks(element_id: u16) -> u64 {
    match element_id {
        U235 => 1_000_000,
        U238 => 2_000_000,
        PU239 => 500_000,
        PU240 => 400_000,
        TRITIUM => 100_000,
        XENON => 8_000,
        IODINE => 2_400,
        _ => 0,
    }
}

pub fn decay_daughter(element_id: u16) -> u16 {
    match element_id {
        U235 => FISSION_PRODUCTS,
        U238 => DEPLETED_URANIUM,
        PU239 => U235,
        PU240 => PU239,
        TRITIUM => HELIUM,
        XENON => AIR,
        IODINE => XENON,
        _ => FALLOUT,
    }
}

pub fn decay_radiation(element_id: u16) -> u16 {
    match element_id {
        U235 | U238 | PU239 | PU240 => ALPHA,
        TRITIUM => BETA,
        XENON | IODINE => AIR, // poison chain, no extra radiation
        _ => GAMMA,
    }
}

pub fn moderator_thermalize_chance(id: u16) -> f32 {
    match id {
        HEAVY_WATER => 0.5,
        WATER => 0.4,
        GRAPHITE => 0.3,
        _ => 0.0,
    }
}

pub fn absorber_chance(id: u16, energy: NeutronEnergy) -> f32 {
    match (id, energy) {
        (BORON, NeutronEnergy::Thermal) => 0.8,
        (BORON, NeutronEnergy::Epithermal) => 0.7,
        (BORON, NeutronEnergy::Fast) => 0.6,
        (CONTROL_ROD, NeutronEnergy::Thermal) => 0.92,
        (CONTROL_ROD, NeutronEnergy::Epithermal) => 0.80,
        (CONTROL_ROD, NeutronEnergy::Fast) => 0.70,
        (XENON, NeutronEnergy::Thermal) => 0.95,
        (XENON, NeutronEnergy::Epithermal) => 0.72,
        (XENON, NeutronEnergy::Fast) => 0.55,
        // I-135 is a real (if weaker than Xe-135) neutron absorber. Without this
        // arm, `absorber_chance(IODINE, _)` returned 0, so the dedicated iodine
        // branch in `process_neutron_queue` never absorbed anything — yet iodine
        // was still counted toward the absorber total used for k-effective, making
        // the criticality estimate inconsistent with actual reactivity.
        (IODINE, NeutronEnergy::Thermal) => 0.35,
        (IODINE, NeutronEnergy::Epithermal) => 0.22,
        (IODINE, NeutronEnergy::Fast) => 0.12,
        _ => 0.0,
    }
}

/// Scaled k-effective estimate used for the HUD / info panel.
pub fn criticality_factor(fissile_count: u32, moderator_count: u32, absorber_count: u32) -> f32 {
    let production = fissile_count as f32 * 2.5;
    let moderation = (moderator_count as f32 * 0.3).min(1.5);
    let absorption = absorber_count as f32 * 0.8;
    let escape = 0.2;
    let k = (production * (1.0 + moderation)) / (1.0 + absorption + escape);
    // Scale so a small pile sits near 0.2–1.5 instead of huge raw numbers.
    (k / 80.0).clamp(0.0, 3.5)
}

/// Extra prompt neutrons when the pile is supercritical.
pub fn k_extra_neutrons(k_eff: f32, rng: &mut fastrand::Rng) -> u32 {
    if k_eff <= 1.0 {
        return 0;
    }
    let p = ((k_eff - 1.0) * 0.45).clamp(0.0, 0.7);
    if rng.f32() < p {
        1
    } else {
        0
    }
}

pub fn spontaneous_fission_prob(k_eff: f32) -> f32 {
    SPONTANEOUS_FISSION_PROB * (0.4 + k_eff).clamp(0.2, 2.5)
}

pub fn is_critical(mass_count: u32, threshold: u32) -> bool {
    mass_count >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u235_prefers_thermal_neutrons() {
        let thermal = fission_base_probability(U235, NeutronEnergy::Thermal);
        let fast = fission_base_probability(U235, NeutronEnergy::Fast);
        assert!(thermal > fast);
        assert!((thermal - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn u238_is_fast_fission_threshold() {
        let thermal = fission_base_probability(U238, NeutronEnergy::Thermal);
        let fast = fission_base_probability(U238, NeutronEnergy::Fast);
        assert!(fast > thermal);
        assert!(thermal < 0.05);
    }

    #[test]
    fn tritium_decays_to_helium_via_beta() {
        assert_eq!(decay_daughter(TRITIUM), HELIUM);
        assert_eq!(decay_radiation(TRITIUM), BETA);
        assert!(half_life_ticks(TRITIUM) > 0);
    }

    #[test]
    fn heavy_water_moderates_better_than_graphite() {
        assert!(moderator_thermalize_chance(HEAVY_WATER) > moderator_thermalize_chance(GRAPHITE));
    }

    #[test]
    fn xenon_decays_to_air_without_radiation() {
        assert_eq!(decay_daughter(XENON), AIR);
        assert_eq!(decay_radiation(XENON), AIR);
        assert_eq!(half_life_ticks(XENON), 8_000);
    }

    #[test]
    fn control_rods_and_xenon_absorb() {
        assert!(
            absorber_chance(CONTROL_ROD, NeutronEnergy::Thermal)
                > absorber_chance(BORON, NeutronEnergy::Thermal)
        );
        assert!(absorber_chance(XENON, NeutronEnergy::Thermal) > 0.9);
    }

    #[test]
    fn iodine_decays_to_xenon() {
        assert_eq!(decay_daughter(IODINE), XENON);
        assert_eq!(decay_radiation(IODINE), AIR);
        assert!(half_life_ticks(IODINE) < half_life_ticks(XENON));
        assert!(
            absorber_chance(IODINE, NeutronEnergy::Thermal)
                < absorber_chance(XENON, NeutronEnergy::Thermal)
        );
    }
}
