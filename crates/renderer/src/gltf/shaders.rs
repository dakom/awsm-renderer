use awsm_renderer_core::shaders::ShaderModuleDescriptor;

// merely a key to hash ad-hoc shader generation
// is not stored on the mesh itself
//
// uniform and other runtime data for mesh
// is controlled via various components as-needed
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct ShaderKey {
    pub position_attribute_loc: Option<u32>,
    pub normal_attribute_loc: Option<u32>,
    pub tangent_attribute_loc: Option<u32>,
    pub morph_targets: Vec<MorphTarget>,
    pub skin_targets: Vec<SkinTarget>,
    pub n_morph_target_weights: u8,
    pub n_skin_joints: u8,
    pub tex_coords: Option<Vec<u32>>,
    pub vertex_colors: Option<Vec<VertexColor>>,
    pub normal_texture_uv_index: Option<u32>,
    pub metallic_roughness_texture_uv_index: Option<u32>,
    pub base_color_texture_uv_index: Option<u32>,
    pub emissive_texture_uv_index: Option<u32>,
    pub alpha_mode: ShaderKeyAlphaMode,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum MorphTarget {
    Position { loc: u32, weight_index: Option<u32> },
    Normal { loc: u32, weight_index: Option<u32> },
    Tangent { loc: u32, weight_index: Option<u32> },
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

// Construct source based on ShaderKey

impl ShaderKey {
    pub fn into_descriptor(&self) -> web_sys::GpuShaderModuleDescriptor {
        ShaderModuleDescriptor::new(&self.into_source(), None).into()
    }

    pub fn into_source(&self) -> String {
        static CAMERA: &str = include_str!("../shaders/camera.wgsl");
        static VERTEX_MESH: &str = include_str!("../shaders/vertex/mesh.wgsl");
        static FRAGMENT_PBR: &str = include_str!("../shaders/fragment/pbr.wgsl");

        let mut source = String::new();
        source.push_str(CAMERA);
        source.push_str(VERTEX_MESH);
        source.push_str(FRAGMENT_PBR);

        source
    }
}
