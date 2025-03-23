struct CameraUniforms {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection_inverse: mat4x4<f32>,
    position: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;