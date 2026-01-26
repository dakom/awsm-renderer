# Open-World Performance Plan (WebGPU)

This plan focuses on open‑world, high‑density scenes. The current renderer is already strong for constrained spaces (arenas, small levels), thanks to frustum + opaque occlusion culling and partial buffer uploads. The items below are incremental, ordered by impact.

## 1) Reduce Draw Calls (Opaque)
**Why:** Open worlds are draw‑call bound more than bandwidth bound.
**Plan:**
- **Spatial chunking:** Group static meshes by zone/sector at load time. Emit one draw list per chunk.
- **Static mesh merging:** Merge meshes that share material + pipeline state and are spatially close.
- **Material bucketing:** Prefer a small set of material variants per zone (reduce pipeline switches).

## 2) LOD + HLOD
**Why:** Geometry cost dominates at distance.
**Plan:**
- **Mesh LOD:** Swap to lower‑poly meshes at distance.
- **HLOD (impostors/mesh clusters):** Replace distant groups with baked proxy meshes or billboards.
- **Material LOD:** Reduce shader complexity with distance (disable expensive features).

## 3) Streaming + World Partitioning
**Why:** Avoid loading or updating what the camera can’t reach.
**Plan:**
- **Chunked asset streaming:** Load/unload by distance and camera direction.
- **Texture/mesh budget:** Keep a hard VRAM budget; evict least‑recently‑used chunks.
- **Asynchronous pipelines:** Avoid runtime hitches when loading new regions.

## 4) Finer Culling Granularity
**Why:** Current culling is mesh‑level. Large chunks can still overdraw.
**Plan:**
- **BVH/Octree per chunk:** Cull groups hierarchically before draw submission.
- **Clustered instancing:** For large instance sets (grass/rocks), split into chunks with per‑chunk AABBs.
- **(Later) GPU culling:** Compute‑driven instance compaction + indirect draw.

## 5) Lighting + Shadow Scalability
**Why:** Lighting cost scales with world size and dynamic lights.
**Plan:**
- **Clustered or tiled lighting:** Keep per‑pixel light counts bounded.
- **Shadow LOD:** Reduce shadow map resolution or update frequency with distance.
- **Cascaded shadow tuning:** Aggressive cascade ranges for performance‑critical views.

## 6) Animation and Skinning Budget
**Why:** Large crowds add CPU/GPU animation cost.
**Plan:**
- **Crowd LOD:** Reduce bones, update rate, or bake to vertex textures for distant crowds.
- **Skinning throttling:** Update skinned meshes at lower cadence for far objects.

## 7) Diagnostics + Metrics
**Why:** Open‑world performance needs constant visibility.
**Plan:**
- **Per‑frame stats:** draw calls, culled meshes, instance counts, GPU upload bytes.
- **Budget warnings:** log when frame exceeds geometry/texture/CPU budgets.
- **Profiler hooks:** emit markers around culling + render passes.

## Immediate Next Step (Low Risk)
Add **chunked instancing** + **static mesh merging** for opaque static geometry. This delivers the biggest win without requiring new rendering features.
