use askama::Template;
use awsm_renderer_core::shaders::ShaderModuleDescriptor;

#[repr(u16)]
pub enum ShaderConstantIds {
    MaxMorphTargets = 1,
}
// merely a key to hash ad-hoc shader generation
// is not stored on the mesh itself
//
// uniform and other runtime data for mesh
// is controlled via various components as-needed
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
#[derive(Template)]
#[template(path = "main.wgsl")]
pub struct ShaderKey {
    // attributes
    pub has_position: bool,
    pub has_normal: bool,
    pub has_tangent: bool,
    // general feature
    pub has_morphs: bool,
    // pub skin_targets: Vec<SkinTarget>,
    // pub n_skin_joints: u8,
    // pub tex_coords: Option<Vec<u32>>,
    // pub vertex_colors: Option<Vec<VertexColor>>,
    // pub normal_texture_uv_index: Option<u32>,
    // pub metallic_roughness_texture_uv_index: Option<u32>,
    // pub base_color_texture_uv_index: Option<u32>,
    // pub emissive_texture_uv_index: Option<u32>,
    // pub alpha_mode: ShaderKeyAlphaMode,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SkinTarget {
    pub weight_loc: u32,
    pub joint_loc: u32,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct VertexColor {
    pub loc: u32,
    pub size: VertexColorSize,
}
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum VertexColorSize {
    Vec3,
    Vec4,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderKeyAlphaMode {
    Opaque,
    Blend,
    Mask,
}

impl Default for ShaderKeyAlphaMode {
    fn default() -> Self {
        Self::Opaque
    }
}

impl ShaderKey {
    pub fn into_descriptor(&self) -> web_sys::GpuShaderModuleDescriptor {
        ShaderModuleDescriptor::new(&self.into_source(), None).into()
    }

    pub fn into_source(&self) -> String {
        self.render().unwrap()
    }
}
