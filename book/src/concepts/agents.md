# Particles as Agents

RDPE particles aren't just physics objects—they can be autonomous **agents** with memory, perception, relationships, and decision-making. This page explains how existing primitives map to agent concepts.

## The Agent Model

Traditional agent-based systems have:

| Agent Concept     | Description                       |
|-------------------|-----------------------------------|
| **Memory**        | State that persists across frames |
| **Perception**    | What the agent can sense          |
| **Relationships** | Connections to other agents       |
| **Behaviors**     | Decision-making and actions       |
| **Communication** | Information exchange              |

RDPE provides all of these through its existing primitives.

## Memory: Particle Fields

Any custom field on your particle struct is persistent memory:

```rust
#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,

    // Memory fields
    hunger: f32,           // Internal state
    fear_level: f32,       // Emotional state
    age: f32,              // Lifetime tracking
    last_seen_food: Vec3,  // Remembered location
    state: u32,            // State machine state
}
```

These persist frame-to-frame and can be read/written in rules:

```rust
.with_rule(Rule::Custom(r#"
    // Update internal state
    p.hunger += uniforms.delta_time * 0.1;
    p.age += uniforms.delta_time;

    // Decay fear over time
    p.fear_level *= 0.99;
"#.into()))
```

## Perception: Sensing the World

### Neighbors (Local Perception)

Spatial hashing lets agents sense nearby entities:

```rust
.with_spatial_config(0.3, 32)
.with_rule(Rule::NeighborCustom(r#"
    // Can I see food nearby?
    if other.particle_type == 1u && neighbor_dist < 0.2 {
        // Remember where food is
        p.last_seen_food = other.position;
        p.hunger -= 0.01;  // Eat!
    }

    // Is there a predator nearby?
    if other.particle_type == 2u && neighbor_dist < 0.3 {
        p.fear_level = 1.0;  // Panic!
    }
"#.into()))
```

### Fields (Environmental Perception)

3D fields provide environmental information:

```rust
.with_field("temperature", 32, |x, y, z| {
    // Warmer at center
    1.0 - (x*x + y*y + z*z).sqrt()
})

.with_rule(Rule::Custom(r#"
    let temp = field_temperature(p.position);
    if temp < 0.3 {
        // Too cold - seek warmth
        p.velocity.y += 0.1 * uniforms.delta_time;
    }
"#.into()))
```

### Direct Access (Specific Knowledge)

Particles can read any other particle directly:

```rust
.with_rule(Rule::Custom(r#"
    // Check on my leader (stored index)
    if p.leader_id != 4294967295u {
        let leader = particles[p.leader_id];
        let to_leader = leader.position - p.position;
        p.velocity += normalize(to_leader) * 0.5 * uniforms.delta_time;
    }
"#.into()))
```

## Relationships: Persistent Connections

### Bond Indices

Store indices of related particles:

```rust
#[derive(Particle, Clone)]
struct SocialCreature {
    position: Vec3,
    velocity: Vec3,

    // Relationships
    parent_id: u32,        // Who spawned me
    friend_ids: [u32; 4],  // Social connections
    enemy_id: u32,         // Current rival
    leader_id: u32,        // Pack leader
}
```

### Using `Rule::BondSprings`

For physical connections (cloth, ropes, molecules):

```rust
.with_rule(Rule::BondSprings {
    bonds: vec!["bond_left", "bond_right", "bond_up", "bond_down"],
    stiffness: 800.0,
    damping: 15.0,
    rest_length: 0.05,
    max_stretch: Some(1.3),
})
```

### Interaction Matrix

Type-based relationships:

```rust
.with_interactions(|m| {
    m.attract(Prey, Prey, 0.3, 0.2);      // Prey flocks
    m.repel(Prey, Predator, 1.0, 0.4);    // Prey flees predators
    m.attract(Predator, Prey, 0.8, 0.5);  // Predators hunt prey
})
```

## Behaviors: Decision Making

### State Machines

Use a `state` field for behavioral modes:

```rust
const STATE_IDLE: u32 = 0;
const STATE_SEEKING: u32 = 1;
const STATE_FLEEING: u32 = 2;
const STATE_EATING: u32 = 3;

.with_rule(Rule::Custom(r#"
    // State transitions
    if p.state == 0u {  // IDLE
        if p.hunger > 0.7 {
            p.state = 1u;  // -> SEEKING
        }
        if p.fear_level > 0.5 {
            p.state = 2u;  // -> FLEEING
        }
    }
    else if p.state == 1u {  // SEEKING
        // Move toward remembered food location
        let to_food = p.last_seen_food - p.position;
        if length(to_food) > 0.01 {
            p.velocity += normalize(to_food) * 0.3 * uniforms.delta_time;
        }

        if p.hunger < 0.3 {
            p.state = 0u;  // -> IDLE (full)
        }
        if p.fear_level > 0.5 {
            p.state = 2u;  // -> FLEEING (danger!)
        }
    }
    else if p.state == 2u {  // FLEEING
        // Run away from threat (handled in neighbor rule)
        p.velocity *= 1.5;  // Sprint!

        if p.fear_level < 0.1 {
            p.state = 0u;  // -> IDLE (safe)
        }
    }
"#.into()))
```

### Conditional Behaviors

Simple if/else logic:

```rust
.with_rule(Rule::Custom(r#"
    let speed = length(p.velocity);

    // Tired? Slow down
    if p.energy < 0.2 {
        p.velocity *= 0.95;
    }

    // Old? Change color
    if p.age > 10.0 {
        p.color = mix(p.color, vec3<f32>(0.5, 0.5, 0.5), 0.01);
    }

    // Hungry and near food? Eat
    // (food detection happens in neighbor rule)
"#.into()))
```

## Communication: Information Exchange

### Inboxes (Direct Messages)

Particles can send targeted messages:

```rust
#[derive(Particle, Clone)]
struct Messenger {
    position: Vec3,
    velocity: Vec3,
    inbox: u32,  // Receives messages here
}

.with_inbox("inbox")
.with_rule(Rule::NeighborCustom(r#"
    // Send alert to nearby friends
    if p.fear_level > 0.8 && other.particle_type == p.particle_type {
        send_inbox(other_idx, 1u);  // "Danger!"
    }
"#.into()))

.with_rule(Rule::Custom(r#"
    // React to received messages
    if p.inbox == 1u {
        p.fear_level = max(p.fear_level, 0.5);
        p.inbox = 0u;  // Clear inbox
    }
"#.into()))
```

### Fields (Broadcast)

Write to fields for area-of-effect communication:

```rust
.with_field_writable("pheromone", 32, |_, _, _| 0.0)

// Leave pheromone trail
.with_rule(Rule::Custom(r#"
    if p.found_food > 0.0 {
        field_pheromone_add(p.position, 1.0);
    }
"#.into()))

// Follow pheromone gradient
.with_rule(Rule::Custom(r#"
    let gradient = field_pheromone_gradient(p.position);
    p.velocity += gradient * 0.2 * uniforms.delta_time;
"#.into()))
```

## Complete Example: Ecosystem

Here's a full agent-based ecosystem:

```rust
#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,  // 0=plant, 1=herbivore, 2=predator
    energy: f32,
    age: f32,
    state: u32,
}

Simulation::<Creature>::new()
    .with_particle_count(2000)
    .with_spawner(|i, _| {
        let creature_type = (i % 10) as u32;  // Mix of types
        Creature {
            position: random_position(),
            velocity: Vec3::ZERO,
            color: match creature_type {
                0 => Vec3::new(0.2, 0.8, 0.2),  // Plants: green
                1 => Vec3::new(0.2, 0.5, 0.9),  // Herbivores: blue
                _ => Vec3::new(0.9, 0.2, 0.2),  // Predators: red
            },
            particle_type: creature_type.min(2),
            energy: 1.0,
            age: 0.0,
            state: 0,
        }
    })
    .with_spatial_config(0.3, 32)

    // Type-based interactions
    .with_interactions(|m| {
        // Herbivores eat plants, flock together
        m.attract(1, 0, 0.5, 0.2);   // Herbivore -> Plant
        m.attract(1, 1, 0.2, 0.15);  // Herbivore -> Herbivore
        m.repel(1, 2, 0.8, 0.3);     // Herbivore <- Predator

        // Predators hunt herbivores
        m.attract(2, 1, 0.7, 0.4);   // Predator -> Herbivore
        m.repel(2, 2, 0.3, 0.2);     // Predators spread out
    })

    // Energy and aging
    .with_rule(Rule::Custom(r#"
        p.age += uniforms.delta_time;

        // Plants don't move, slowly regenerate
        if p.particle_type == 0u {
            p.velocity = vec3<f32>(0.0);
            p.energy = min(p.energy + uniforms.delta_time * 0.1, 1.0);
        } else {
            // Animals burn energy moving
            p.energy -= length(p.velocity) * uniforms.delta_time * 0.01;
        }

        // Color reflects energy
        let energy_color = mix(vec3<f32>(0.3), p.color, p.energy);
        p.color = energy_color;
    "#.into()))

    // Eating (in neighbor loop)
    .with_rule(Rule::NeighborCustom(r#"
        // Herbivores eat plants
        if p.particle_type == 1u && other.particle_type == 0u && neighbor_dist < 0.05 {
            p.energy = min(p.energy + 0.1, 1.0);
        }

        // Predators eat herbivores
        if p.particle_type == 2u && other.particle_type == 1u && neighbor_dist < 0.05 {
            p.energy = min(p.energy + 0.2, 1.0);
        }
    "#.into()))

    .with_rule(Rule::Drag(1.0))
    .with_rule(Rule::WrapWalls)
    .run();
```

## Design Patterns

### Pattern: Finite State Machine

```rust
// States as constants
const WANDER: u32 = 0;
const CHASE: u32 = 1;
const FLEE: u32 = 2;
const REST: u32 = 3;

// State transitions based on conditions
// Actions based on current state
```

### Pattern: Blackboard (Shared Memory via Fields)

```rust
// Global information in fields
.with_field_writable("danger_zone", 16, |_,_,_| 0.0)

// Agents write when they spot danger
// Other agents read and react
```

### Pattern: Stigmergy (Indirect Communication)

```rust
// Pheromone trails
// Agents modify environment
// Other agents sense modifications
// No direct communication needed
```

## Performance Considerations

1. **State machines are cheap** - Integer comparisons are fast
2. **Memory fields add bandwidth** - Each field increases particle size
3. **Neighbor perception is expensive** - Spatial queries dominate cost
4. **Direct access is fast** - `particles[index]` is a single read
5. **Fields are moderate** - 3D texture lookups have some cost

## Summary

RDPE particles are agents when you use them as agents:

| Agent Need         | RDPE Solution                  |
|--------------------|--------------------------------|
| Memory             | Particle fields                |
| Local perception   | Neighbor queries               |
| Global perception  | Fields                         |
| Specific knowledge | Direct buffer access           |
| Physical bonds     | `Rule::BondSprings`            |
| Type relationships | Interaction matrix             |
| Decisions          | Custom rules with conditionals |
| Direct messages    | Inboxes                        |
| Broadcast          | Writable fields                |

No special "Agent" API needed—the primitives compose into whatever agent architecture your simulation requires.
