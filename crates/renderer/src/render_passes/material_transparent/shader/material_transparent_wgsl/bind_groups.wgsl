@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> model_transforms : array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read> materials: array<PbrMaterialRaw>;
@group(0) @binding(3) var<storage, read> geometry_morph_weights: array<f32>;
@group(0) @binding(4) var<storage, read> geometry_morph_values: array<f32>;
@group(0) @binding(5) var<storage, read> skin_joint_matrices: array<mat4x4<f32>>;
// Joint buffer - exploded per vertex (matches morph pattern)
// We interleave indices with weights and get our index back losslessly via bitcast
// Layout: exploded vertex 0: [joints_0, joints_1, ...], exploded vertex 1: [joints_0, joints_1, ...], etc.
@group(0) @binding(6) var<storage, read> skin_joint_index_weights: array<f32>;
@group(0) @binding(7) var<storage, read> texture_transforms: array<TextureTransform>;

@group(1) @binding(0) var<uniform> lights_info: LightsInfoPacked;
@group(1) @binding(1) var<storage, read> lights: array<LightPacked>;

{% for i in 0..texture_pool_arrays_len %}
    @group(2) @binding({{ i }}u) var pool_tex_{{ i }}: texture_2d_array<f32>;
{% endfor %}
{% for i in 0..texture_pool_samplers_len %}
    @group(2) @binding({{ texture_pool_arrays_len + i }}u) var pool_sampler_{{ i }}: sampler;
{% endfor %}

@group(3) @binding(0) var<uniform> geometry_mesh_meta: GeometryMeshMeta;
