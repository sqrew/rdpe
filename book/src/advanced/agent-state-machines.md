# Agent State Machines

`Rule::Agent` provides a declarative finite state machine (FSM) for particles. Each particle can be in one of several states, with transitions triggered by conditions and actions that run on state changes.

## Why State Machines?

State machines organize complex behaviors into manageable chunks:

| Without State Machine | With State Machine |
|-----------------------|-------------------|
| Giant if/else chains | Separate state definitions |
| Spaghetti transitions | Explicit transition rules |
| Hard to add states | Just add another `AgentState` |
| Debugging nightmare | Clear state names, entry/exit hooks |

## Quick Example

A creature that wanders, chases food, eats, then rests:

```rust
#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,

    // Required state machine fields
    state: u32,           // Current state ID
    prev_state: u32,      // Previous state (for edge detection)
    state_timer: f32,     // Time in current state

    // Your custom fields
    energy: f32,
    food_nearby: f32,
}

// State constants (for readability)
const WANDERING: u32 = 0;
const CHASING: u32 = 1;
const EATING: u32 = 2;
const RESTING: u32 = 3;

Simulation::<Creature>::new()
    .with_rule(Rule::Agent {
        state_field: "state".into(),
        prev_state_field: "prev_state".into(),
        state_timer_field: Some("state_timer".into()),
        states: vec![
            AgentState::new(WANDERING)
                .named("wandering")
                .on_enter(r#"p.color = vec3<f32>(0.3, 0.5, 0.8);"#)
                .on_update(r#"p.energy = min(1.0, p.energy + 0.05 * uniforms.delta_time);"#)
                .transition(CHASING, "p.food_nearby > 0.5 && p.energy > 0.3")
                .transition(RESTING, "p.energy < 0.15"),

            AgentState::new(CHASING)
                .named("chasing")
                .on_enter(r#"p.color = vec3<f32>(1.0, 0.3, 0.2);"#)
                .on_update(r#"p.energy -= 0.15 * uniforms.delta_time;"#)
                .on_exit(r#"p.velocity *= 0.5;"#)  // Slow down when stopping
                .transition(EATING, "p.food_nearby > 0.9")
                .transition(WANDERING, "p.food_nearby < 0.3")
                .transition(RESTING, "p.energy < 0.1"),

            AgentState::new(EATING)
                .named("eating")
                .on_enter(r#"
                    p.color = vec3<f32>(0.2, 0.9, 0.3);
                    p.velocity = vec3<f32>(0.0);  // Stop moving
                "#)
                .on_update(r#"
                    p.energy = min(1.0, p.energy + 0.4 * uniforms.delta_time);
                    p.scale = 1.0 + sin(p.state_timer * 10.0) * 0.3;  // Pulse
                "#)
                .on_exit(r#"p.scale = 1.0;"#)
                .transition(WANDERING, "p.state_timer > 1.5"),  // Done eating

            AgentState::new(RESTING)
                .named("resting")
                .on_enter(r#"
                    p.color = vec3<f32>(0.5, 0.5, 0.5);
                    p.velocity *= 0.1;
                "#)
                .on_update(r#"
                    p.energy = min(1.0, p.energy + 0.15 * uniforms.delta_time);
                "#)
                .transition(WANDERING, "p.energy > 0.6"),
        ],
    })
    .run();
```

## Required Particle Fields

Your particle struct must include these fields:

```rust
#[derive(Particle, Clone)]
struct MyAgent {
    position: Vec3,
    velocity: Vec3,

    // REQUIRED for Rule::Agent
    state: u32,           // Current state ID (0, 1, 2, ...)
    prev_state: u32,      // Previous frame's state

    // OPTIONAL but recommended
    state_timer: f32,     // Auto-incremented time in current state
}
```

| Field | Type | Purpose |
|-------|------|---------|
| `state` | `u32` | Current state ID. You set the initial value in spawner. |
| `prev_state` | `u32` | Stores last frame's state. Used to detect state changes. |
| `state_timer` | `f32` | Time spent in current state. Resets to 0 on transitions. |

## Rule::Agent Structure

```rust
Rule::Agent {
    state_field: String,            // Name of the state u32 field
    prev_state_field: String,       // Name of the prev_state u32 field
    state_timer_field: Option<String>,  // Optional timer f32 field
    states: Vec<AgentState>,        // State definitions
}
```

## AgentState API

### Creating a State

```rust
AgentState::new(id)              // Create state with numeric ID
    .named("state_name")         // Optional name for debugging
    .on_enter(wgsl_code)         // Runs once when entering
    .on_update(wgsl_code)        // Runs every frame while in state
    .on_exit(wgsl_code)          // Runs once when leaving
    .transition(target_id, condition)  // Add transition
```

### Transition Conditions

Transitions are WGSL boolean expressions:

```rust
// Simple field comparisons
.transition(1, "p.energy < 0.2")
.transition(2, "p.health <= 0.0")

// Using state timer
.transition(0, "p.state_timer > 3.0")  // Leave after 3 seconds

// Compound conditions
.transition(1, "p.food_nearby > 0.5 && p.energy > 0.3")
.transition(2, "p.fear_level > 0.8 || p.health < 0.1")

// Using uniforms
.transition(1, "uniforms.time > 10.0")  // After 10 seconds total
```

### Transition Priority

When multiple transitions could fire, use priority:

```rust
AgentState::new(0)
    // High priority: emergency transitions checked first
    .transition_priority(DEAD, "p.health <= 0.0", 100)
    .transition_priority(FLEEING, "p.fear > 0.9", 50)
    // Normal priority (default = 0)
    .transition(SEEKING, "p.hunger > 0.7")
    .transition(IDLE, "true")  // Fallback
```

Higher priority transitions are checked first. First matching transition wins.

## Execution Order

Each frame, for each particle:

1. **Detect state change**: Compare `state` to `prev_state`
2. **Exit action**: If changed, run old state's `on_exit`
3. **Entry action**: If changed, run new state's `on_enter`
4. **Update prev_state**: Set `prev_state = state`
5. **Update action**: Run current state's `on_update`
6. **Increment timer**: Add `delta_time` to `state_timer`
7. **Check transitions**: Evaluate in priority order, first match updates `state`
8. **Reset timer**: If state changed, set `state_timer = 0`

Note: Entry/exit actions run on the *next* frame after transition, not immediately.

## Available Variables in WGSL

Inside `on_enter`, `on_update`, `on_exit`, and transition conditions:

```wgsl
p                    // Current particle (read/write)
p.state              // Current state ID
p.prev_state         // Previous state ID
p.state_timer        // Time in current state
p.position           // Particle position
p.velocity           // Particle velocity
// ... all your custom fields

uniforms.time        // Total elapsed time
uniforms.delta_time  // Frame delta time
index                // Particle index
```

## Common Patterns

### Pattern: Timed States

Stay in a state for a fixed duration:

```rust
AgentState::new(STUNNED)
    .on_enter(r#"p.velocity = vec3<f32>(0.0);"#)
    .on_update(r#"
        // Flash effect while stunned
        p.color = mix(vec3<f32>(1.0), vec3<f32>(0.5), sin(p.state_timer * 20.0) * 0.5 + 0.5);
    "#)
    .transition(IDLE, "p.state_timer > 2.0")  // Recover after 2 seconds
```

### Pattern: Cooldown

Prevent rapid state changes:

```rust
AgentState::new(ATTACKING)
    .on_enter(r#"p.attack_cooldown = 0.5;"#)  // Set cooldown
    .transition(IDLE, "p.state_timer > 0.3")   // Attack animation duration

AgentState::new(IDLE)
    .on_update(r#"p.attack_cooldown = max(0.0, p.attack_cooldown - uniforms.delta_time);"#)
    .transition(ATTACKING, "p.target_in_range > 0.5 && p.attack_cooldown <= 0.0")
```

### Pattern: State-Specific Colors

Visual feedback for debugging:

```rust
AgentState::new(IDLE)
    .on_enter(r#"p.color = vec3<f32>(0.3, 0.5, 0.8);"#)  // Blue

AgentState::new(ALERT)
    .on_enter(r#"p.color = vec3<f32>(1.0, 1.0, 0.2);"#)  // Yellow

AgentState::new(ATTACKING)
    .on_enter(r#"p.color = vec3<f32>(1.0, 0.2, 0.2);"#)  // Red

AgentState::new(FLEEING)
    .on_enter(r#"p.color = vec3<f32>(0.8, 0.5, 1.0);"#)  // Purple
```

### Pattern: Hierarchical States (Simulated)

Use state ranges for sub-states:

```rust
// States 0-9: Idle variants
// States 10-19: Combat variants
// States 20-29: Social variants

const IDLE_STANDING: u32 = 0;
const IDLE_SITTING: u32 = 1;
const IDLE_SLEEPING: u32 = 2;

const COMBAT_CHASING: u32 = 10;
const COMBAT_ATTACKING: u32 = 11;
const COMBAT_RETREATING: u32 = 12;

// Check state category in conditions:
// "p.state >= 10u && p.state < 20u"  // In combat?
```

### Pattern: Random Transitions

Add randomness to state changes:

```rust
AgentState::new(IDLE)
    .on_update(r#"
        // Random chance to transition
        let roll = rand(index + u32(uniforms.time * 1000.0));
        if roll < 0.001 {  // ~0.1% per frame
            p.wants_to_move = 1.0;
        }
    "#)
    .transition(WANDERING, "p.wants_to_move > 0.5")
```

## Combining with Other Rules

`Rule::Agent` works alongside other rules. Place it strategically:

```rust
Simulation::<Creature>::new()
    // 1. Update sensory inputs (before state machine)
    .with_rule(Rule::Custom(r#"
        // Simulate food detection
        let food_pos = vec3<f32>(sin(uniforms.time * 0.3) * 0.5, 0.0, cos(uniforms.time * 0.4) * 0.5);
        p.food_dist = length(p.position - food_pos);
        p.food_nearby = select(0.0, 1.0, p.food_dist < 0.4);
    "#.into()))

    // 2. State machine (makes decisions)
    .with_rule(Rule::Agent { ... })

    // 3. State-independent behaviors
    .with_rule(Rule::Wander { strength: 0.5, frequency: 2.0 })
    .with_rule(Rule::Drag(1.5))
    .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.0 })
    .with_rule(Rule::BounceWalls)

    // 4. Visual adjustments (after state machine)
    .with_rule(Rule::Custom(r#"
        // Dim color when low energy
        p.color *= (0.5 + p.energy * 0.5);
    "#.into()))
```

## Performance

State machines are cheap:

- **State storage**: 8-12 bytes per particle (state, prev_state, timer)
- **Execution**: A few integer comparisons + one code block per frame
- **Transitions**: Checked in order; first match exits early

For thousands of states, consider using `particle_type` for broad categories and `state` for sub-behaviors within each type.

## Debugging Tips

1. **Use state names**: `.named("wandering")` helps when debugging shader code
2. **Color by state**: Set `p.color` in each `on_enter` for visual debugging
3. **Print state timer**: Watch how long particles stay in each state
4. **Start simple**: Get 2-3 states working before adding more
5. **Check transitions**: If stuck in a state, verify the exit condition can become true

## Complete Example

See `examples/agent_demo.rs` for a full working implementation with:
- 4 states (wandering, chasing, eating, resting)
- Energy management
- Visual feedback
- State-dependent movement

Run it with:
```bash
cargo run --example agent_demo
```
