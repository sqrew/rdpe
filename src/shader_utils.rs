//! Built-in WGSL utility functions for shader code.
//!
//! These functions are automatically available in all compute shaders.
//! You can call them from `Rule::Custom` or custom functions added via
//! [`Simulation::with_function`].
//!
//! # Available Functions
//!
//! ## Random & Hash
//! - `hash(n: u32) -> u32` - Hash a u32 to pseudo-random u32
//! - `hash2(p: vec2<u32>) -> u32` - Hash a 2D coordinate to pseudo-random u32
//! - `hash3(p: vec3<u32>) -> u32` - Hash a 3D coordinate to pseudo-random u32
//! - `rand(seed: u32) -> f32` - Returns random float in [0, 1)
//! - `rand_range(seed: u32, min: f32, max: f32) -> f32` - Random float in range
//! - `rand_vec3(seed: u32) -> vec3<f32>` - Random unit vector
//!
//! ## Noise
//! - `noise2(p: vec2<f32>) -> f32` - 2D gradient noise in [-1, 1]
//! - `noise3(p: vec3<f32>) -> f32` - 3D gradient noise in [-1, 1]
//! - `fbm2(p: vec2<f32>, octaves: i32) -> f32` - 2D fractal Brownian motion
//! - `fbm3(p: vec3<f32>, octaves: i32) -> f32` - 3D fractal Brownian motion
//!
//! ## Color
//! - `hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32>` - Convert HSV to RGB
//! - `rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32>` - Convert RGB to HSV
//!
//! # Example
//!
//! ```ignore
//! .with_rule(Rule::Custom(r#"
//!     // Random color based on particle index
//!     let seed = index * 12345u;
//!     p.color = hsv_to_rgb(rand(seed), 0.8, 1.0);
//!
//!     // Add noise-based force
//!     let noise_force = vec3<f32>(
//!         noise3(p.position * 3.0 + uniforms.time),
//!         noise3(p.position * 3.0 + uniforms.time + vec3(100.0, 0.0, 0.0)),
//!         noise3(p.position * 3.0 + uniforms.time + vec3(0.0, 100.0, 0.0))
//!     );
//!     p.velocity += noise_force * 0.1;
//! "#.into()))
//! ```

/// WGSL code for random/hash functions.
pub const RANDOM_WGSL: &str = r#"
// Hash functions for pseudo-random number generation
fn hash(n: u32) -> u32 {
    var x = n;
    x = x ^ (x >> 17u);
    x = x * 0xed5ad4bbu;
    x = x ^ (x >> 11u);
    x = x * 0xac4c1b51u;
    x = x ^ (x >> 15u);
    x = x * 0x31848babu;
    x = x ^ (x >> 14u);
    return x;
}

fn hash2(p: vec2<u32>) -> u32 {
    return hash(p.x + hash(p.y));
}

fn hash3(p: vec3<u32>) -> u32 {
    return hash(p.x + hash(p.y + hash(p.z)));
}

// Random float in [0, 1)
fn rand(seed: u32) -> f32 {
    return f32(hash(seed)) / 4294967295.0;
}

// Random float in [min, max)
fn rand_range(seed: u32, min_val: f32, max_val: f32) -> f32 {
    return min_val + rand(seed) * (max_val - min_val);
}

// Random direction vector (not normalized)
fn rand_vec3(seed: u32) -> vec3<f32> {
    return vec3<f32>(
        rand(seed) * 2.0 - 1.0,
        rand(seed + 1u) * 2.0 - 1.0,
        rand(seed + 2u) * 2.0 - 1.0
    );
}

// Random unit sphere point (normalized)
fn rand_sphere(seed: u32) -> vec3<f32> {
    // Use rejection sampling conceptually, but approximate with normalization
    let v = rand_vec3(seed);
    let len = length(v);
    if len < 0.001 {
        return vec3<f32>(0.0, 1.0, 0.0);
    }
    return v / len;
}
"#;

/// WGSL code for gradient noise functions.
pub const NOISE_WGSL: &str = r#"
// Gradient noise helpers
fn mod289_3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_4(x: vec4<f32>) -> vec4<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute4(x: vec4<f32>) -> vec4<f32> {
    return mod289_4(((x * 34.0) + 1.0) * x);
}

fn taylor_inv_sqrt4(r: vec4<f32>) -> vec4<f32> {
    return 1.79284291400159 - 0.85373472095314 * r;
}

// 3D Simplex noise
fn noise3(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0/6.0, 1.0/3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

    // First corner
    var i = floor(v + dot(v, vec3(C.y)));
    let x0 = v - i + dot(i, vec3(C.x));

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    let x1 = x0 - i1 + C.x;
    let x2 = x0 - i2 + C.y;
    let x3 = x0 - D.yyy;

    // Permutations
    i = mod289_3(i);
    let p = permute4(permute4(permute4(
        i.z + vec4<f32>(0.0, i1.z, i2.z, 1.0))
      + i.y + vec4<f32>(0.0, i1.y, i2.y, 1.0))
      + i.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));

    // Gradients
    let n_ = 0.142857142857;
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z * ns.z);

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    var p0 = vec3<f32>(a0.xy, h.x);
    var p1 = vec3<f32>(a0.zw, h.y);
    var p2 = vec3<f32>(a1.xy, h.z);
    var p3 = vec3<f32>(a1.zw, h.w);

    // Normalize gradients
    let norm = taylor_inv_sqrt4(vec4<f32>(dot(p0,p0), dot(p1,p1), dot(p2,p2), dot(p3,p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;

    // Mix final noise value
    var m = max(0.6 - vec4<f32>(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), vec4<f32>(0.0));
    m = m * m;
    return 42.0 * dot(m*m, vec4<f32>(dot(p0,x0), dot(p1,x1), dot(p2,x2), dot(p3,x3)));
}

// 2D Simplex noise (wrapper using z=0)
fn noise2(p: vec2<f32>) -> f32 {
    return noise3(vec3<f32>(p, 0.0));
}

// Fractal Brownian Motion - 3D
fn fbm3(p: vec3<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise3(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Fractal Brownian Motion - 2D
fn fbm2(p: vec2<f32>, octaves: i32) -> f32 {
    return fbm3(vec3<f32>(p, 0.0), octaves);
}
"#;

/// WGSL code for color conversion functions.
pub const COLOR_WGSL: &str = r#"
// HSV to RGB conversion
// h: hue [0, 1], s: saturation [0, 1], v: value [0, 1]
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;
    if hp < 1.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if hp < 2.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if hp < 3.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if hp < 4.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if hp < 5.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m, m, m);
}

// RGB to HSV conversion
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let cmax = max(max(rgb.r, rgb.g), rgb.b);
    let cmin = min(min(rgb.r, rgb.g), rgb.b);
    let delta = cmax - cmin;

    var h = 0.0;
    if delta > 0.0001 {
        if cmax == rgb.r {
            h = ((rgb.g - rgb.b) / delta) % 6.0;
        } else if cmax == rgb.g {
            h = (rgb.b - rgb.r) / delta + 2.0;
        } else {
            h = (rgb.r - rgb.g) / delta + 4.0;
        }
        h = h / 6.0;
        if h < 0.0 {
            h = h + 1.0;
        }
    }

    var s = 0.0;
    if cmax > 0.0001 {
        s = delta / cmax;
    }

    return vec3<f32>(h, s, cmax);
}
"#;

/// Get all built-in utility functions combined.
pub fn all_utils_wgsl() -> String {
    format!(
        "// Built-in utility functions\n{}\n{}\n{}\n",
        RANDOM_WGSL, NOISE_WGSL, COLOR_WGSL
    )
}
