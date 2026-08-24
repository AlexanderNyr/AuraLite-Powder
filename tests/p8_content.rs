//! P8 content gates (ROADMAP): the two new missions are winnable and the
//! campaign unlock logic is correct.

use aura_lite_core::{Campaign, Mission, MissionId, MissionStatus, SimulationState};

#[test]
fn test_mission_tritium_breeder_wins() {
    let mut sim = SimulationState::new(96, 96, 1);
    let mut mission = Mission::start(&mut sim, MissionId::TritiumBreeder);
    for _ in 0..80 {
        sim.tick();
        mission.tick(&sim);
        if mission.status != MissionStatus::Running {
            break;
        }
    }
    assert_eq!(
        mission.status,
        MissionStatus::Won,
        "tritium breeder: {}",
        mission.message
    );
}

#[test]
fn test_mission_quench_wins() {
    let mut sim = SimulationState::new(96, 96, 2);
    let mut mission = Mission::start(&mut sim, MissionId::Quench);
    for _ in 0..300 {
        sim.tick();
        mission.tick(&sim);
        if mission.status != MissionStatus::Running {
            break;
        }
    }
    assert_ne!(
        mission.status,
        MissionStatus::Failed,
        "quench failed: {}",
        mission.message
    );
    assert_eq!(
        mission.status,
        MissionStatus::Won,
        "quench: {}",
        mission.message
    );
}

#[test]
fn test_campaign_unlock_logic() {
    let mut c = Campaign::new();
    // The first mission is unlocked from the start; everything else is locked.
    assert!(c.is_unlocked(MissionId::all()[0]));
    assert!(!c.is_unlocked(MissionId::all()[1]));
    assert_eq!(c.next(), Some(MissionId::all()[0]));

    // Winning the first unlocks the second.
    c.record(MissionId::all()[0], MissionStatus::Won);
    assert!(c.is_unlocked(MissionId::all()[1]));
    assert!(c.completed.contains(&MissionId::all()[0]));
    assert_eq!(c.next(), Some(MissionId::all()[1]));

    // A failure does not unlock anything.
    let mut c2 = Campaign::new();
    c2.record(MissionId::all()[0], MissionStatus::Failed);
    assert!(!c2.is_unlocked(MissionId::all()[1]));

    // All eight missions exist and round-trip through from_u8.
    assert_eq!(MissionId::all().len(), 8);
    for &m in MissionId::all() {
        assert_eq!(MissionId::from_u8(m as u8), Some(m));
    }
}

#[test]
fn test_eight_missions_all_start() {
    // Every mission must start without panic and report a running status.
    for &m in MissionId::all() {
        let mut sim = SimulationState::new(96, 96, 7);
        let mission = Mission::start(&mut sim, m);
        assert_eq!(
            mission.status,
            MissionStatus::Running,
            "{} did not start running",
            m.title()
        );
        assert!(!mission.message.is_empty());
    }
}
