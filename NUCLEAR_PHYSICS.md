# Nuclear Physics Mechanics

This document explains the simplified but physically inspired models used for fission, fusion, decay, chains, temperature, radiation.

## Element Registry

- Fissile: U-235 (id 4), U-238 (5), Pu-239 (6), Pu-240 (7), Molten Fuel (26) also fissile
- Moderator/Coolant: Heavy Water D2O (8), Graphite (9), Water (2)
- Structural: Lead (10), Concrete (11), Steel (12), Boron (28 absorber)
- Radiation: Neutron thermal (13), Neutron fast (14), Gamma (15), Alpha (16), Beta (17)
- Products: Depleted Uranium (18), Fission Products (19), Fallout (27)
- Fusion fuel: Tritium (20), Deuterium (21), Helium (25 product)
- Other: TNT (22), Hydrogen (23), Lithium (24)

## Fission

Trigger: fissile particle adjacent (8-neighborhood) to neutron.

Probability function `fission_probability(element, neutron_energy, temperature)`:

- U-235: thermal 0.85, fast 0.35
- Pu-239: thermal 0.90, fast 0.40
- U-238: thermal 0.02, fast 0.25 (fast fission threshold)
- Pu-240: thermal 0.10, fast 0.30
- Temperature factor: 1.0 + (T-293)/1000 clamped [-0.5,1.0], hotter increases reactivity slightly

On fission:

- Transform fissile -> Fission Products (or Molten if hot)
- Spawn 2-3 fast neutrons via `neutron_queue` with delay 1-3 ticks, spread radius 2 cells
- Spawn 1-2 gamma rays adjacent if empty
- Heat neighbors radius 2 by 50-200 K
- Increment fission_count, reaction_count
- Energy released tracked (202 MeV scaled) but currently converted to temperature

ReactionTable entry also data-driven:

```
U235 + thermal neutron -> FP + 2*fast neutron + gamma, prob 0.85, temp +500
```

## Chain Reaction & Neutron Queue

Neutrons have finite travel speed simulated by delaying spawn.

- `NeutronEvent { x,y,delay,energy }` enqueued on fission
- Each tick, delay--, when 0 attempted placement:
  - If target empty: spawn thermal/fast neutron particle with lifetime 30/40 ticks
  - If fissile: attempt fission with probability
  - If absorber Boron: 80% absorbed -> Fallout + alpha
  - If moderator Water/HW/Graphite: fast -> thermal conversion with 30-50% probability, enqueues slowed neutron
- Thermal neutrons walk 1 cell/tick random, fast 2 cells

This creates branching chain reaction visible.

Criticality:

- Simple check `mass_count >= critical_mass_threshold` (default 8)
- k-effective approx: `(fissile*2.5 * (1+moderator*0.3)) / (1+absorber*0.8 + escape)` scaled /100
- Not yet enforcing; for future feedback on tick rate

## Fusion

Condition: Deuterium adjacent to Tritium AND both temperature > FUSION_THRESHOLD = 1500 K

- Probability 0.05 per tick per pair (once high T, probability logistic)
- Reaction: D+T -> He + fast neutron + 1200K spike radius 3
- Massive temp spike makes nearby D+T more likely to fuse, causing propagation
- Products: Helium gas (buoyant)

Data-driven ReactionTable also contains D+T entry.

## Decay & Half-Life

Each isotope has `half_life_ticks` scaled down from real half-life for visible effect:

- U-235: 1e6 ticks
- U-238: 2e6
- Pu-239: 5e5
- Pu-240: 4e5
- Tritium: 1e5
- Others stable

Decay probability per tick: `ln2 / half_life`. Each tick, RNG roll.

On decay:

- Transform parent -> daughter (simplified chain):
  - U-238 -> Depleted Uranium
  - U-235 -> Fission Products
  - Pu-239 -> U-235
  - Pu-240 -> Pu-239
  - Tritium -> Helium
- Emit radiation particle (alpha for actinides, beta for Tritium, gamma otherwise) placed adjacent if empty
- Increment decay_count

Full chain U-238 -> Th-234 -> ... -> Pb-206 approximated for MVP; extendable to full chain table.

## Temperature & Heat Transfer

- Each particle stores temperature u16 Kelvin (0-5000)
- Baseline 293K (20C)
- Diffusion: each tick, for each cell, average with 8 neighbors: `new = cur + (avg - cur) * diffusion_rate` (default 0.08)
- Plus cooling: ` *0.999 + 293*0.001` slight decay to ambient
- Fission adds +50..200K to neighbors radius 2, plus +500K to product itself
- Fusion adds +800K radius 3
- Gamma, neutrons deposit small energy on penetration (5-15K)

Meltdown:

- If fissile && temp >2000K, 1% chance per tick to become Molten Fuel, spreads heat +100K to neighbors
- If water && temp >2500K, 5% chance to become Hydrogen gas (boiling)
- TNT && temp >500K triggers explosion (radius 6 clearing)

## Radiation Penetration

Radiation particles are moving particles (not just static property):

- Movement: random walk each tick; neutron fast 2 steps, others 1, gamma 3
- Lifetime: alpha 8 ticks, beta 12, gamma 20, thermal neutron 30, fast 40; then disappear
- When colliding with non-empty particle:
  - Check penetration_depth per type:
    - Gamma 12, Fast neutron 15, Thermal 8, Beta 4, Alpha 2
  - If RNG < penetration_depth, deposit energy to target temperature and allow gamma to continue (70% chance passes through)
  - Otherwise stop

- Lead, concrete provide shielding via higher density but currently not special blocking except via low penetration prob; future: increase blocking based on density and thickness

- Boron specially absorbs thermal neutrons high cross-section

## Isotope Decay Chains & Waste

Fission products aggregated as single element "Fission Products" for simplicity. Future extension: mixed isotope waste with variable decay and heat.

Tritium breeding: Lithium + neutron (thermal or fast) converts the cell to Tritium and, if a neighbour is empty, spawns Helium. Probability is `LITHIUM_BREED_CHANCE` (0.40) in `aura_lite_core::reactions`.

## Validation

Test fission chain stability:

- Place 3x3 U-235 block, 1 thermal neutron adjacent -> observe exponential growth then saturation as fuel converts to FP
- Apply boron rods to control

Test fusion:

- Place D+T pair heated to 1600K via initial temperature field -> should fuse after few ticks, spawn He + neutron, heat spike

Test moderation:

- Fast neutrons in water should thermalize after few collisions, increasing fission rate

Performance: 512x512 grid with ~50% fill, fission chain reaction running, should stay >=30 ticks/sec on modern hardware (single-threaded MVP); with future rayon chunking, 1024x1024 target.

## Limitations & Future

- No neutron cross-section energy dependence beyond thermal/fast binary
- No photon transport Monte Carlo; simplified random walk
- No delayed neutrons separate from prompt
- No coolant flow modeling
- No isotopic enrichment modeling beyond distinct IDs
- Criticality calculated but not feeding back into reactivity feedback
- Temperature diffusion simple averaging, not conduction equation
- No explicit pressure field

These can be added incrementally in elements::nuclear modules.
