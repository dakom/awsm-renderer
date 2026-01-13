# Shader Guidelines

This document captures best practices and gotchas learned from debugging WGSL shaders, particularly for visibility buffer rendering with MSAA.

## Loop Unrolling

### Don't manually unroll loops

WGSL compilers optimize loops with compile-time-known bounds effectively. Manual unrolling using template `{% for %}` blocks can cause issues:

**Bad (template unrolling):**
```wgsl
{% for i in 0..4 %}
    let sample_{{ i }} = textureLoad(tex, coords, {{ i }});
    // ... process sample_{{ i }}
{% endfor %}
```

This template-generates code with different variable names (`sample_0`, `sample_1`, etc.) which can cause unexpected behavior and is harder to debug.

**Good (runtime loop with compile-time bound):**
```wgsl
for (var i = 0u; i < 4u; i++) {
    let sample = load_sample(coords, i);  // Use helper function
    // ... process sample
}
```

### Template constants for loop bounds are fine

Using template values for loop bounds is acceptable because the compiler can still optimize:

```wgsl
for (var s = 0u; s < {{ msaa_sample_count }}u; s++) {
    // This is fine - the bound is known at compile time
}
```

## Variable Declarations in Loops

### Move code to helper functions

Don't declare variables inline in loops or use complex expressions. Instead, extract them to helper functions:

**Bad:**
```wgsl
for (var s = 0u; s < MSAA_SAMPLES; s++) {
    var vis_data: vec4<u32>;
    switch(s) {
        case 0u: { vis_data = textureLoad(tex, coords, 0); }
        // ...
    }
    let triangle_id = join32(vis_data.x, vis_data.y);
    // ... more processing
}
```

**Good:**
```wgsl
fn load_sample_triangle_id(coords: vec2<i32>, s: u32) -> u32 {
    var v: vec4<u32>;
    switch(s) {
        case 0u: { v = textureLoad(visibility_data_tex, coords, 0); }
        case 1u: { v = textureLoad(visibility_data_tex, coords, 1); }
        case 2u: { v = textureLoad(visibility_data_tex, coords, 2); }
        case 3u, default: { v = textureLoad(visibility_data_tex, coords, 3); }
    }
    return join32(v.x, v.y);
}

// Then in main code:
for (var s = 0u; s < MSAA_SAMPLES; s++) {
    let triangle_id = load_sample_triangle_id(coords, s);
    // ... clean, simple processing
}
```

## WGSL textureLoad() Sample Index Requirements

In WGSL, `textureLoad()` for multisampled textures requires the sample index to be a compile-time constant literal. You cannot use a runtime variable:

**Won't compile:**
```wgsl
let sample = textureLoad(msaa_texture, coords, sample_index);  // Error!
```

**Solution - use switch statement:**
```wgsl
fn load_msaa_sample(coords: vec2<i32>, s: u32) -> vec4<f32> {
    var result: vec4<f32>;
    switch(s) {
        case 0u: { result = textureLoad(msaa_texture, coords, 0); }
        case 1u: { result = textureLoad(msaa_texture, coords, 1); }
        case 2u: { result = textureLoad(msaa_texture, coords, 2); }
        case 3u, default: { result = textureLoad(msaa_texture, coords, 3); }
    }
    return result;
}
```

## MSAA Processing Patterns

### Shared vs Per-Sample Data

For MSAA resolve, distinguish between:
- **Shared data**: Computed once and reused for all samples (e.g., `standard_coordinates`, `lights_info`)
- **Per-sample data**: Must be loaded/computed for each sample (e.g., visibility data, barycentric coordinates)

```wgsl
// Compute shared data once
let standard_coordinates = get_standard_coordinates(coords, screen_dims);
let lights_info = get_lights_info();

// Process each sample
for (var s = 0u; s < MSAA_SAMPLES; s++) {
    let sample_result = process_sample(
        standard_coordinates,  // Shared
        lights_info,          // Shared
        load_sample_textures(coords, s)  // Per-sample
    );
    // accumulate results...
}
```

### Encapsulate Sample Processing

Create a helper function that processes one sample and returns a result struct:

```wgsl
struct SampleResult {
    color: vec3<f32>,
    alpha: f32,
    is_valid: bool,
}

fn process_sample(
    shared_data: SharedData,
    sample_textures: SampleTextures,
) -> SampleResult {
    // All sample processing in one place
}

fn msaa_resolve_samples(/* ... */) -> ResolveResult {
    var color_sum = vec3<f32>(0.0);
    var valid_count = 0u;

    for (var s = 0u; s < MSAA_SAMPLES; s++) {
        let result = process_sample(shared, load_sample_textures(coords, s));
        if (result.is_valid) {
            color_sum += result.color;
            valid_count++;
        }
    }

    return ResolveResult(color_sum, valid_count);
}
```

## General Best Practices

1. **Keep functions small and focused** - easier to debug and maintain
2. **Use descriptive struct names** - `MsaaSampleTextures` over `TexData`
3. **Early exit for invalid cases** - check for `U32_MAX`, zero values, etc.
4. **Match main code path logic** - MSAA sample processing should mirror the main non-MSAA path
5. **Avoid magic numbers** - use named constants
6. **Comment non-obvious optimizations** - especially for lazy-loading patterns
