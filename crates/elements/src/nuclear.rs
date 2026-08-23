//! Nuclear physics helpers. Runtime values live in `aura_lite_core::reactions`
//! so the simulation and this crate cannot drift apart.

use aura_lite_core::element_id::*;
use aura_lite_core::reactions;

/// Fission mechanics
pub mod fission {
    use super::*;

    pub fn fission_products(rng: &mut fastrand::Rng) -> Vec<u16> {
        let _ = rng;
        vec![FISSION_PRODUCTS]
    }

    pub fn energy_released(element: u16) -> f32 {
        reactions::energy_released_mev(element)
    }

    pub fn neutron_count(element: u16, rng: &mut fastrand::Rng) -> u32 {
        reactions::neutron_count(element, rng)
    }
}

/// Fusion mechanics
pub mod fusion {
    use super::*;

    pub const FUSION_THRESHOLD: u16 = reactions::FUSION_THRESHOLD;

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
                daughter: reactions::decay_daughter(U238),
                radiation: reactions::decay_radiation(U238),
                half_life_ticks: reactions::half_life_ticks(U238),
            },
            DecayStep {
                parent: U235,
                daughter: reactions::decay_daughter(U235),
                radiation: reactions::decay_radiation(U235),
                half_life_ticks: reactions::half_life_ticks(U235),
            },
            DecayStep {
                parent: PU239,
                daughter: reactions::decay_daughter(PU239),
                radiation: reactions::decay_radiation(PU239),
                half_life_ticks: reactions::half_life_ticks(PU239),
            },
            DecayStep {
                parent: PU240,
                daughter: reactions::decay_daughter(PU240),
                radiation: reactions::decay_radiation(PU240),
                half_life_ticks: reactions::half_life_ticks(PU240),
            },
            DecayStep {
                parent: TRITIUM,
                daughter: reactions::decay_daughter(TRITIUM),
                radiation: reactions::decay_radiation(TRITIUM),
                half_life_ticks: reactions::half_life_ticks(TRITIUM),
            },
        ]
    }

    pub fn daughter_for(parent: u16) -> u16 {
        reactions::decay_daughter(parent)
    }
}

/// Criticality calculations
pub mod criticality {
    use super::*;

    pub fn is_critical(mass_count: u32, threshold: u32) -> bool {
        reactions::is_critical(mass_count, threshold)
    }

    pub fn criticality_factor(
        fissile_count: u32,
        moderator_count: u32,
        absorber_count: u32,
    ) -> f32 {
        reactions::criticality_factor(fissile_count, moderator_count, absorber_count)
    }
}
