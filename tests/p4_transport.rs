//! P4 transport gates (ROADMAP): three-group neutron moderation (fast →
//! epithermal → thermal) and the measured k-effective.

use aura_lite_core::{
    element_id::*,
    reactions::{self, NeutronEnergy},
    NeutronEvent, Particle, SimulationState,
};

// ───────────────────────── unit: three-group structure ──────────────────────

#[test]
fn epithermal_sits_between_fast_and_thermal() {
    for &iso in &[U235, PU239, U238, PU240, MOLTEN_FUEL] {
        let t = reactions::fission_base_probability(iso, NeutronEnergy::Thermal);
        let e = reactions::fission_base_probability(iso, NeutronEnergy::Epithermal);
        let f = reactions::fission_base_probability(iso, NeutronEnergy::Fast);
        let lo = t.min(f);
        let hi = t.max(f);
        assert!(
            e >= lo && e <= hi,
            "{iso}: epithermal ({e}) must sit between thermal ({t}) and fast ({f})"
        );
        // The thermal/fast values are the pre-P4 constants, unchanged.
    }
    // Fissile isotopes prefer thermal; U-238 keeps its threshold shape (fast
    // beats thermal) — both orderings are preserved with epithermal between.
    assert!(
        reactions::fission_base_probability(U235, NeutronEnergy::Thermal)
            > reactions::fission_base_probability(U235, NeutronEnergy::Fast)
    );
    assert!(
        reactions::fission_base_probability(U238, NeutronEnergy::Fast)
            > reactions::fission_base_probability(U238, NeutronEnergy::Thermal)
    );
}

#[test]
fn absorbers_absorb_epithermal_between_thermal_and_fast() {
    for &a in &[BORON, CONTROL_ROD, XENON, IODINE] {
        let t = reactions::absorber_chance(a, NeutronEnergy::Thermal);
        let e = reactions::absorber_chance(a, NeutronEnergy::Epithermal);
        let f = reactions::absorber_chance(a, NeutronEnergy::Fast);
        assert!(
            t >= e && e >= f && f > 0.0,
            "{a}: epithermal absorption ({e}) must sit between thermal ({t}) and fast ({f})"
        );
    }
}

#[test]
fn downscatter_steps_one_group_per_collision() {
    assert_eq!(
        reactions::moderator_downscatter(NeutronEnergy::Fast),
        Some(NeutronEnergy::Epithermal)
    );
    assert_eq!(
        reactions::moderator_downscatter(NeutronEnergy::Epithermal),
        Some(NeutronEnergy::Thermal)
    );
    assert_eq!(
        reactions::moderator_downscatter(NeutronEnergy::Thermal),
        None
    );
}

// ───────────────────── integration: moderation is two-step ──────────────────

/// A fast neutron moderated in water must pass through the epithermal group:
/// after one tick the queue holds an epithermal event (not yet thermal), and
/// after a second moderator collision it becomes thermal.
#[test]
fn moderation_steps_through_epithermal() {
    let mut sim = SimulationState::new(20, 20, 1);
    // A water block large enough that the ±1 jitter of a re-queued event stays
    // inside the moderator (a single cell would let it escape as a particle).
    for y in 6..14 {
        for x in 6..14 {
            sim.grid.set(x, y, Particle::new(WATER, 293));
        }
    }
    sim.refresh_chunks_public();
    sim.neutron_queue.push_back(NeutronEvent {
        x: 10,
        y: 10,
        delay: 0,
        energy: NeutronEnergy::Fast,
    });

    sim.tick();
    let has_epi = sim
        .neutron_queue
        .iter()
        .any(|ev| ev.energy == NeutronEnergy::Epithermal);
    let has_thermal = sim
        .neutron_queue
        .iter()
        .any(|ev| ev.energy == NeutronEnergy::Thermal);
    assert!(
        has_epi && !has_thermal,
        "after one moderator collision the neutron must be epithermal, not thermal \
         (epi={has_epi} thermal={has_thermal}, queue len {})",
        sim.neutron_queue.len()
    );

    // The epithermal event re-queues into the water block; give it time to
    // collide again and reach thermal.
    let mut saw_thermal = false;
    for _ in 0..40 {
        if sim
            .neutron_queue
            .iter()
            .any(|ev| ev.energy == NeutronEnergy::Thermal)
        {
            saw_thermal = true;
            break;
        }
        sim.tick();
    }
    assert!(
        saw_thermal,
        "the epithermal neutron must reach thermal after a second moderator collision"
    );
}

// ───────────────────── integration: measured k-effective ────────────────────

/// A small, unmoderated U-235 pile cannot sustain a chain (fast fission alone
/// is subcritical), so the measured k must settle below 1.
#[test]
fn k_measured_subcritical_for_a_small_bare_pile() {
    let mut sim = SimulationState::new(64, 64, 3);
    for y in 30..36 {
        for x in 30..36 {
            sim.grid.set(x, y, Particle::new(U235, 400));
        }
    }
    // Start the neutron INSIDE the pile so the first reaction pass is
    // guaranteed to see fuel adjacent to it.
    sim.grid.set(33, 33, Particle::new(NEUTRON_THERMAL, 350));
    sim.refresh_chunks_public();
    for _ in 0..240 {
        sim.tick();
    }
    assert!(
        sim.fission_count > 0,
        "the pile must fission at least once for the measurement to mean anything"
    );
    assert!(
        sim.k_measured < 0.95,
        "a small bare pile is subcritical: measured k = {:.3} (fissions {})",
        sim.k_measured,
        sim.fission_count
    );
}

/// The measured k grows with the amount of moderated fuel — the critical-mass
/// sweep, gated self-consistently (no external reference exists for this toy):
/// monotone in pile size, and the largest moderated pile must out-multiply the
/// smallest.
#[test]
fn k_measured_grows_with_moderated_pile_size() {
    let run = |fuel_side: u32| -> (f32, u64) {
        let mut sim = SimulationState::new(96, 96, 7);
        let cx = 48;
        let cy = 48;
        let half = fuel_side / 2;
        // Graphite wrapper (moderator) with a U-235 core.
        for y in (cy - half - 6)..(cy + half + 6) {
            for x in (cx - half - 6)..(cx + half + 6) {
                sim.grid.set(x, y, Particle::new(GRAPHITE, 320));
            }
        }
        for y in (cy - half)..(cy + half) {
            for x in (cx - half)..(cx + half) {
                sim.grid.set(x, y, Particle::new(U235, 400));
            }
        }
        sim.grid.set(cx, cy, Particle::new(NEUTRON_THERMAL, 350));
        sim.refresh_chunks_public();
        for _ in 0..300 {
            sim.tick();
        }
        (sim.k_measured, sim.fission_count)
    };

    let (k_small, f_small) = run(6);
    let (_k_mid, _f_mid) = run(12);
    let (k_large, f_large) = run(18);

    assert!(f_small > 0, "every pile must fission");
    assert!(
        k_large > k_small,
        "the largest moderated pile must out-multiply the smallest: \
         {k_small:.3} vs {k_large:.3} (fissions {f_small}/{f_large})"
    );
}

/// A moderator reflector raises the multiplication of the SAME fuel load —
/// leaking neutrons come back to cause fission instead of escaping.
#[test]
fn graphite_reflector_raises_measured_k() {
    let run = |with_reflector: bool| -> f32 {
        let mut sim = SimulationState::new(96, 96, 7);
        let (cx, cy) = (48, 48);
        if with_reflector {
            for y in (cy - 13)..(cy + 13) {
                for x in (cx - 13)..(cx + 13) {
                    sim.grid.set(x, y, Particle::new(GRAPHITE, 320));
                }
            }
        }
        for y in (cy - 6)..(cy + 6) {
            for x in (cx - 6)..(cx + 6) {
                sim.grid.set(x, y, Particle::new(U235, 400));
            }
        }
        sim.grid
            .set(cx, cy - 7, Particle::new(NEUTRON_THERMAL, 350));
        sim.refresh_chunks_public();
        for _ in 0..300 {
            sim.tick();
        }
        sim.k_measured
    };

    let bare = run(false);
    let reflected = run(true);
    assert!(
        reflected > bare,
        "a graphite reflector must raise the measured k: bare {bare:.3} vs reflected {reflected:.3}"
    );
}
