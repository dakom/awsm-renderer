# MSAA Optimization: Resolve Targets + Discard

## TL;DR - The Optimization

Using resolve targets with `StoreOp::Discard` eliminates **~480 MB of memory bandwidth per frame** at 1080p:
- ❌ **240 MB writes eliminated**: Multisampled G-buffer data never written to VRAM
- ❌ **240 MB reads eliminated**: Compute shader reads single-sample resolved data instead

**How it works:**
1. GPU rasterizes to multisampled render targets (keeps data in tile memory/on-chip cache)
2. GPU performs hardware MSAA resolve to single-sample textures (highly optimized)
3. **GPU discards multisampled data without writing to VRAM** (the key optimization!)
4. Only single-sample resolved data goes to VRAM

This is standard practice for deferred rendering with MSAA. The multisampled data is purely an intermediate representation for antialiasing - we don't need to store it.

---

## Current Implementation

### Memory Layout
When MSAA is enabled (4 samples), all geometry pass render targets are multisampled:
- `visibility_data` (Rgba16uint) - 4x samples stored
- `barycentric` (Rg16float) - 4x samples stored
- `normal_tangent` (Rgba16float) - 4x samples stored
- `barycentric_derivatives` (Rgba16float) - 4x samples stored
- `depth` (Depth24plus) - 4x samples stored

For a 1920x1080 render target, this means:
- Each color attachment: ~16-33 MB × 4 samples = **64-132 MB per texture**
- Total G-buffer: ~**320 MB** for multisampled storage

### Current Pipeline
1. **Geometry Pass** (render pass):
   - Rasterizes geometry to multisampled render targets
   - Uses `StoreOp::Store` for all attachments
   - All 4 MSAA samples written to memory

2. **Material Opaque Pass** (compute):
   - Binds multisampled textures: `texture_multisampled_2d<T>`
   - Reads all 4 samples per pixel using `textureLoad(..., sample_index)`
   - Performs custom MSAA resolve with edge detection:
     - Detects edges via normal discontinuity and depth variation
     - Interior pixels: uses sample 0 only
     - Edge pixels: averages all 4 samples with full material evaluation per sample
   - Writes to single-sample `opaque_color` storage texture

### Costs
- **Memory**: 4x storage for all G-buffer textures (~320 MB at 1080p)
- **Bandwidth**: 4x reads from memory in compute shader
- **Compute**: Custom edge detection + conditional per-sample processing

### Benefits
- Sophisticated edge detection logic
- Per-sample material evaluation only where needed
- Can use barycentric derivatives for proper texture gradients per sample

## Proposed Optimization

### The Critical Optimization: StoreOp::Discard

**This is the key performance win.**

When you use a resolve target WITH `StoreOp::Discard`, you're telling the GPU:
1. ✅ Rasterize to the multisampled render target (in tile memory / on-chip cache)
2. ✅ Resolve the MSAA samples to the single-sample target (hardware operation)
3. ❌ **DO NOT write the multisampled data back to VRAM**

Without discard, the GPU would:
1. ✅ Rasterize to multisampled render target
2. ✅ Resolve to single-sample target
3. ❌ **ALSO write all 4x multisampled data back to VRAM** (wasted bandwidth!)

**The discard operation eliminates a massive memory write**: at 1080p, we avoid writing ~240 MB of multisampled G-buffer data every frame. This is pure wasted bandwidth since we only need the resolved single-sample data.

Modern tile-based GPUs (including most mobile and Apple Silicon GPUs) can keep the multisampled data entirely on-chip, perform the resolve, and discard without ever touching VRAM. This makes `StoreOp::Discard` + resolve targets one of the most important optimizations for deferred rendering with MSAA.

### Memory Layout Changes
When MSAA is enabled:
- Color attachments created as **multisampled** for rasterization
- **Resolve targets** created as single-sample textures
- Multisampled color data marked `StoreOp::Discard` (**CRITICAL: never written to VRAM**)
- Only **depth remains multisampled and stored**

After geometry pass, only these are in memory:
- `visibility_data` - single sample resolved
- `barycentric` - single sample resolved
- `normal_tangent` - single sample resolved
- `barycentric_derivatives` - single sample resolved
- `depth` - 4x samples stored

Memory savings: ~**240 MB at 1080p** (75% reduction in G-buffer storage)

**Bandwidth savings per frame:**
- **WRITE eliminated**: ~240 MB multisampled G-buffer data (never written to VRAM)
- **READ eliminated**: ~240 MB in compute shader (reads single-sample instead)
- **Total per-frame savings**: ~480 MB of memory bandwidth

### Implementation Details

#### Texture Creation (`render_textures.rs:234-250`)
Create two sets of textures when MSAA is enabled:
```rust
// Multisampled render targets (for rasterization only, will be discarded)
let visibility_data_msaa = gpu.create_texture(
    &geometry_texture(formats.visibility_data, "Visibility Data MSAA")
        .with_sample_count(4)
        .into()
)?;

// Single-sample resolve targets (these get stored)
let visibility_data = gpu.create_texture(
    &geometry_texture(formats.visibility_data, "Visibility Data")
        .into()  // No sample_count, defaults to 1
)?;

// Repeat for: barycentric, normal_tangent, barycentric_derivatives

// Depth remains multisampled only (no resolve target)
let depth = gpu.create_texture(
    &geometry_texture(formats.depth, "Depth")
        .with_sample_count(4)
        .into()
)?;
```

#### Geometry Pass Changes (`geometry/render_pass.rs:51-88`)
```rust
let mut color_attachments = vec![
    ColorAttachment::new(
        &ctx.render_texture_views.visibility_data_msaa,  // Multisampled view
        LoadOp::Clear,
        StoreOp::Discard,  // CRITICAL OPTIMIZATION: Never write multisampled data to VRAM!
                           // GPU keeps this in tile memory, resolves it, then discards.
                           // Saves ~240 MB write bandwidth per frame at 1080p.
    )
    .with_clear_color(VISIBILITY_CLEAR_COLOR.clone())
    .with_resolve_target(&ctx.render_texture_views.visibility_data), // Hardware resolves to single-sample

    ColorAttachment::new(
        &ctx.render_texture_views.barycentric_msaa,
        LoadOp::Clear,
        StoreOp::Discard,  // Discard multisampled intermediate
    )
    .with_resolve_target(&ctx.render_texture_views.barycentric),

    // ... repeat for normal_tangent, barycentric_derivatives
];

// Depth remains unchanged - multisampled and stored
depth_stencil_attachment: Some(
    DepthStencilAttachment::new(&ctx.render_texture_views.depth)
        .with_depth_load_op(LoadOp::Clear)
        .with_depth_store_op(StoreOp::Store)  // Still store depth for later passes
        .with_depth_clear_value(1.0),
),
```

#### Material Pass Changes (`material/opaque/`)

**Bind Group** (`bind_group.rs:492-549`):
- Remove `multisampled_geometry` variants
- All geometry texture bindings become `texture_2d<T>` instead of `texture_multisampled_2d<T>`
- Depth can remain `texture_depth_multisampled_2d` if needed for future depth-aware effects

**Pipeline** (`pipeline.rs`):
- Remove `multisampled_pipeline_layout_key`
- Single pipeline layout for all cases
- Simpler cache key without MSAA variants

**Shader** (`material_opaque_wgsl/compute.wgsl`):
- Remove all `{% if multisampled_geometry %}` conditionals
- Remove `msaa.wgsl` include and edge detection logic
- Change texture types from multisampled to single-sample:
  ```wgsl
  @group(0) @binding(0) var visibility_data_tex: texture_2d<u32>;
  @group(0) @binding(1) var barycentric_tex: texture_2d<f32>;
  @group(0) @binding(3) var normal_tangent_tex: texture_2d<f32>;
  @group(0) @binding(4) var barycentric_derivatives_tex: texture_2d<f32>;
  // Note: depth could remain texture_depth_multisampled_2d if needed
  ```
- Simplify texture loads from `textureLoad(tex, coords, sample_index)` to `textureLoad(tex, coords, 0)`
- Remove MSAA resolve logic (loop over samples, averaging, etc.)

### New Pipeline
1. **Geometry Pass** (render pass):
   - Rasterizes to multisampled render targets (proper edge antialiasing)
   - Hardware resolves color attachments to single-sample textures
   - Discards multisampled intermediates (saves memory)
   - Stores multisampled depth buffer

2. **Material Opaque Pass** (compute):
   - Binds single-sample resolved textures: `texture_2d<T>`
   - One texture load per pixel: `textureLoad(tex, coords, 0)`
   - No MSAA edge detection or multi-sample loops
   - Direct material evaluation per pixel
   - Writes to `opaque_color` storage texture

### Benefits
1. **Bandwidth** (The Big Win):
   - **StoreOp::Discard eliminates ~240 MB writes per frame** (multisampled G-buffer never written to VRAM)
   - 75% reduction in compute shader reads (~240 MB reads saved)
   - **Total: ~480 MB memory bandwidth saved per frame at 1080p**
   - On tile-based GPUs (mobile, Apple Silicon), multisampled data stays on-chip entirely

2. **Memory**: 75% reduction in G-buffer storage (240 MB saved at 1080p)

3. **Performance**:
   - Hardware MSAA resolve is highly optimized (GPU fixed-function)
   - Simpler compute shader (no branching, no sample loops)
   - Better cache coherency (single-sample loads)

4. **Code Simplicity**:
   - Remove ~200 lines of MSAA edge detection logic
   - Single pipeline variant instead of multisampled/singlesampled pairs
   - Easier to maintain and debug

### Tradeoffs
1. **Quality**: Lose custom edge detection and per-sample shading
   - Hardware resolve uses fixed pattern (typically box filter averaging)
   - Current implementation does intelligent per-pixel MSAA resolve
   - In practice, hardware MSAA resolve quality is excellent for most cases

2. **Flexibility**: Can't do per-sample material evaluation
   - Current code evaluates materials per-sample on edges
   - New approach evaluates materials once per pixel on resolved data
   - For PBR materials, the difference is usually imperceptible

3. **Depth Buffer**: Stays multisampled
   - Still 4x storage cost for depth (8 MB at 1080p)
   - Could be used for future depth-aware effects
   - Could also be resolved if depth isn't needed multisampled

## Performance Expectations

### Memory Savings (1920x1080)
- Before: ~320 MB G-buffer
- After: ~80 MB G-buffer + 8 MB depth = **88 MB total**
- **Savings: 232 MB (72%)**

### Bandwidth Savings
- Compute shader reads: 4x fewer texture fetches per pixel
- At 1080p (2M pixels): 8M texture loads → 2M texture loads per frame
- Significant memory bandwidth reduction

### Compute Savings
- Remove edge detection logic (normal comparison, depth checks)
- Remove MSAA sample loops (up to 4x material evaluations per edge pixel)
- Simpler, more cache-friendly memory access pattern

## Quality Comparison

Hardware MSAA resolve is well-tested and used by most modern renderers:
- Unreal Engine uses hardware resolve
- Unity uses hardware resolve
- Most AAA games use hardware resolve

The custom edge detection approach is clever but likely over-engineered. The quality difference would be minimal in practice, while the performance cost is significant.

## Implementation Strategy - Two-Phase Approach

### 🚨 RISK ASSESSMENT: Why Split The Work?

The **compute shader simplification** is the highest risk part of this refactor:
- ~200 lines of template-generated WGSL with conditionals and unrolled loops
- Complex MSAA edge detection logic (`msaa.wgsl`)
- Multiple texture type changes across many binding points
- Easy to miss a conditional or leave inconsistent state

**By having the human do the shader work first**, we reduce risk significantly:
1. Human understands the template system and shader logic better
2. Claude handles the straightforward Rust plumbing (texture creation, bind groups, pipelines)
3. Clear separation of concerns and validation points

---

## PHASE 1: Human Does Shader Simplification (YOU DO THIS FIRST)

### Prerequisites
Before starting, ensure you have:
- [ ] Visual testing capability (can run renderer and see results)
- [ ] Test scene with hard geometric edges (cubes, sharp angles - to verify MSAA quality)
- [ ] Ability to take before/after screenshots
- [ ] WebGPU validation layers enabled (will catch binding/format errors)
- [ ] Working on a safe branch (`texture-pool` seems fine, but confirm)

### Step 1.1: Take "Before" Screenshots
```bash
# Run your renderer with MSAA enabled
# Take screenshots of test scene focusing on:
# - Hard geometric edges (cube corners, etc.)
# - High contrast boundaries
# - Texture detail on angled surfaces
# Save these as: before_msaa_*.png
```

### Step 1.2: Simplify Compute Shader (`material_opaque_wgsl/compute.wgsl`)

**Current state:** Shader has multisampled texture bindings and conditional MSAA logic

**Target state:** Shader only uses single-sample textures, all MSAA logic removed

**Changes to make:**

1. **Remove the MSAA helper include** (around line 52-55):
   ```wgsl
   // DELETE THIS ENTIRE BLOCK:
   {% if multisampled_geometry %}
   /*************** START msaa.wgsl ******************/
   {% include "material_opaque_wgsl/helpers/msaa.wgsl" %}
   /*************** END msaa.wgsl ******************/
   ```

2. **Change texture bindings from multisampled to single-sample** (around line 77-82):
   ```wgsl
   // BEFORE (multisampled - DELETE THIS):
   {% if multisampled_geometry %}
       @group(0) @binding(0) var visibility_data_tex: texture_multisampled_2d<u32>;
       @group(0) @binding(1) var barycentric_tex: texture_multisampled_2d<f32>;
       @group(0) @binding(2) var depth_tex: texture_depth_multisampled_2d;
       @group(0) @binding(3) var normal_tangent_tex: texture_multisampled_2d<f32>;
       @group(0) @binding(4) var barycentric_derivatives_tex: texture_multisampled_2d<f32>;

   // AFTER (single-sample - CHANGE TO THIS):
   @group(0) @binding(0) var visibility_data_tex: texture_2d<u32>;
   @group(0) @binding(1) var barycentric_tex: texture_2d<f32>;
   @group(0) @binding(2) var depth_tex: texture_depth_2d;  // or remove if not needed
   @group(0) @binding(3) var normal_tangent_tex: texture_2d<f32>;
   @group(0) @binding(4) var barycentric_derivatives_tex: texture_2d<f32>;
   ```

3. **Remove MSAA sample checking in early return** (around line 139-172):
   ```wgsl
   // DELETE THIS ENTIRE CONDITIONAL BLOCK:
   {% if multisampled_geometry %}
       // With MSAA, check if ANY sample hit geometry before early returning
       var any_sample_hit = false;
       {% for s in 0..msaa_sample_count %}
           // ... sample checking logic ...
       {% endfor %}
       // ... more MSAA logic ...

   // KEEP ONLY THE SIMPLE SINGLE-SAMPLE VERSION:
   let visibility_data = textureLoad(visibility_data_tex, coords, 0);
   if (visibility_data.x == 0xFFFFFFFF) {
       return;
   }
   ```

4. **Remove MSAA material evaluation loop** (around line 172-540):
   Look for blocks like:
   ```wgsl
   {% if multisampled_geometry %}
       // ... loop over samples ...
       {% for s in 0..msaa_sample_count %}
           // ... per-sample material evaluation ...
       {% endfor %}
   ```

   **DELETE** all these conditional MSAA blocks, **KEEP** only the single-sample code path.

5. **Remove MSAA resolve logic at the end** (around line 542-672):
   ```wgsl
   // DELETE THIS ENTIRE SECTION:
   // MSAA Resolve: if this is an edge pixel, sample all MSAA samples and blend
   {% if multisampled_geometry && !debug.msaa_detect_edges %}
       let samples_to_process = msaa_sample_count_for_pixel(...);
       if samples_to_process > 1 {
           // Edge pixel - resolve MSAA by averaging all samples
           {% for s in 0..msaa_sample_count %}
               // ... complex per-sample evaluation ...
           {% endfor %}
       }
   ```

   **KEEP** only the simple single-sample write:
   ```wgsl
   // Write to output texture (single-sample path)
   textureStore(opaque_color_tex, coords, final_color);
   ```

6. **Remove debug MSAA edge detection** (around line 672):
   ```wgsl
   // DELETE:
   {% if multisampled_geometry && debug.msaa_detect_edges %}
       // ... edge detection debug visualization ...
   ```

**Result after Step 1.2:**
- `compute.wgsl` should have NO `{% if multisampled_geometry %}` conditionals
- All texture loads should be: `textureLoad(tex, coords, 0)` (single sample)
- No loops over samples
- ~200 lines removed

### Step 1.3: Update Shader Cache Key (`material_opaque_wgsl/cache_key.rs`)

**File:** `crates/renderer/src/render_passes/material/opaque/shader/cache_key.rs`

**Around line 18**, remove or set to 0:
```rust
// BEFORE:
pub msaa_sample_count: u32, // 0 if no MSAA

// AFTER (either remove the field entirely, or always set to 0):
// Option A: Remove the field
// (delete the line)

// Option B: Keep it but it's always 0 now
pub msaa_sample_count: u32, // Always 0 now - using hardware resolve
```

If you remove the field, you'll need to update everywhere it's constructed (likely in `pipeline.rs`).

**Decision point:** I recommend **Option A (remove entirely)** for cleaner code.

### Step 1.4: Update Geometry Shader Cache Key (`geometry/shader/cache_key.rs`)

**File:** `crates/renderer/src/render_passes/geometry/shader/cache_key.rs`

**Around line 9:**
```rust
// BEFORE:
pub msaa_samples: u32,

// AFTER (same decision as material cache key):
// Option A: Remove the field (recommended)
// Option B: Always set to 0
```

Again, recommend **remove entirely**.

### Step 1.5: Verify Compilation

```bash
cd crates/renderer
cargo check
```

**Expected result:**
- Compilation errors in `pipeline.rs` files where cache keys are constructed
- That's OK! Claude will fix these in Phase 2
- Main goal: ensure shader template is valid

### Step 1.6: Document Your Changes

Create a file: `PHASE1_COMPLETE.md` with:
```markdown
# Phase 1 Complete - Shader Simplification

## What I Did:
- [ ] Removed MSAA conditionals from compute.wgsl (~200 lines)
- [ ] Changed all texture types to single-sample
- [ ] Removed msaa.wgsl include
- [ ] Removed msaa_sample_count from shader cache keys
- [ ] Took "before" screenshots (attached/linked)

## Current State:
- Shader is simplified but Rust code won't compile yet
- Compilation errors in: [list files with errors]
- Branch: [branch name]
- Last commit: [commit hash]

## Ready for Phase 2:
YES / NO

## Notes:
[Any concerns, questions, or observations]
```

---

## PHASE 2: Claude Does Rust Plumbing (CLAUDE DOES THIS AFTER PHASE 1)

**When you're done with Phase 1**, tell Claude:
> "I've completed Phase 1 shader simplification. Please read MULTISAMPLE_OPTIMIZATION.md and PHASE1_COMPLETE.md, then execute Phase 2."

### Step 2.1: Fix Shader Cache Key Construction

**Files to update:**
- `crates/renderer/src/render_passes/material/opaque/pipeline.rs` (~line 89)
- `crates/renderer/src/render_passes/geometry/pipeline.rs` (~line 109, 121, 131)

**Remove** `msaa_sample_count` or `msaa_samples` field from cache key construction.

### Step 2.2: Update Texture Creation (`render_textures.rs`)

**Goal:** Create both MSAA and single-sample textures when MSAA is enabled

**Changes in `RenderTexturesInner::new()` method:**

1. Add new fields to `RenderTexturesInner` struct:
   ```rust
   // Multisampled render targets (only when MSAA enabled, will be discarded)
   pub visibility_data_msaa: Option<web_sys::GpuTexture>,
   pub visibility_data_msaa_view: Option<web_sys::GpuTextureView>,
   pub barycentric_msaa: Option<web_sys::GpuTexture>,
   pub barycentric_msaa_view: Option<web_sys::GpuTextureView>,
   // ... etc for other G-buffer textures
   ```

2. Update texture creation logic:
   ```rust
   let has_msaa = anti_aliasing.msaa_sample_count.is_some();

   if has_msaa {
       // Create multisampled render targets
       let visibility_data_msaa = gpu.create_texture(
           &geometry_texture(formats.visibility_data, "Visibility Data MSAA")
               .with_sample_count(4)
               .into()
       )?;

       // Create single-sample resolve targets
       let visibility_data = gpu.create_texture(
           &geometry_texture(formats.visibility_data, "Visibility Data")
               .into()  // No sample_count
       )?;
   } else {
       // Create only single-sample (no MSAA)
       let visibility_data = gpu.create_texture(
           &geometry_texture(formats.visibility_data, "Visibility Data")
               .into()
       )?;
   }
   ```

3. Update `RenderTextureViews` to expose MSAA views when needed:
   ```rust
   pub struct RenderTextureViews {
       // Resolved single-sample views (always present)
       pub visibility_data: web_sys::GpuTextureView,
       pub barycentric: web_sys::GpuTextureView,
       // ...

       // Multisampled views (only when MSAA enabled)
       pub visibility_data_msaa: Option<web_sys::GpuTextureView>,
       pub barycentric_msaa: Option<web_sys::GpuTextureView>,
       // ...
   }
   ```

### Step 2.3: Update Geometry Pass (`geometry/render_pass.rs`)

**Goal:** Add resolve targets and use `StoreOp::Discard`

**Changes in `render()` method:**

```rust
let mut color_attachments = if let (Some(vis_msaa), Some(bary_msaa), ...) = (
    &ctx.render_texture_views.visibility_data_msaa,
    &ctx.render_texture_views.barycentric_msaa,
    // ...
) {
    // MSAA path: use multisampled attachments with resolve targets
    vec![
        ColorAttachment::new(vis_msaa, LoadOp::Clear, StoreOp::Discard)
            .with_clear_color(VISIBILITY_CLEAR_COLOR.clone())
            .with_resolve_target(&ctx.render_texture_views.visibility_data),
        ColorAttachment::new(bary_msaa, LoadOp::Clear, StoreOp::Discard)
            .with_resolve_target(&ctx.render_texture_views.barycentric),
        // ... etc
    ]
} else {
    // Non-MSAA path: render directly to single-sample
    vec![
        ColorAttachment::new(&ctx.render_texture_views.visibility_data, LoadOp::Clear, StoreOp::Store)
            .with_clear_color(VISIBILITY_CLEAR_COLOR.clone()),
        ColorAttachment::new(&ctx.render_texture_views.barycentric, LoadOp::Clear, StoreOp::Store),
        // ... etc
    ]
};
```

### Step 2.4: Simplify Material Pass Bind Groups (`material/opaque/bind_group.rs`)

**Goal:** Remove multisampled texture binding variants

**Changes:**

1. Remove `multisampled_main_bind_group_layout_key` field from struct
2. Remove the separate multisampled bind group layout creation
3. Update `create_main_bind_group_layout()` function:
   - Remove `multisampled_geometry` parameter
   - All texture bindings become `texture_2d<T>` (never multisampled)
4. Update `get_bind_groups()` method:
   - Remove the conditional check for `ctx.anti_aliasing.msaa_sample_count.is_some()`
   - Always use single-sample texture bindings

### Step 2.5: Simplify Material Pass Pipeline (`material/opaque/pipeline.rs`)

**Goal:** Single pipeline layout instead of multisampled/singlesampled variants

**Changes:**

1. Remove `multisampled_pipeline_layout_key` field from struct
2. Remove multisampled pipeline layout creation
3. Update `get_pipeline_key()` method:
   - Remove conditional check for `msaa_sample_count`
   - Always use single pipeline layout

### Step 2.6: Update Pipeline Creation (`geometry/pipeline.rs`)

**Goal:** Simplify MSAA pipeline handling if needed

**Changes:**
- Review how `msaa_4_pipeline_keys` is used
- Depth attachment still uses MSAA, so pipelines still need `sample_count(4)`
- Color attachments now resolve, but pipeline multisample state stays the same
- Likely **no changes needed here** - the multisample state is for rasterization, which we still want

### Step 2.7: Compile and Test

```bash
cargo build
# Fix any remaining compilation errors
```

**Then run renderer and verify:**
- [ ] No crashes
- [ ] MSAA edges still look good
- [ ] Take "after" screenshots
- [ ] Compare with "before" screenshots from Phase 1
- [ ] Profile memory usage (should see ~240 MB reduction at 1080p)

---

## Validation Checklist

### After Phase 1 (Human):
- [ ] Shader compiles (template renders without errors)
- [ ] All `multisampled_geometry` conditionals removed
- [ ] All texture types are single-sample
- [ ] Rust code has expected compilation errors in pipeline.rs files
- [ ] "Before" screenshots captured

### After Phase 2 (Claude):
- [ ] All Rust code compiles without errors
- [ ] Renderer runs without crashes
- [ ] MSAA quality visually matches "before" screenshots
- [ ] Memory usage reduced by ~240 MB (verify with profiler)
- [ ] No WebGPU validation errors
- [ ] Performance improved (frame time reduced)

---

## Troubleshooting

### If shader template has errors:
- Check that you removed ALL `{% if multisampled_geometry %}` blocks
- Verify texture load calls are: `textureLoad(tex, coords, 0)` not `textureLoad(tex, coords, sample_idx)`
- Make sure you kept the "else" branch of conditionals (the single-sample path)

### If Rust won't compile after Phase 1:
- That's expected! Pipeline files reference removed cache key fields
- Claude will fix in Phase 2

### If visual quality regressed:
- Verify `StoreOp::Discard` is set (Phase 2, Step 2.3)
- Verify resolve targets are attached (Phase 2, Step 2.3)
- Check depth attachment still uses MSAA (`sample_count: 4`)
- Compare screenshots side-by-side at pixel level

### If memory usage didn't improve:
- Verify multisampled textures are being discarded (not stored)
- Check GPU memory profiler to see texture allocations
- Ensure resolve targets are single-sample (not accidentally multisampled)

---

## Summary

**PHASE 1 (You):** Simplify shaders - remove ~200 lines of MSAA logic
**PHASE 2 (Claude):** Wire up resolve targets, fix Rust plumbing, test

**Why this split?** The shader template logic is complex and error-prone for AI. You understand the codebase better. Claude is better at the mechanical Rust changes (texture creation, bind groups, pipelines).

**When ready for Phase 2:** Create `PHASE1_COMPLETE.md` and tell Claude to continue.

## Future Considerations

If the depth buffer doesn't need to be multisampled for future passes, it could also use a resolve target, saving the final 8 MB and allowing fully single-sample G-buffer processing.

Alternatively, if depth-based effects require multisampled depth, the current approach preserves that capability while still saving 72% of G-buffer memory.
