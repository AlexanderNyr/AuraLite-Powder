use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReactionOutcome {
    pub probability: f32,
    pub products: Vec<u16>, // resulting element IDs spawned
    pub energy_change: f32,
    pub particle_spawns: Vec<u16>, // neutrons etc
    pub temperature_delta: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionPair(pub u16, pub u16);

impl ReactionPair {
    pub fn new(a: u16, b: u16) -> Self {
        // order independent? keep sorted for lookup
        if a <= b {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }
}

pub struct ReactionTable {
    pub map: HashMap<ReactionPair, Vec<ReactionOutcome>>,
}

impl ReactionTable {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, a: u16, b: u16, outcome: ReactionOutcome) {
        let key = ReactionPair::new(a, b);
        self.map.entry(key).or_default().push(outcome);
    }

    pub fn get(&self, a: u16, b: u16) -> Option<&Vec<ReactionOutcome>> {
        let key = ReactionPair::new(a, b);
        self.map.get(&key)
    }

    pub fn build_default() -> Self {
        let mut table = Self::new();
        use aura_lite_core::element_id::*;
        use aura_lite_core::reactions::{self, NeutronEnergy};

        // Fission reactions
        table.insert(
            U235,
            NEUTRON_THERMAL,
            ReactionOutcome {
                probability: reactions::fission_base_probability(U235, NeutronEnergy::Thermal),
                products: vec![FISSION_PRODUCTS],
                energy_change: 200.0,
                particle_spawns: vec![NEUTRON_FAST, NEUTRON_FAST, GAMMA],
                temperature_delta: 500,
            },
        );
        table.insert(
            U235,
            NEUTRON_FAST,
            ReactionOutcome {
                probability: reactions::fission_base_probability(U235, NeutronEnergy::Fast),
                products: vec![FISSION_PRODUCTS],
                energy_change: 200.0,
                particle_spawns: vec![NEUTRON_FAST, NEUTRON_FAST],
                temperature_delta: 400,
            },
        );
        table.insert(
            PU239,
            NEUTRON_THERMAL,
            ReactionOutcome {
                probability: reactions::fission_base_probability(PU239, NeutronEnergy::Thermal),
                products: vec![FISSION_PRODUCTS],
                energy_change: 210.0,
                particle_spawns: vec![NEUTRON_FAST, NEUTRON_FAST, NEUTRON_FAST],
                temperature_delta: 550,
            },
        );
        table.insert(
            PU239,
            NEUTRON_FAST,
            ReactionOutcome {
                probability: reactions::fission_base_probability(PU239, NeutronEnergy::Fast),
                products: vec![FISSION_PRODUCTS],
                energy_change: 210.0,
                particle_spawns: vec![NEUTRON_FAST, NEUTRON_FAST],
                temperature_delta: 450,
            },
        );
        table.insert(
            U238,
            NEUTRON_FAST,
            ReactionOutcome {
                probability: reactions::fission_base_probability(U238, NeutronEnergy::Fast),
                products: vec![FISSION_PRODUCTS],
                energy_change: 190.0,
                particle_spawns: vec![NEUTRON_FAST, NEUTRON_FAST],
                temperature_delta: 350,
            },
        );

        // Fusion D+T
        table.insert(
            DEUTERIUM,
            TRITIUM,
            ReactionOutcome {
                probability: reactions::FUSION_PROBABILITY, // requires high temp check elsewhere
                products: vec![HELIUM],
                energy_change: 500.0,
                particle_spawns: vec![NEUTRON_FAST],
                temperature_delta: 1200,
            },
        );

        // Moderation: fast neutron + water -> thermal. Since P4 the runtime
        // moderates in TWO steps (fast -> epithermal -> thermal, one group per
        // collision); the table records the net effect. The epithermal group is
        // a queue-energy, not an element, so it has no row here.
        table.insert(
            NEUTRON_FAST,
            WATER,
            ReactionOutcome {
                probability: reactions::moderator_thermalize_chance(WATER),
                products: vec![WATER],
                energy_change: -5.0,
                particle_spawns: vec![NEUTRON_THERMAL],
                temperature_delta: 10,
            },
        );
        table.insert(
            NEUTRON_FAST,
            HEAVY_WATER,
            ReactionOutcome {
                probability: reactions::moderator_thermalize_chance(HEAVY_WATER),
                products: vec![HEAVY_WATER],
                energy_change: -3.0,
                particle_spawns: vec![NEUTRON_THERMAL],
                temperature_delta: 5,
            },
        );
        table.insert(
            NEUTRON_FAST,
            GRAPHITE,
            ReactionOutcome {
                probability: reactions::moderator_thermalize_chance(GRAPHITE),
                products: vec![GRAPHITE],
                energy_change: -4.0,
                particle_spawns: vec![NEUTRON_THERMAL],
                temperature_delta: 8,
            },
        );

        // Absorption: boron absorbs neutrons
        table.insert(
            BORON,
            NEUTRON_THERMAL,
            ReactionOutcome {
                probability: 0.8,
                products: vec![FALLOUT],
                energy_change: 2.0,
                particle_spawns: vec![ALPHA],
                temperature_delta: 50,
            },
        );
        table.insert(
            BORON,
            NEUTRON_FAST,
            ReactionOutcome {
                probability: reactions::absorber_chance(BORON, NeutronEnergy::Fast),
                products: vec![FALLOUT],
                energy_change: 2.0,
                particle_spawns: vec![ALPHA],
                temperature_delta: 50,
            },
        );

        // Lithium breeding: Li + n -> T + He
        table.insert(
            LITHIUM,
            NEUTRON_THERMAL,
            ReactionOutcome {
                probability: reactions::LITHIUM_BREED_CHANCE,
                products: vec![TRITIUM],
                energy_change: 4.8,
                particle_spawns: vec![HELIUM],
                temperature_delta: 50,
            },
        );
        table.insert(
            LITHIUM,
            NEUTRON_FAST,
            ReactionOutcome {
                probability: reactions::LITHIUM_BREED_CHANCE,
                products: vec![TRITIUM],
                energy_change: 4.8,
                particle_spawns: vec![HELIUM],
                temperature_delta: 50,
            },
        );

        table
    }
}

impl Default for ReactionTable {
    fn default() -> Self {
        Self::build_default()
    }
}
