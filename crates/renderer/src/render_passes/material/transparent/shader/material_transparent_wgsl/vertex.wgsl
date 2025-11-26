/*************** START meta.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/meta.wgsl" %}
/*************** END meta.wgsl ******************/

/*************** START camera.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/camera.wgsl" %}
/*************** END camera.wgsl ******************/

/*************** START transform.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/transform.wgsl" %}
/*************** END transform.wgsl ******************/

/*************** START morph.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/morph.wgsl" %}
/*************** END morph.wgsl ******************/

/*************** START skin.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/skin.wgsl" %}
/*************** END skin.wgsl ******************/

/*************** START apply.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/apply.wgsl" %}
/*************** END apply.wgsl ******************/

/*************** START vertex_color.wgsl ******************/
{% include "opaque_and_transparency_wgsl/vertex_color.wgsl" %}
/*************** END vertex_color.wgsl ******************/

/*************** START textures.wgsl ******************/
{% include "opaque_and_transparency_wgsl/textures.wgsl" %}
/*************** END textures.wgsl ******************/

/*************** START material.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/material.wgsl" %}
/*************** END material.wgsl ******************/


@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> model_transforms : array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read> materials: array<PbrMaterialRaw>;
@group(0) @binding(3) var<uniform> mesh_meta: GeometryMeshMeta;
@group(0) @binding(4) var<storage, read> geometry_morph_weights: array<f32>;
@group(0) @binding(5) var<storage, read> geometry_morph_values: array<f32>;
@group(0) @binding(6) var<storage, read> skin_joint_matrices: array<mat4x4<f32>>;
// Joint buffer - exploded per vertex (matches morph pattern)
// We interleave indices with weights and get our index back losslessly via bitcast
// Layout: exploded vertex 0: [joints_0, joints_1, ...], exploded vertex 1: [joints_0, joints_1, ...], etc.
@group(0) @binding(7) var<storage, read> skin_joint_index_weights: array<f32>;
@group(0) @binding(8) var<storage, read> texture_transforms: array<TextureTransform>;



//***** MAIN *****
@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    return apply_vertex(input);
}
