//! Nuclear physics mechanics: fission, fusion, decay, criticality

use aura_lite_core::element_id::*;

/// Fission mechanics
pub mod fission {
    use super::*;

    pub fn fission_products(rng: &mut fastrand::Rng) -> Vec<u16> {
        // simplified: always fission products, but could be varied
        let _ = rng;
        vec![FISSION_PRODUCTS]
    }

    pub fn energy_released(element: u16) -> f32 {
        match element {
            U235 => 202.5, // MeV, scaled
            U238 => 205.0,
            PU239 => 207.0,
            PU240 => 200.0,
            _ => 0.0,
        }
    }

    pub fn neutron_count(element: u16, rng: &mut fastrand::Rng) -> u32 {
        match element {
            U235 => rng.u32(2..=3),
            PU239 => rng.u32(2..=4),
            U238 => rng.u32(2..=3),
            PU240 => rng.u32(2..=3),
            _ => rng.u32(2..=3),
        }
    }
}

/// Fusion mechanics
pub mod fusion {
    use super::*;

    pub const FUSION_THRESHOLD: u16 = 1500;

    pub fn can_fuse(a: u16, b: u16, temp: u16) -> bool {
        if temp < FUSION_THRESHOLD {
            return false;
        }
        (a == DEUTERIUM && b == TRITIUM) || (a == TRITIUM && b == DEUTERIUM)
    }

    pub fn products() -> (u16, Vec<u16>) {
        (HELIUM, vec![NEUTRON_FAST])
    }
}

/// Decay chains
pub mod decay {
    use super::*;

    pub struct DecayStep {
        pub parent: u16,
        pub daughter: u16,
        pub radiation: u16,
        pub half_life_ticks: u64,
    }

    pub fn decay_chain() -> Vec<DecayStep> {
        vec![
            DecayStep {
                parent: U238,
                daughter: DEPLETED_URANIUM,
                radiation: ALPHA,
                half_life_ticks: 2_000_000,
            },
            DecayStep {
                parent: U235,
                daughter: FISSION_PRODUCTS,
                radiation: ALPHA,
                half_life_ticks: 1_000_000,
            },
            DecayStep {
                parent: PU239,
                daughter: U235,
                radiation: ALPHA,
                half_life_ticks: 500_000,
            },
            DecayStep {
                parent: PU240,
                daughter: PU239,
                radiation: ALPHA,
                half_life_ticks: 400_000,
            },
            DecayStep {
                parent: TRITIUM,
                daughter: HELIUM,
                radiation: BETA,
                half_life_ticks: 100_000,
            },
        ]
    }

    pub fn daughter_for(parent: u16) -> u16 {
        match parent {
            U238 => DEPLETED_URANIUM,
            U235 => FISSION_PRODUCTS,
            PU239 => U235,
            PU240 => PU239,
            TRITIUM => HELIUM,
            _ => FALLOUT,
        }
    }
}

/// Criticality calculations
pub mod criticality {

    pub fn is_critical(mass_count: u32, threshold: u32) -> bool {
        mass_count >= threshold
    }

    pub fn criticality_factor(
        fissile_count: u32,
        moderator_count: u32,
        absorber_count: u32,
    ) -> f32 {
        // simplified k-effective
        let production = fissile_count as f32 * 2.5;
        let moderation = (moderator_count as f32 * 0.3).min(1.5);
        let absorption = absorber_count as f32 * 0.8;
        let escape = 0.2; // geometric loss
        let k = (production * (1.0 + moderation)) / (1.0 + absorption + escape);
        k / 100.0 // scaled
    }
}
