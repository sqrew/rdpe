# Rules

Rules define how particles behave. They're composable building blocks that execute every frame in the order you add them. RDPE includes **90+ built-in rules** across multiple categories.

## Quick Reference

| Category                                      | Rules                                                                                                                                             |
|-----------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| [Physics](#physics-rules)                     | Gravity, Drag, Acceleration, BounceWalls, WrapWalls                                                                                               |
| [Forces](#force-rules)                        | AttractTo, RepelFrom, Seek, Flee, Arrive, Vortex, Turbulence, Orbit, Curl, PointGravity, Spring, Radial, Shockwave, Pulse, Oscillate, PositionNoise |
| [Neighbors](#neighbor-rules)                  | Separate, Cohere, Align, Flock, Collide, Avoid, NBodyGravity, LennardJones, DLA, Viscosity, Pressure, Magnetism, SurfaceTension, Diffuse, Signal, Absorb, Accumulate |
| [Types](#type-rules)                          | Typed, Convert, Chase, Evade                                                                                                                      |
| [Lifecycle](#lifecycle-rules)                 | Age, Lifetime, FadeOut, ShrinkOut, Die, Grow, Decay, Split                                                                                        |
| [Visual](#visual-rules)                       | ColorOverLife, ColorBySpeed, ColorByAge, ScaleBySpeed                                                                                             |
| [Springs](#spring-rules)                      | BondSprings, ChainSprings, RadialSprings                                                                                                          |
| [Environment](#environment-rules)             | Buoyancy, DensityBuoyancy, Friction, Wind, Current, RespawnBelow                                                                                  |
| [State](#state-rules)                         | State, Agent                                                                                                                                      |
| [Fields](#field-rules)                        | Gradient, Sync, Deposit, Sense, Consume                                                                                                           |
| [Conditional](#conditional-rules)             | Maybe, Trigger, Periodic, Gate, Switch                                                                                                            |
| [Signal Processing](#signal-processing-rules) | Lerp, Tween, Threshold, Noise, Remap, Clamp, Smooth, Quantize, Modulo, Copy, Mass                                                                 |
| [Logic Gates](#logic-gate-rules)              | And, Or, Not, Xor, Hysteresis, Latch, Edge, Select, Blend                                                                                         |
| [Custom](#custom-rules)                       | Custom, NeighborCustom, OnCollision                                                                                                               |

---

## Physics Rules

Basic physics behaviors that form the foundation of most simulations.

### Gravity

Constant downward acceleration:

```rust
Rule::Gravity(9.8)  // Earth-like
Rule::Gravity(1.6)  // Moon-like
```

### Drag

Air resistance / friction - slows particles over time:

```rust
Rule::Drag(1.0)   // Moderate air resistance
Rule::Drag(0.1)   // Very little friction (space-like)
Rule::Drag(5.0)   // Heavy friction (underwater feel)
```

### Acceleration

Constant acceleration in any direction:

```rust
Rule::Acceleration(Vec3::new(1.0, 0.0, 0.0))   // Rightward wind
Rule::Acceleration(Vec3::new(0.0, -9.8, 0.0))  // Same as Gravity(9.8)
```

### BounceWalls

Particles reflect off the bounding box:

```rust
.with_bounds(1.0)           // Cube from -1 to +1
.with_rule(Rule::BounceWalls)
```

### WrapWalls

Toroidal topology - particles wrap to opposite side:

```rust
.with_bounds(1.0)
.with_rule(Rule::WrapWalls)  // Endless space, no edges
```

### SpeedLimit

Clamp velocity to min/max:

```rust
Rule::SpeedLimit {
    min: 0.5,   // Always moving
    max: 3.0,   // But not too fast
}
```

---

## Force Rules

Point-based and field-based forces that shape particle motion.

### AttractTo

Pull toward a fixed point:

```rust
Rule::AttractTo {
    point: Vec3::ZERO,
    strength: 2.0,
}
```

### RepelFrom

Push away from a point within a radius:

```rust
Rule::RepelFrom {
    point: Vec3::ZERO,
    strength: 5.0,
    radius: 0.5,  // Only affects particles within 0.5 units
}
```

### Seek

Reynolds-style steering toward a target point:

```rust
Rule::Seek {
    target: Vec3::new(1.0, 0.0, 0.0),
    max_speed: 2.0,
    max_force: 1.0,  // Limits steering acceleration
}
```

Computes desired velocity toward target, then applies steering force (desired - current). Great for agents, missiles, and homing behaviors.

### Flee

Steering away from a target - the opposite of Seek:

```rust
Rule::Flee {
    target: Vec3::ZERO,
    max_speed: 3.0,
    max_force: 1.5,
    panic_radius: 0.5,  // Only flee when within radius (0.0 = always flee)
}
```

### Arrive

Seek with deceleration - slows down as it approaches the target:

```rust
Rule::Arrive {
    target: Vec3::new(0.5, 0.5, 0.0),
    max_speed: 2.0,
    max_force: 1.0,
    slowing_radius: 0.3,  // Start decelerating within this radius
}
```

Prevents overshooting - particles smoothly come to rest at the target.

### Vortex

Rotational force around an axis (tornados, whirlpools):

```rust
Rule::Vortex {
    center: Vec3::ZERO,
    axis: Vec3::Y,      // Rotate around Y axis
    strength: 2.0,      // Positive = counter-clockwise
}
```

### Turbulence

Noise-based chaotic force field:

```rust
Rule::Turbulence {
    scale: 2.0,      // Noise frequency (smaller = larger swirls)
    strength: 1.5,
}
```

### Curl

Divergence-free flow for fluid/smoke effects:

```rust
Rule::Curl {
    scale: 1.5,      // Large flowing structures
    strength: 2.0,
}
```

More expensive than Turbulence (samples noise 6x), but particles never bunch up.

### PointGravity

Inverse-square gravity toward a point (black holes, attractors):

```rust
Rule::PointGravity {
    point: Vec3::ZERO,
    strength: 2.0,
    softening: 0.05,  // Prevents singularities
}
```

### Spring

Hooke's law force toward an anchor:

```rust
Rule::Spring {
    anchor: Vec3::ZERO,
    stiffness: 5.0,
    damping: 0.5,
}
```

### Orbit

Circular orbit around a center:

```rust
Rule::Orbit {
    center: Vec3::ZERO,
    strength: 2.0,
}
```

### Radial

Configurable explode/implode force with falloff:

```rust
// Explosion
Rule::Radial {
    point: Vec3::ZERO,
    strength: 5.0,       // Positive = outward
    radius: 2.0,
    falloff: Falloff::InverseSquare,
}

// Black hole
Rule::Radial {
    point: Vec3::ZERO,
    strength: -3.0,      // Negative = inward
    radius: 1.5,
    falloff: Falloff::Smooth,
}
```

**Falloff types:** `Constant`, `Linear`, `Inverse`, `InverseSquare`, `Smooth`

### Shockwave

Expanding ring/sphere that pushes particles:

```rust
Rule::Shockwave {
    origin: Vec3::ZERO,
    speed: 2.0,       // Expansion rate
    width: 0.3,       // Wavefront thickness
    strength: 5.0,
    repeat: 2.0,      // Every 2 seconds (0.0 = one-time)
}
```

### Pulse

Breathing radial force - oscillates between expand/contract:

```rust
Rule::Pulse {
    point: Vec3::ZERO,
    strength: 2.0,
    frequency: 0.5,   // One breath every 2 seconds
    radius: 1.5,
}
```

### Oscillate

Sine-wave velocity oscillation:

```rust
// Simple up-down pulse (all particles in sync)
Rule::Oscillate {
    axis: Vec3::Y,
    amplitude: 0.5,
    frequency: 2.0,
    spatial_scale: 0.0,
}

// Radial ripples (like dropping a stone in water)
Rule::Oscillate {
    axis: Vec3::Y,
    amplitude: 0.3,
    frequency: 1.0,
    spatial_scale: 5.0,  // Higher = tighter ripples
}
```

### PositionNoise

Add jitter to positions:

```rust
Rule::PositionNoise {
    scale: 5.0,
    strength: 0.02,
    speed: 2.0,
}
```

---

## Neighbor Rules

These rules require **spatial hashing** (automatically enabled when used). They consider nearby particles for interactions.

```rust
// Enable spatial hashing
.with_spatial_config(cell_size, grid_resolution)
```

### Separate

Avoid crowding neighbors (boids):

```rust
Rule::Separate {
    radius: 0.05,     // Personal space
    strength: 2.0,
}
```

### Cohere

Steer toward center of nearby neighbors (boids):

```rust
Rule::Cohere {
    radius: 0.15,     // Flock awareness range
    strength: 1.0,
}
```

### Align

Match velocity with neighbors (boids):

```rust
Rule::Align {
    radius: 0.1,
    strength: 1.5,
}
```

### Flock

All three boid rules combined:

```rust
Rule::Flock {
    radius: 0.15,
    separation: 2.0,
    cohesion: 1.0,
    alignment: 1.5,
}
```

### Collide

Elastic collision response:

```rust
Rule::Collide {
    radius: 0.05,
    restitution: 0.8,  // 0=sticky, 1=bouncy
}
```

### Avoid

Smooth steering-based avoidance (better for flocking/crowds):

```rust
Rule::Avoid {
    radius: 0.1,
    strength: 3.0,
}
```

### NBodyGravity

Particle-to-particle gravitational attraction:

```rust
Rule::NBodyGravity {
    strength: 0.5,
    softening: 0.02,   // Prevents singularities
    radius: 0.5,       // Performance: limit range
}
```

### LennardJones

Molecular dynamics potential - repulsion at close range, attraction at medium range:

```rust
Rule::LennardJones {
    epsilon: 1.0,    // Well depth (attraction strength at equilibrium)
    sigma: 0.05,     // Zero-crossing distance (effective particle diameter)
    cutoff: 0.125,   // Cutoff radius (typically 2.5 * sigma)
}
```

Creates realistic molecular clustering - particles form stable structures with preferred spacing. At equilibrium distance (r ≈ 1.12σ), forces balance. Closer = strong repulsion, farther = gentle attraction.

### DLA

Diffusion-Limited Aggregation - creates fractal crystal/tree structures:

```rust
Rule::DLA {
    seed_type: 0,           // Immobile structure particles
    mobile_type: 1,         // Random-walking particles
    stick_radius: 0.03,     // Contact distance to stick
    diffusion_strength: 0.5, // Brownian motion intensity
}
```

Mobile particles wander randomly. When they touch a seed particle, they stick and become part of the structure. Creates beautiful fractal patterns like snowflakes, lightning, or mineral dendrites.

### Viscosity

Velocity smoothing (fluid-like):

```rust
Rule::Viscosity {
    radius: 0.1,
    strength: 0.5,     // Higher = thicker fluid
}
```

### Pressure

Density-based repulsion (SPH-style):

```rust
Rule::Pressure {
    radius: 0.1,
    strength: 2.0,
    target_density: 8.0,  // Push when > 8 neighbors
}
```

### Magnetism

Charge-based attraction/repulsion using `particle_type`:

```rust
Rule::Magnetism {
    radius: 0.3,
    strength: 1.5,
    same_repel: true,  // Same types repel, opposites attract
}
```

### SurfaceTension

Keep fluid blobs together:

```rust
Rule::SurfaceTension {
    radius: 0.1,
    strength: 2.0,
    threshold: 8.0,  // Apply when < 8 neighbors
}
```

### Diffuse

Property diffusion through neighbor averaging:

```rust
Rule::Diffuse {
    field: "temperature".into(),
    rate: 0.3,
    radius: 0.15,
}
```

### Signal

Broadcast a value to nearby particles (chemical signaling, pheromones):

```rust
Rule::Signal {
    source: "pheromone".into(),    // Field to broadcast
    target: "detected".into(),      // Field to receive signal
    radius: 0.2,
    strength: 1.0,
    falloff: Some(Falloff::Linear),
}
```

Useful for communication between particles - each particle receives the average signal from neighbors within radius.

### Absorb

Consume nearby particles and absorb their properties:

```rust
Rule::Absorb {
    target_type: Some(1),           // Only absorb type 1 (None = any)
    radius: 0.08,
    source_field: "energy".into(),  // What to absorb
    target_field: "energy".into(),  // Where to store it
}
```

When a neighbor is found within radius, its `source_field` value is added to this particle's `target_field`. Great for predator-prey dynamics, resource collection, and cell-eating behaviors.

### Accumulate

Gather values from neighbors with configurable operations:

```rust
// Sum neighbor energies
Rule::Accumulate {
    source: "energy".into(),
    target: "neighbor_energy".into(),
    radius: 0.15,
    operation: "sum".into(),  // "sum", "average", "max", "min"
    falloff: Some(Falloff::Smooth),
}
```

More flexible than Signal - supports different aggregation modes and custom falloff.

---

## Type Rules

Rules that filter by or interact with particle types.

### Typed

Wrap any neighbor rule with type filters:

```rust
Rule::Typed {
    self_type: 0,
    other_type: Some(1),  // Only interact with type 1
    rule: Box::new(Rule::Separate { radius: 0.1, strength: 5.0 }),
}
```

### Convert

Change particle type on contact:

```rust
Rule::Convert {
    from_type: 0,        // Healthy
    trigger_type: 1,     // Infected
    to_type: 1,          // Becomes infected
    radius: 0.08,
    probability: 0.15,   // 15% per contact
}
```

### Chase

Steer toward nearest target type:

```rust
Rule::Chase {
    self_type: 1,        // Predators
    target_type: 0,      // Chase prey
    radius: 0.4,
    strength: 4.0,
}
```

### Evade

Steer away from nearest threat type:

```rust
Rule::Evade {
    self_type: 0,        // Prey
    threat_type: 1,      // Flee predators
    radius: 0.25,
    strength: 6.0,
}
```

---

## Lifecycle Rules

Manage particle age, death, and visual fading.

### Age

Increment particle age each frame:

```rust
Rule::Age
```

### Lifetime

Kill particles after duration:

```rust
.with_rule(Rule::Age)
.with_rule(Rule::Lifetime(3.0))  // Die after 3 seconds
```

### FadeOut

Fade color over lifetime:

```rust
.with_rule(Rule::Age)
.with_rule(Rule::FadeOut(2.0))
.with_rule(Rule::Lifetime(2.0))
```

### ShrinkOut

Shrink scale over lifetime:

```rust
.with_rule(Rule::Age)
.with_rule(Rule::ShrinkOut(2.0))
.with_rule(Rule::Lifetime(2.0))
```

### Die

Conditional particle death:

```rust
Rule::Die {
    condition: "p.energy <= 0.0".into(),
    field: "alive".into(),
}
```

### Grow

Change scale over time:

```rust
Rule::Grow { rate: 0.5, min: 0.1, max: 2.0 }   // Growing
Rule::Grow { rate: -0.3, min: 0.0, max: 1.0 }  // Shrinking
```

### Decay

Multiplicative decay of a field:

```rust
Rule::Decay {
    field: "energy".into(),
    rate: 0.5,  // Halves per second
}
```

### Split

Particle division/reproduction when a condition is met:

```rust
// Cell division when energy is high
Rule::Split {
    condition: "p.energy > 1.5".into(),
    offspring_count: 2,
    offspring_type: None,            // Same type as parent
    resource_field: Some("energy".into()),
    resource_cost: 0.8,              // Costs 0.8 energy to split
    spread: std::f32::consts::PI / 4.0,
    speed_min: 0.1,
    speed_max: 0.3,
}

// Fragmentation on death
Rule::Split {
    condition: "p.health < 0.1".into(),
    offspring_count: 5,
    offspring_type: Some(2),         // Spawn as type 2 (fragments)
    resource_field: None,            // No cost
    resource_cost: 0.0,
    spread: std::f32::consts::TAU,   // Full sphere
    speed_min: 0.5,
    speed_max: 1.5,
}
```

Useful for biological simulations, explosions, and chain reactions. Requires the sub-emitter system.

---

## Visual Rules

Rules that modify appearance based on particle state.

### ColorOverLife

Lerp color from start to end over lifetime:

```rust
Rule::ColorOverLife {
    start: Vec3::new(1.0, 1.0, 0.0),  // Yellow
    end: Vec3::new(1.0, 0.0, 0.0),    // Red
    duration: 2.0,
}
```

### ColorBySpeed

Color gradient based on velocity:

```rust
Rule::ColorBySpeed {
    slow_color: Vec3::new(0.2, 0.3, 0.8),  // Blue
    fast_color: Vec3::new(1.0, 0.9, 0.5),  // Yellow
    max_speed: 2.0,
}
```

### ColorByAge

Color gradient based on age:

```rust
Rule::ColorByAge {
    young_color: Vec3::new(1.0, 1.0, 1.0),  // White
    old_color: Vec3::new(1.0, 0.3, 0.1),    // Red
    max_age: 3.0,
}
```

### ScaleBySpeed

Scale particles based on velocity:

```rust
Rule::ScaleBySpeed {
    min_scale: 0.5,
    max_scale: 2.0,
    max_speed: 3.0,
}
```

---

## Spring Rules

Structural constraints for cloth, ropes, and soft bodies.

### BondSprings

Spring forces between explicitly bonded particles:

```rust
#[derive(Particle)]
struct ClothPoint {
    position: Vec3,
    velocity: Vec3,
    bond_left: u32,   // u32::MAX = no bond
    bond_right: u32,
    bond_up: u32,
    bond_down: u32,
}

Rule::BondSprings {
    bonds: vec!["bond_left", "bond_right", "bond_up", "bond_down"],
    stiffness: 800.0,
    damping: 15.0,
    rest_length: 0.05,
    max_stretch: Some(1.3),  // Extra stiff past 130%
}
```

### ChainSprings

Automatic chain from sequential indices (ropes, tentacles):

```rust
Rule::ChainSprings {
    stiffness: 500.0,
    damping: 10.0,
    rest_length: 0.02,
    max_stretch: Some(1.2),
}
```

### RadialSprings

Hub-and-spoke structure (spider webs, wheels):

```rust
Rule::RadialSprings {
    hub_stiffness: 200.0,
    ring_stiffness: 100.0,
    damping: 5.0,
    hub_length: 0.3,
    ring_length: 0.1,
}
```

---

## Environment Rules

Environmental effects and respawning.

### Buoyancy

Height-based buoyancy force:

```rust
Rule::Buoyancy {
    surface_y: 0.0,
    density: 1.2,  // >1 floats, <1 sinks
}
```

### DensityBuoyancy

Per-particle density-based buoyancy:

```rust
Rule::DensityBuoyancy {
    density_field: "density".into(),
    medium_density: 1.0,
    strength: 5.0,
}
```

### Friction

Ground friction near a surface:

```rust
Rule::Friction {
    ground_y: -1.0,
    strength: 0.8,
    threshold: 0.1,
}
```

### Wind

Directional wind with turbulence:

```rust
Rule::Wind {
    direction: Vec3::new(1.0, 0.0, 0.0),
    strength: 2.0,
    turbulence: 0.3,
}
```

### Current

Follow a field as flow:

```rust
Rule::Current {
    field: "flow",
    strength: 2.0,
}
```

### RespawnBelow

Respawn particles that fall below threshold:

```rust
Rule::RespawnBelow {
    threshold_y: -1.0,
    spawn_y: 1.0,
    reset_velocity: true,
}
```

---

## State Rules

Finite state machines for complex behaviors.

### State

Simple FSM with conditional transitions:

```rust
Rule::State {
    field: "state".into(),
    transitions: vec![
        (0, 1, "p.age > 2.0".into()),   // young → mature
        (1, 2, "p.age > 5.0".into()),   // mature → old
        (2, 3, "p.age > 8.0".into()),   // old → dead
    ],
}
```

### Agent

Full-featured agent state machine with entry/exit/update actions:

```rust
Rule::Agent {
    state_field: "state".into(),
    prev_state_field: "prev_state".into(),
    state_timer_field: Some("state_timer".into()),
    states: vec![
        AgentState::new(0)
            .named("wandering")
            .on_enter(r#"p.color = vec3<f32>(0.3, 0.5, 0.8);"#)
            .on_update(r#"p.energy += 0.05 * uniforms.delta_time;"#)
            .transition(1, "p.food_nearby > 0.5"),

        AgentState::new(1)
            .named("chasing")
            .on_enter(r#"p.color = vec3<f32>(1.0, 0.3, 0.2);"#)
            .on_exit(r#"p.velocity *= 0.5;"#)
            .transition(0, "p.state_timer > 3.0"),
    ],
}
```

See [Agent State Machines](../advanced/agent-state-machines.md) for full documentation.

---

## Field Rules

Rules that interact with 3D spatial fields.

### Gradient

Move toward higher/lower field values (chemotaxis):

```rust
Rule::Gradient {
    field: 0,
    strength: 2.0,
    ascending: true,  // Move toward higher values
}
```

### Sync

Oscillator synchronization via field coupling (Kuramoto model):

```rust
Rule::Sync {
    phase_field: "phase".into(),
    frequency: 1.0,
    field: 0,
    emit_amount: 0.5,
    coupling: 0.3,
    detection_threshold: 0.1,
    on_fire: Some(r#"p.brightness = 1.0;"#.into()),
}
```

### Deposit

Write a particle field value to a 3D spatial field texture:

```rust
// Pheromone trail - particles leave scent at their position
Rule::Deposit {
    source_field: "pheromone".into(),
    target_field: 0,                    // Field index
    scale: 0.5,
    falloff: Some(Falloff::Smooth),
}

// Heat emission
Rule::Deposit {
    source_field: "temperature".into(),
    target_field: 1,
    scale: 1.0,
    falloff: None,
}
```

Particles write their field value to the 3D texture at their position. Use with `Gradient` to create feedback loops where particles respond to trails.

### Sense

Read field values at the particle's position into a particle property:

```rust
// Detect pheromone concentration
Rule::Sense {
    field: 0,
    target: "detected_pheromone".into(),
    scale: 1.0,
    offset: 0.0,
}

// Sample temperature field
Rule::Sense {
    field: 1,
    target: "local_temp".into(),
    scale: 2.0,
    offset: -1.0,  // output = field_value * 2.0 - 1.0
}
```

Great for letting particles "smell" their environment and make decisions based on field values.

### Consume

Read and deplete field values - like Sense but also clears the field:

```rust
// Eat food from the field
Rule::Consume {
    field: 0,
    target: "energy".into(),
    rate: 0.8,           // Take 80% of field value
    scale: 1.0,
}

// Absorb nutrients while depleting the source
Rule::Consume {
    field: 2,
    target: "nutrients".into(),
    rate: 1.0,           // Take all
    scale: 0.5,          // But only store half
}
```

Useful for foraging behaviors, resource competition, and grazing simulations where the field represents a depletable resource.

---

## Conditional Rules

Execute actions based on conditions.

### Maybe

Probabilistic action execution:

```rust
Rule::Maybe {
    probability: 0.01,  // 1% per frame
    action: r#"p.color = vec3<f32>(1.0, 0.0, 0.0);"#.into(),
}
```

### Trigger

Execute when condition is true:

```rust
Rule::Trigger {
    condition: "p.energy < 0.2".into(),
    action: r#"p.color = vec3<f32>(1.0, 0.0, 0.0);"#.into(),
}
```

### Periodic

Time-based periodic execution:

```rust
Rule::Periodic {
    interval: 0.5,
    phase_field: None,
    action: r#"field_write(0u, p.position, 1.0);"#.into(),
}
```

### Gate

Every-frame conditional (like Trigger but runs continuously):

```rust
Rule::Gate {
    condition: "p.energy > 0.8".into(),
    action: "p.velocity *= 1.5;".into(),
}
```

### Switch

Multi-branch conditional with configurable behavior switching:

```rust
// Mode-based behavior
Rule::Switch {
    field: "mode".into(),
    cases: vec![
        (0, "p.velocity += vec3<f32>(0.0, -1.0, 0.0);".into()),  // Falling
        (1, "p.velocity *= 0.95;".into()),                        // Resting
        (2, "p.velocity.y += 0.5;".into()),                       // Rising
    ],
    default: Some("p.velocity = vec3<f32>(0.0);".into()),
}

// State-driven animation
Rule::Switch {
    field: "state".into(),
    cases: vec![
        (0, r#"p.color = vec3<f32>(0.2, 0.8, 0.2);"#.into()),  // Healthy: green
        (1, r#"p.color = vec3<f32>(0.8, 0.8, 0.2);"#.into()),  // Sick: yellow
        (2, r#"p.color = vec3<f32>(0.8, 0.2, 0.2);"#.into()),  // Critical: red
    ],
    default: None,
}
```

More readable than nested Gate rules when dispatching on a discrete state or mode field. Each case executes only when the field equals that integer value.

---

## Signal Processing Rules

Transform and manipulate particle fields.

### Lerp

Smoothly interpolate toward a target:

```rust
Rule::Lerp {
    field: "temperature".into(),
    target: 0.5,
    rate: 2.0,
}
```

### Tween

Animate a property over time:

```rust
Rule::Tween {
    field: "scale".into(),
    from: 0.0,
    to: 1.0,
    duration: 0.5,
    timer_field: "age".into(),
}
```

### Threshold

Binary step function:

```rust
Rule::Threshold {
    input_field: "health".into(),
    output_field: "alive".into(),
    threshold: 0.0,
    above: 1.0,
    below: 0.0,
}
```

### Noise

Add procedural noise to a field:

```rust
Rule::Noise {
    field: "brightness".into(),
    amplitude: 0.3,
    frequency: 2.0,
    time_scale: 5.0,
}
```

### Remap

Linear range remapping:

```rust
Rule::Remap {
    field: "opacity".into(),
    in_min: 0.0, in_max: 10.0,
    out_min: 1.0, out_max: 0.0,
}
```

### Clamp

Clamp to range:

```rust
Rule::Clamp {
    field: "energy".into(),
    min: 0.0,
    max: 100.0,
}
```

### Smooth

Exponential smoothing:

```rust
Rule::Smooth {
    field: "brightness".into(),
    target: 0.0,
    rate: 0.1,
}
```

### Quantize

Snap to discrete steps:

```rust
Rule::Quantize {
    field: "position.x".into(),
    step: 0.1,
}
```

### Modulo

Wrap value within range:

```rust
Rule::Modulo {
    field: "phase".into(),
    min: 0.0,
    max: 6.28318,  // 2π
}
```

### Copy

Copy field with optional scale/offset:

```rust
Rule::Copy {
    from: "health".into(),
    to: "damage".into(),
    scale: -1.0,
    offset: 100.0,  // damage = 100 - health
}
```

### Mass

Scale accelerations by inverse mass (F=ma):

```rust
Rule::Mass { field: "mass".into() }
```

---

## Logic Gate Rules

Analog-style logic operations on fields.

### And

Minimum of two fields:

```rust
Rule::And {
    a: "has_energy".into(),
    b: "is_ready".into(),
    output: "can_fire".into(),
}
```

### Or

Maximum of two fields:

```rust
Rule::Or {
    a: "danger_left".into(),
    b: "danger_right".into(),
    output: "any_danger".into(),
}
```

### Not

Inversion:

```rust
Rule::Not {
    input: "alive".into(),
    output: "dead".into(),
    max: 1.0,
}
```

### Xor

Absolute difference (high when inputs differ):

```rust
Rule::Xor {
    a: "signal_a".into(),
    b: "signal_b".into(),
    output: "mismatch".into(),
}
```

### Hysteresis

Schmitt trigger - prevents oscillation:

```rust
Rule::Hysteresis {
    input: "temperature".into(),
    output: "heater_on".into(),
    low_threshold: 18.0,   // Turn off below
    high_threshold: 22.0,  // Turn on above
    on_value: 1.0,
    off_value: 0.0,
}
```

### Latch

SR flip-flop - persistent memory:

```rust
Rule::Latch {
    output: "alarm".into(),
    set_condition: "p.danger > 0.9".into(),
    reset_condition: "p.acknowledged > 0.5".into(),
    set_value: 1.0,
    reset_value: 0.0,
}
```

### Edge

Detect threshold crossings:

```rust
Rule::Edge {
    input: "energy".into(),
    prev_field: "energy_prev".into(),
    output: "energy_crossed".into(),
    threshold: 0.5,
    rising: true,
    falling: true,
}
```

### Select

Ternary operator:

```rust
Rule::Select {
    condition: "p.is_fleeing > 0.5".into(),
    then_field: "fast_speed".into(),
    else_field: "normal_speed".into(),
    output: "current_speed".into(),
}
```

### Blend

Mix two fields by weight:

```rust
Rule::Blend {
    a: "cold_color".into(),
    b: "hot_color".into(),
    weight: "temperature_normalized".into(),
    output: "display_color".into(),
}
```

---

## Custom Rules

For anything not built-in, write raw WGSL.

### Custom

Per-particle custom behavior:

```rust
Rule::Custom(r#"
    // Access particle as 'p'
    p.velocity.y += sin(uniforms.time) * 0.1;

    // Available variables:
    // - p: current particle (read/write)
    // - index: particle index (u32)
    // - uniforms.time: elapsed time (f32)
    // - uniforms.delta_time: frame time (f32)
"#.to_string())
```

### NeighborCustom

Custom neighbor interactions (requires spatial hashing):

```rust
Rule::NeighborCustom(r#"
    // Available variables:
    // - p: current particle (read/write)
    // - other: neighbor particle (read-only)
    // - neighbor_dist: distance to neighbor
    // - neighbor_dir: unit vector from neighbor to self
    // - neighbor_pos, neighbor_vel: neighbor state
    // - index, other_idx: particle indices

    if neighbor_dist < 0.2 && neighbor_dist > 0.01 {
        let force = 0.5 / (neighbor_dist * neighbor_dist);
        p.velocity -= neighbor_dir * force * uniforms.delta_time;
    }
"#.into())
```

### OnCollision

Custom collision response:

```rust
Rule::OnCollision {
    radius: 0.05,
    response: r#"
        // Additional variables:
        // - overlap: penetration depth
        // - rel_vel: relative velocity along normal

        if p.particle_type != other.particle_type {
            p.velocity += neighbor_dir * rel_vel * 2.0;
        }
    "#.into(),
}
```

---

## Rule Order

Rules execute in the order added. A typical order:

```rust
.with_rule(Rule::Gravity(9.8))           // 1. Apply forces
.with_rule(Rule::Wander { ... })         // 2. Random movement
.with_rule(Rule::Separate { ... })       // 3. Neighbor interactions
.with_rule(Rule::Cohere { ... })
.with_rule(Rule::SpeedLimit { ... })     // 4. Clamp velocity
.with_rule(Rule::Drag(1.0))              // 5. Apply drag
.with_rule(Rule::BounceWalls)            // 6. Boundary conditions
```

Velocity integration (`position += velocity * dt`) happens automatically after all rules.

---

## Spatial Configuration

For neighbor rules, configure the spatial hash:

```rust
.with_spatial_config(cell_size, grid_resolution)
```

- **cell_size**: Should be >= your largest interaction radius
- **grid_resolution**: Must be power of 2 (16, 32, 64, etc.)

Example for bounds of 1.0 with max interaction radius of 0.1:

```rust
.with_bounds(1.0)
.with_spatial_config(0.1, 32)  // 32³ cells
```
