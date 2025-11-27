@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<storage, read> model_transforms : array<mat4x4<f32>>;
@group(2) @binding(0) var<uniform> mesh_meta: GeometryMeshMeta;
@group(3) @binding(0) var<storage, read> geometry_morph_weights: array<f32>;
@group(3) @binding(1) var<storage, read> geometry_morph_values: array<f32>;
@group(3) @binding(2) var<storage, read> skin_joint_matrices: array<mat4x4<f32>>;
// Joint buffer - exploded per vertex (matches morph pattern)
// We interleave indices with weights and get our index back losslessly via bitcast
// Layout: exploded vertex 0: [joints_0, joints_1, ...], exploded vertex 1: [joints_0, joints_1, ...], etc.
@group(3) @binding(3) var<storage, read> skin_joint_index_weights: array<f32>;
