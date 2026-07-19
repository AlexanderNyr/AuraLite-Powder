use criterion::{black_box, criterion_group, criterion_main, Criterion};
use aura_lite_core::{SimulationState, Particle};

fn bench_256(c: &mut Criterion) {
    c.bench_function("simulation_tick_256", |b| {
        let mut sim = SimulationState::new(256, 256, 42);
        // populate
        for y in 0..128 {
            for x in 0..256 {
                if (x+y) % 2 == 0 {
                    sim.grid.set(x, y, Particle::new(1, 293));
                }
            }
        }
        b.iter(|| {
            sim.tick();
            black_box(&sim.grid);
        })
    });
}

fn bench_512(c: &mut Criterion) {
    c.bench_function("simulation_tick_512", |b| {
        let mut sim = SimulationState::new(512, 512, 42);
        for y in 0..256 {
            for x in 0..512 {
                if (x+y) % 3 == 0 {
                    sim.grid.set(x, y, Particle::new(1, 293));
                }
            }
        }
        b.iter(|| {
            sim.tick();
            black_box(&sim.grid);
        })
    });
}

criterion_group!(benches, bench_256, bench_512);
criterion_main!(benches);
