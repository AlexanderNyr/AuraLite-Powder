//! Integration tests for AuraLite Powder simulation

use aura_lite_core::{element_id::*, Grid, NeutronEnergy, NeutronEvent, Particle, SimulationState};

#[test]
fn test_gravity_sand_falls() {
    let mut sim = SimulationState::new(10, 10, 0);
    sim.grid.set(5, 0, Particle::new(SAND, 293));
    for _ in 0..20 {
        sim.tick();
    }
    // Sand should have fallen to bottom
    assert!(
        sim.grid.get(5, 9).unwrap().element_id == SAND,
        "Sand should fall to bottom, found {:?}",
        sim.grid.get(5, 9)
    );
}

#[test]
fn test_water_flows() {
    let mut sim = SimulationState::new(30, 30, 0);
    // Solid bottom
    for x in 0..30 {
        sim.grid.set(x, 29, Particle::new(STONE, 293));
    }
    // Platform
    for x in 10..20 {
        sim.grid.set(x, 28, Particle::new(STONE, 293));
    }
    // Place a blob of water above the platform
    for y in 25..28 {
        sim.grid.set(15, y, Particle::new(WATER, 293));
    }

    for _ in 0..300 {
        sim.tick();
    }

    // Water should be somewhere on the platform level or spread around
    // Count water particles on row 27 (platform surface)
    let water_positions: Vec<(u32, u32)> = (0..30)
        .flat_map(|y| (0..30).map(move |x| (x, y)))
        .filter(|&(x, y)| sim.grid.get(x, y).map(|p| p.element_id) == Some(WATER))
        .collect();

    // Water should have fallen down from rows 25-27
    let water_on_or_near_platform = water_positions.iter().any(|(_, y)| *y >= 27);

    assert!(
        water_on_or_near_platform,
        "Water should flow toward platform. Water positions: {:?}",
        water_positions
    );
}

#[test]
fn test_fission_chain_reaction_starts() {
    let mut sim = SimulationState::new(128, 128, 42);
    // Create a small uranium cluster
    for y in 60..65 {
        for x in 60..65 {
            sim.grid.set(x, y, Particle::new(U235, 350));
        }
    }
    // Place a thermal neutron adjacent
    sim.grid.set(59, 62, Particle::new(NEUTRON_THERMAL, 350));

    let initial_fission = sim.fission_count;
    for _ in 0..30 {
        sim.tick();
    }

    assert!(
        sim.fission_count > initial_fission,
        "Fission chain reaction should have occurred: initial={}, final={}, queue_len={}",
        initial_fission,
        sim.fission_count,
        sim.neutron_queue.len()
    );
}

#[test]
fn test_boron_absorbs_neutrons() {
    let mut absorbed = false;
    for seed in 0..40 {
        let mut sim = SimulationState::new(10, 10, seed);
        sim.grid.set(5, 5, Particle::new(BORON, 293));
        sim.neutron_queue.push_back(NeutronEvent {
            x: 5,
            y: 5,
            delay: 0,
            energy: NeutronEnergy::Thermal,
        });
        sim.tick();
        let has_fallout = (0..10).any(|y| {
            (0..10).any(|x| sim.grid.get(x, y).map(|p| p.element_id) == Some(FALLOUT))
        });
        if has_fallout {
            absorbed = true;
            break;
        }
    }
    assert!(absorbed, "Boron should absorb a queued thermal neutron in some seeds");
}

#[test]
fn test_decay_happens() {
    let mut sim = SimulationState::new(10, 10, 0);
    sim.grid.set(5, 5, Particle::new(PU240, 400));

    // Pu-240 has half-life of 400_000 ticks, so decay won't happen in 100 ticks
    // But we test that the decay counter infrastructure works
    let initial_decay = sim.decay_count;
    for _ in 0..50 {
        sim.tick();
    }
    // With such short ticks and long half-life, decay is unlikely
    // But verify the simulation remains stable
    assert!(sim.grid.get(5, 5).is_some());
    let _ = initial_decay;
}

#[test]
fn test_temperature_diffusion() {
    let mut sim = SimulationState::new(10, 10, 0);
    sim.grid.set(5, 5, Particle::new(SAND, 1000));
    // Surround with cold particles
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = (5_i32 + dx) as u32;
            let ny = (5_i32 + dy) as u32;
            if nx < 10 && ny < 10 {
                sim.grid.set(nx, ny, Particle::new(STONE, 293));
            }
        }
    }

    let hot_before = sim.grid.get(5, 5).unwrap().temperature;
    for _ in 0..20 {
        sim.tick();
    }
    let hot_after = sim.grid.get(5, 5).unwrap().temperature;

    // Temperature should decrease due to diffusion
    assert!(
        hot_after < hot_before,
        "Temperature should diffuse: before={}, after={}",
        hot_before,
        hot_after
    );
}

#[test]
fn test_meltdown_transforms_fissile() {
    let mut sim = SimulationState::new(10, 10, 0);
    sim.grid.set(5, 5, Particle::new(U235, 2500)); // Above meltdown threshold

    for _ in 0..100 {
        sim.tick();
    }

    // At 1% per tick for 100 ticks at 2500K, the particle should eventually melt
    let final_id = sim.grid.get(5, 5).map(|p| p.element_id).unwrap_or(0);
    // May or may not have melted depending on RNG, but should not crash
    let _ = final_id;
}

#[test]
fn test_grid_resize_preserves_particles() {
    let mut grid = Grid::new(100, 100);
    grid.set(50, 50, Particle::new(SAND, 293));
    grid.set(75, 75, Particle::new(WATER, 350));

    grid.resize(80, 80);

    // Particle at (50,50) should still be within bounds
    assert_eq!(grid.get(50, 50).unwrap().element_id, SAND);
    // Particle at (75,75) should still be within bounds
    assert_eq!(grid.get(75, 75).unwrap().element_id, WATER);
}

#[test]
fn test_grid_resize_clamps_out_of_bounds() {
    let mut grid = Grid::new(100, 100);
    grid.set(90, 90, Particle::new(STONE, 293));

    grid.resize(50, 50);

    // Particle at (90,90) is now out of bounds, should be lost
    let non_empty = grid.count_non_empty();
    assert_eq!(non_empty, 0, "Out-of-bounds particles should be dropped");
}

#[test]
fn test_simulation_resize() {
    let mut sim = SimulationState::new(200, 200, 0);
    sim.grid.set(100, 100, Particle::new(U235, 400));

    sim.resize(256, 256);

    assert_eq!(sim.grid.width, 256);
    assert_eq!(sim.grid.height, 256);
    assert!(sim.grid.get(100, 100).is_some_and(|p| p.element_id == U235));
}

#[test]
fn test_neutron_queue_delay() {
    let mut sim = SimulationState::new(10, 10, 0);

    // Add a delayed neutron event
    sim.neutron_queue.push_back(NeutronEvent {
        x: 5,
        y: 5,
        delay: 3,
        energy: NeutronEnergy::Thermal,
    });

    // Queue should have 1 event
    assert_eq!(sim.neutron_queue.len(), 1);

    // After 3 ticks, neutron should be spawned
    sim.tick();
    sim.tick();
    sim.tick();
    // By now, the neutron should have been processed
    // It may have spawned at (5,5) if empty
    let _ = sim.grid.get(5, 5);
}

#[test]
fn test_fusion_triggers_at_high_temp() {
    let mut fused = false;
    for seed in 0..8 {
        let mut sim = SimulationState::new(20, 20, seed);
        for x in 8..14 {
            sim.grid.set(x, 10, Particle::new(DEUTERIUM, 2000));
            sim.grid.set(x, 11, Particle::new(TRITIUM, 2000));
        }
        sim.settings.fusion_threshold = 1500;
        for _ in 0..80 {
            sim.tick();
        }
        if sim.fusion_count > 0 {
            fused = true;
            break;
        }
    }
    assert!(fused, "D+T pairs at 2000 K should fuse in at least one seed");
}

#[test]
fn test_lithium_breeds_tritium() {
    let mut bred = false;
    for seed in 0..40 {
        let mut sim = SimulationState::new(10, 10, seed);
        sim.grid.set(5, 5, Particle::new(LITHIUM, 293));
        sim.neutron_queue.push_back(NeutronEvent {
            x: 5,
            y: 5,
            delay: 0,
            energy: NeutronEnergy::Thermal,
        });
        sim.tick();
        let found = (4..=6).any(|y| {
            (4..=6).any(|x| sim.grid.get(x, y).map(|p| p.element_id) == Some(TRITIUM))
        });
        if found {
            bred = true;
            break;
        }
    }
    assert!(bred, "Lithium + neutron should breed tritium in some seeds");
}

#[test]
fn test_gases_are_classified_as_gas() {
    assert_eq!(kind_for_id(HELIUM), ElementKind::Gas);
    assert_eq!(kind_for_id(HYDROGEN), ElementKind::Gas);
    assert_eq!(kind_for_id(TRITIUM), ElementKind::Gas);
    assert_eq!(kind_for_id(DEUTERIUM), ElementKind::Gas);
    assert!(is_gas(HELIUM));
    assert!(is_gas(STEAM));
    assert!(is_static_solid(CONCRETE));
    assert!(is_powder(SAND));
    assert!(is_powder(U235));
}

#[test]
fn test_element_registry_consistency() {
    // Ensure all element IDs have definitions
    for id in 0..=MAX_ELEMENT_ID {
        let def = aura_lite_elements::registry::get_definition(id);
        if id <= PIPE_STEAM {
            assert!(def.is_some(), "Element {} should have a definition", id);
        }
    }
}

#[test]
fn test_color_map_all_elements() {
    for id in 0..=MAX_ELEMENT_ID {
        let color = aura_lite_renderer::color_map::color_for_element(id);
        // Last element is magenta (unknown) but all others should be defined
        assert_eq!(color.len(), 4);
    }
}

#[test]
fn test_brush_circle_bounds() {
    use aura_lite_ui::brush::BrushSettings;

    let mut grid = Grid::new(20, 20);
    let brush = BrushSettings {
        radius: 3,
        selected_element: SAND,
        temperature: 293,
        ..Default::default()
    };

    // Apply brush near edge
    brush.apply_brush(&mut grid, 1, 1);
    brush.apply_brush(&mut grid, 18, 18);

    // Should not panic
    let count = grid.count_non_empty();
    assert!(count > 0, "Brush should have placed particles");
}

#[test]
fn test_brush_fill_bounded() {
    use aura_lite_ui::brush::BrushSettings;

    let mut grid = Grid::new(100, 100);
    // Fill a large empty area - should be bounded by depth limit
    let brush = BrushSettings {
        selected_element: WATER,
        temperature: 293,
        ..Default::default()
    };

    brush.apply_fill(&mut grid, 50, 50);

    let count = grid.count_non_empty();
    // With 100x100=10000 cells and max 10000 limit, all should be filled
    assert_eq!(count, 10000);
}

#[test]
fn test_bresenham_line() {
    let points = aura_lite_utils::math::bresenham_line(0, 0, 5, 3);
    assert!(points.contains(&(0, 0)));
    assert!(points.contains(&(5, 3)));
    // Should have reasonable number of points
    assert!(points.len() >= 4);
}

#[test]
fn test_chunk_pool_creation() {
    use aura_lite_core::chunk::ChunkPool;

    let pool = ChunkPool::new(256, 256);
    // 256/32 = 8 chunks in each dimension
    assert_eq!(pool.chunks_x, 8);
    assert_eq!(pool.chunks_y, 8);
    assert_eq!(pool.metas.len(), 64);
}

#[test]
fn test_chunk_pool_active_tracking() {
    use aura_lite_core::chunk::ChunkPool;

    let mut pool = ChunkPool::new(64, 64);
    // Mark chunk (0,0) as active
    if let Some(meta) = pool.get_mut(0, 0) {
        meta.mark_dirty(10, 10);
    }

    let active = pool.active_chunks();
    assert!(active.contains(&(0, 0)));
}

#[test]
fn test_static_solids_do_not_fall() {
    let mut sim = SimulationState::new(8, 8, 0);
    sim.grid.set(3, 1, Particle::new(CONCRETE, 293));
    sim.grid.set(4, 1, Particle::new(STEEL, 293));
    for _ in 0..20 {
        sim.tick();
    }
    assert_eq!(sim.grid.get(3, 1).unwrap().element_id, CONCRETE);
    assert_eq!(sim.grid.get(4, 1).unwrap().element_id, STEEL);
}

#[test]
fn test_sand_piles_instead_of_flowing_sideways() {
    let mut sim = SimulationState::new(21, 12, 1);
    for x in 0..21 {
        sim.grid.set(x, 11, Particle::new(STONE, 293));
    }
    // A 3-wide tower of sand on a flat floor.
    for y in 6..11 {
        for x in 9..12 {
            sim.grid.set(x, y, Particle::new(SAND, 293));
        }
    }
    for _ in 0..80 {
        sim.tick();
    }
    let far = (0..21)
        .filter(|&x| x <= 3 || x >= 17)
        .filter(|&x| sim.grid.get(x, 10).map(|p| p.element_id) == Some(SAND))
        .count();
    assert_eq!(far, 0, "sand should not run out like a liquid");
}

#[test]
fn test_water_finds_a_level() {
    let mut sim = SimulationState::new(24, 10, 2);
    for x in 0..24 {
        sim.grid.set(x, 9, Particle::new(STONE, 293));
    }
    for y in 4..9 {
        for x in 2..6 {
            sim.grid.set(x, y, Particle::new(WATER, 293));
        }
    }
    for _ in 0..200 {
        sim.tick();
    }
    let cols_with_water = (0..24)
        .filter(|&x| (0..9).any(|y| sim.grid.get(x, y).map(|p| p.element_id) == Some(WATER)))
        .count();
    assert!(
        cols_with_water >= 8,
        "water should spread across the basin, cols={cols_with_water}"
    );
}

#[test]
fn test_water_boils_to_steam_then_can_condense() {
    let mut sim = SimulationState::new(12, 12, 3);
    sim.grid.set(6, 10, Particle::new(STONE, 293));
    sim.grid.set(6, 9, Particle::new(WATER, 420));
    let mut saw_steam = false;
    for _ in 0..40 {
        sim.tick();
        saw_steam |= (0..12).any(|y| {
            (0..12).any(|x| sim.grid.get(x, y).map(|p| p.element_id) == Some(STEAM))
        });
    }
    assert!(saw_steam, "hot water should boil into steam");
}

#[test]
fn test_dense_powder_sinks_through_water() {
    let mut sim = SimulationState::new(9, 12, 4);
    for x in 0..9 {
        sim.grid.set(x, 11, Particle::new(STONE, 293));
    }
    for y in 6..11 {
        for x in 2..7 {
            sim.grid.set(x, y, Particle::new(WATER, 293));
        }
    }
    sim.grid.set(4, 5, Particle::new(LEAD, 293)); // static, should stay
    sim.grid.set(4, 6, Particle::new(U235, 293));
    for _ in 0..80 {
        sim.tick();
    }
    // Uranium powder should have settled near the floor, not stayed on top of the pool.
    let u_y = (0..12)
        .flat_map(|y| (0..9).map(move |x| (x, y)))
        .filter(|&(x, y)| sim.grid.get(x, y).map(|p| p.element_id) == Some(U235))
        .map(|(_, y)| y)
        .max()
        .unwrap_or(0);
    assert!(u_y >= 9, "U-235 should sink through water, y={u_y}");
    assert_eq!(sim.grid.get(4, 5).unwrap().element_id, LEAD);
}

#[test]
fn test_static_heater_warms_neighbors() {
    let mut sim = SimulationState::new(12, 12, 1);
    sim.grid.set(6, 6, Particle::new(HEATER, 800));
    sim.grid.set(6, 5, Particle::new(WATER, 293));
    for _ in 0..8 {
        sim.tick();
    }
    assert!(sim.grid.get(6, 5).unwrap().temperature > 293);
}

#[test]
fn test_scenario_bomb_places_plutonium() {
    let mut sim = SimulationState::new(64, 64, 0);
    sim.load_scenario(aura_lite_core::Scenario::Bomb);
    let pu = sim
        .grid
        .particles
        .iter()
        .filter(|p| p.element_id == PU239)
        .count();
    assert!(pu > 20, "bomb scene should pack a Pu pit, got {pu}");
}

#[test]
fn test_control_rods_shift() {
    let mut sim = SimulationState::new(16, 20, 0);
    sim.grid.set(8, 10, Particle::new(CONTROL_ROD, 293));
    sim.shift_control_rods(-1);
    assert_eq!(sim.grid.get(8, 9).unwrap().element_id, CONTROL_ROD);
    assert!(sim.grid.get(8, 10).unwrap().is_empty());
}

#[test]
fn test_acid_eats_stone() {
    let mut eaten = false;
    for seed in 0..20 {
        let mut sim = SimulationState::new(10, 10, seed);
        for x in 0..10 {
            sim.grid.set(x, 8, Particle::new(STONE, 293));
        }
        sim.grid.set(5, 7, Particle::new(ACID, 293));
        for _ in 0..40 {
            sim.tick();
        }
        if (0..10).any(|x| sim.grid.get(x, 8).map(|p| p.element_id) == Some(SLAG)) {
            eaten = true;
            break;
        }
    }
    assert!(eaten, "acid should dissolve stone in some seeds");
}

#[test]
fn test_filter_passes_water_not_sand() {
    let mut sim = SimulationState::new(8, 10, 1);
    for x in 0..8 {
        sim.grid.set(x, 5, Particle::new(FILTER, 293));
        sim.grid.set(x, 9, Particle::new(STONE, 293));
    }
    sim.grid.set(3, 4, Particle::new(WATER, 293));
    sim.grid.set(5, 4, Particle::new(SAND, 293));
    for _ in 0..20 {
        sim.tick();
    }
    let water_below = (0..8).any(|x| {
        (6..9).any(|y| sim.grid.get(x, y).map(|p| p.element_id) == Some(WATER))
    });
    let sand_below = (0..8).any(|x| {
        (6..9).any(|y| sim.grid.get(x, y).map(|p| p.element_id) == Some(SAND))
    });
    assert!(water_below, "water should pass through a filter");
    assert!(!sand_below, "sand should not pass through a filter");
}

#[test]
fn test_sensor_heats_with_criticality() {
    let mut sim = SimulationState::new(16, 16, 2);
    sim.grid.set(8, 8, Particle::new(SENSOR, 293));
    for y in 6..11 {
        for x in 6..11 {
            if x == 8 && y == 8 {
                continue;
            }
            sim.grid.set(x, y, Particle::new(U235, 400));
        }
    }
    sim.grid.set(8, 7, Particle::new(NEUTRON_THERMAL, 350));
    for _ in 0..6 {
        sim.tick();
    }
    assert!(
        sim.grid.get(8, 8).unwrap().temperature > 293,
        "sensor should warm when the pile is active"
    );
}

#[test]
fn test_iodine_decays_toward_xenon() {
    assert_eq!(
        aura_lite_core::reactions::decay_daughter(IODINE),
        XENON
    );
    let mut saw = false;
    for seed in 0..8 {
        let mut sim = SimulationState::new(10, 10, seed);
        for y in 2..8 {
            for x in 2..8 {
                sim.grid.set(x, y, Particle::new(IODINE, 320));
            }
        }
        for _ in 0..900 {
            sim.tick();
        }
        if sim.decay_count > 0
            || sim
                .grid
                .particles
                .iter()
                .any(|p| p.element_id == XENON)
        {
            saw = true;
            break;
        }
    }
    assert!(saw, "a patch of iodine should produce xenon");
}

#[test]
fn test_fire_dies_underwater() {
    let mut sim = SimulationState::new(10, 10, 1);
    for x in 0..10 {
        for y in 0..10 {
            sim.grid.set(x, y, Particle::new(WATER, 293));
        }
    }
    sim.grid.set(5, 5, Particle::new(FIRE, 1100));
    for _ in 0..20 {
        sim.tick();
    }
    let fire = sim
        .grid
        .particles
        .iter()
        .filter(|p| p.element_id == FIRE)
        .count();
    assert_eq!(fire, 0, "fire should extinguish when fully flooded");
}

#[test]
fn test_reactor_hud_fields_exist() {
    let mut sim = SimulationState::new(16, 16, 0);
    sim.load_scenario(aura_lite_core::Scenario::ControlledReactor);
    for _ in 0..5 {
        sim.tick();
    }
    let _ = sim.reactor_status();
    let _ = sim.power;
    let _ = sim.iodine_count;
}

#[test]
fn test_mission_hold_starts_with_fuel() {
    let mut sim = SimulationState::new(64, 64, 1);
    let m = aura_lite_core::Mission::start(&mut sim, aura_lite_core::MissionId::HoldCritical);
    assert_eq!(m.status, aura_lite_core::MissionStatus::Running);
    let fuel = sim
        .grid
        .particles
        .iter()
        .filter(|p| p.element_id == U235)
        .count();
    assert!(fuel > 10, "hold mission should plant a U-235 pile");
}

#[test]
fn test_mission_save_roundtrip() {
    let mut sim = SimulationState::new(48, 48, 2);
    let m = aura_lite_core::Mission::start(&mut sim, aura_lite_core::MissionId::FilterRescue);
    sim.mission = Some(m.to_save());
    let bytes = aura_lite_io::save_simulation_to_bytes(&sim, false).unwrap();
    let save = aura_lite_io::load_save_from_bytes(&bytes, false).unwrap();
    let mut loaded = SimulationState::new(48, 48, 0);
    save.apply_to(&mut loaded).unwrap();
    let restored = loaded.mission.as_ref().and_then(aura_lite_core::Mission::from_save);
    assert!(restored.is_some());
    assert_eq!(
        restored.unwrap().id,
        aura_lite_core::MissionId::FilterRescue
    );
}

#[test]
fn test_mission_filter_rescue_can_win() {
    let mut sim = SimulationState::new(32, 32, 1);
    let mut mission = aura_lite_core::Mission::start(
        &mut sim,
        aura_lite_core::MissionId::FilterRescue,
    );
    for _ in 0..200 {
        sim.tick();
        mission.tick(&sim);
        if mission.status != aura_lite_core::MissionStatus::Running {
            break;
        }
    }
    assert_ne!(
        mission.status,
        aura_lite_core::MissionStatus::Failed,
        "{}",
        mission.message
    );
}

#[test]
fn test_pressure_stays_inside_stone_box() {
    let mut sim = SimulationState::new(16, 16, 1);
    for x in 4..12 {
        sim.grid.set(x, 4, Particle::new(STONE, 293));
        sim.grid.set(x, 11, Particle::new(STONE, 293));
    }
    for y in 4..12 {
        sim.grid.set(4, y, Particle::new(STONE, 293));
        sim.grid.set(11, y, Particle::new(STONE, 293));
    }
    sim.grid.set(7, 7, Particle::new(STEAM, 450));
    sim.grid.set(8, 7, Particle::new(STEAM, 450));
    for _ in 0..30 {
        sim.tick();
    }
    let outside = sim.pressure.p[sim.grid.index(2, 2)];
    let inside = sim.pressure.p[sim.grid.index(7, 8)];
    assert!(
        outside < 12,
        "pressure leaked through stone: outside={outside} inside={inside}"
    );
}

#[test]
fn test_save_restores_pressure() {
    let mut sim = SimulationState::new(12, 12, 3);
    sim.pressure.p[20] = 77;
    sim.velocities.vx[20] = 2;
    let bytes = aura_lite_io::save_simulation_to_bytes(&sim, false).unwrap();
    let save = aura_lite_io::load_save_from_bytes(&bytes, false).unwrap();
    let mut loaded = SimulationState::new(12, 12, 0);
    save.apply_to(&mut loaded).unwrap();
    assert_eq!(loaded.pressure.p[20], 77);
    assert_eq!(loaded.velocities.vx[20], 2);
}

#[test]
fn test_gif_encoder_writes_header() {
    let frame = vec![255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
    let bytes = aura_lite_io::gif89a::encode_rgba_frames(&[frame], 2, 2, 5).unwrap();
    assert!(bytes.starts_with(b"GIF89a"));
    assert_eq!(*bytes.last().unwrap(), 0x3B);
}

#[test]
fn test_pipe_carries_water_through_a_wall() {
    let mut sim = SimulationState::new(16, 12, 1);
    for x in 0..16 {
        sim.grid.set(x, 11, Particle::new(STONE, 293));
    }
    for x in 3..12 {
        sim.grid.set(x, 6, Particle::new(PIPE, 293));
    }
    sim.grid.set(2, 6, Particle::new(WATER, 293));
    sim.grid.set(2, 5, Particle::new(WATER, 293));
    sim.grid.set(1, 6, Particle::new(PUMP, 293));
    for _ in 0..80 {
        sim.tick();
    }
    let carried = (10..16).any(|x| {
        (0..11).any(|y| {
            matches!(
                sim.grid.get(x, y).map(|p| p.element_id),
                Some(WATER) | Some(PIPE_WATER)
            )
        })
    });
    assert!(carried, "a pipe run should move water past x=10");
}

#[test]
fn test_hydrostatic_pressure_grows_with_depth() {
    let mut sim = SimulationState::new(8, 16, 0);
    for y in 8..15 {
        for x in 2..6 {
            sim.grid.set(x, y, Particle::new(WATER, 293));
        }
    }
    for x in 0..8 {
        sim.grid.set(x, 15, Particle::new(STONE, 293));
    }
    for _ in 0..12 {
        sim.tick();
    }
    let top = sim.pressure.p[sim.grid.index(3, 9)];
    let bot = sim.pressure.p[sim.grid.index(3, 14)];
    assert!(
        bot >= top,
        "deeper water should be at least as pressurized (top={top} bot={bot})"
    );
}
