# Typed Interactions

Typed interactions let different particle types behave differently toward each other. This enables predator-prey dynamics, team-based systems, and state machines.

## Defining Particle Types

Use `#[derive(ParticleType)]` to create a type-safe enum:

```rust
#[derive(ParticleType, Clone, Copy, PartialEq)]
enum Species {
    Prey,      // = 0
    Predator,  // = 1
}
```

The derive macro automatically:
- Implements `Into<u32>` (variants get sequential IDs: 0, 1, 2...)
- Implements `From<u32>` (convert back from runtime values)
- Adds a `count()` method

Every particle has a `particle_type: u32` field. If you don't add it, it's auto-added with value 0.

```rust
#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
}
```

Set types in the spawner:

```rust
.with_spawner(|i, count| {
    let species = if i < 50 { Species::Predator } else { Species::Prey };
    Creature {
        position: random_pos(),
        velocity: Vec3::ZERO,
        particle_type: species.into(),
    }
})
```

## Chase & Evade

For predator-prey dynamics, use the dedicated rules:

```rust
// Predators chase nearest prey
.with_rule(Rule::Chase {
    self_type: Species::Predator.into(),
    target_type: Species::Prey.into(),
    radius: 0.4,
    strength: 4.0,
})

// Prey evades nearest predator
.with_rule(Rule::Evade {
    self_type: Species::Prey.into(),
    threat_type: Species::Predator.into(),
    radius: 0.25,
    strength: 6.0,
})
```

These find the **nearest** target/threat and steer directly toward/away from it.

## The Typed Wrapper

`Rule::Typed` wraps any neighbor rule with type filters:

```rust
Rule::Typed {
    self_type: u32,           // Which particles this rule affects
    other_type: Option<u32>,  // Which neighbors to consider
    rule: Box<Rule>,          // The wrapped rule
}
```

### Example: Prey Flocking

```rust
// Prey flocks with other prey
.with_rule(Rule::Typed {
    self_type: Species::Prey.into(),
    other_type: Some(Species::Prey.into()),
    rule: Box::new(Rule::Cohere { radius: 0.15, strength: 1.0 }),
})
```

### Interacting with All Types

Use `other_type: None` to interact with everyone:

```rust
// Everyone avoids collisions with everyone
.with_rule(Rule::Typed {
    self_type: Species::Prey.into(),
    other_type: None,  // All types
    rule: Box::new(Rule::Collide { radius: 0.05, response: 0.5 }),
})
```

## Type Conversion

`Rule::Convert` changes particle types at runtime:

```rust
#[derive(ParticleType, Clone, Copy, PartialEq)]
enum Health {
    Healthy,
    Infected,
    Recovered,
}

// Healthy can become infected
.with_rule(Rule::Convert {
    from_type: Health::Healthy.into(),
    trigger_type: Health::Infected.into(),
    to_type: Health::Infected.into(),
    radius: 0.08,
    probability: 0.15,
})

// Infected eventually recover
.with_rule(Rule::Convert {
    from_type: Health::Infected.into(),
    trigger_type: Health::Infected.into(),  // Self-trigger
    to_type: Health::Recovered.into(),
    radius: 0.01,
    probability: 0.002,
})
```

## Updating Visuals

When types change, you'll want colors to update. Use `Rule::Custom`:

```rust
.with_rule(Rule::Custom(r#"
    if p.particle_type == 0u {
        p.color = vec3<f32>(0.1, 0.9, 0.2); // Green
    } else if p.particle_type == 1u {
        p.color = vec3<f32>(1.0, 0.1, 0.1); // Red
    } else {
        p.color = vec3<f32>(0.2, 0.4, 1.0); // Blue
    }
"#.to_string()))
```

## Use Cases

| Scenario | Types | Interactions |
|----------|-------|--------------|
| Predator-Prey | Predator, Prey | Chase/Evade rules |
| Infection | Healthy, Infected, Recovered | Convert rules for spread |
| Charged Particles | Positive, Negative | Opposites attract, same repels |
| Food Chain | Plant, Herbivore, Carnivore | Each level hunts the one below |
| Teams | Team A, Team B | Same team coheres, enemies separate |
| Life Stages | Young, Adult, Elder | Convert based on age |

## Performance Note

Typed rules add conditional checks inside the neighbor loop. For best performance:

- Use fewer distinct types when possible
- Group related type interactions
- Consider if untyped rules with Custom code might be simpler
