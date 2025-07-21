struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;