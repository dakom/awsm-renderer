use askama::Template;

use crate::{
    debug::{debug_once, debug_unique_string},
    render_passes::material::opaque::shader::{
        attributes::ShaderMaterialOpaqueVertexAttributes, cache_key::ShaderCacheKeyMaterialOpaque,
    },
    shaders::{print_shader_source, AwsmShaderError, Result},
};

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaque {
    /// Offset (in floats) within the packed vertex attribute array
    /// where the first UV component lives for each vertex.
    pub uv_sets_index: u32,
    pub texture_atlas_len: u32,
    pub sampler_atlas_len: u32,
    /// Offset (in floats) within the packed vertex attribute array
    /// where the first vertex color component lives for each vertex.
    pub color_sets_index: u32,
    pub normals: bool,
    pub tangents: bool,
    pub color_sets: Option<u32>,
    /// Number of UV sets available on the mesh.
    /// `None` means the mesh supplied no TEXCOORD attributes, which triggers the
    /// `pbr_material_has_any_uvs` branch inside `pbr_should_run`.
    pub uv_sets: Option<u32>,
    pub debug: ShaderTemplateMaterialOpaqueDebug,
    pub mipmap: MipmapMode,
    pub multisampled_geometry: bool,
    pub clamp_sampler_index: u32,
    pub msaa_sample_count: u32, // 0 if no MSAA
}

impl ShaderTemplateMaterialOpaque {
    pub fn has_lighting_ibl(&self) -> bool {
        match self.debug.lighting {
            ShaderTemplateMaterialOpaqueDebugLighting::None => true,
            ShaderTemplateMaterialOpaqueDebugLighting::IblOnly => true,
            ShaderTemplateMaterialOpaqueDebugLighting::PunctualOnly => false,
        }
    }

    pub fn has_lighting_punctual(&self) -> bool {
        match self.debug.lighting {
            ShaderTemplateMaterialOpaqueDebugLighting::None => true,
            ShaderTemplateMaterialOpaqueDebugLighting::IblOnly => false,
            ShaderTemplateMaterialOpaqueDebugLighting::PunctualOnly => true,
        }
    }
}

impl TryFrom<&ShaderCacheKeyMaterialOpaque> for ShaderTemplateMaterialOpaque {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialOpaque) -> Result<Self> {
        // Calculate the offset (in floats) to the first UV set within the packed custom attribute data.
        //
        // IMPORTANT: Normals and tangents are NO LONGER in attribute_data - they go in the visibility
        // buffer and geometry textures. Only custom attributes (colors, UVs, joints, weights) are in
        // attribute_data.
        //
        // The ordering follows `impl Ord for MeshBufferCustomVertexAttributeInfo`:
        // - Positions (0) - visibility attribute, NOT in attribute_data
        // - Normals (1) - visibility attribute, NOT in attribute_data
        // - Tangents (2) - visibility attribute, NOT in attribute_data
        // - Colors (3) - custom attribute, IN attribute_data
        // - TexCoords (4) - custom attribute, IN attribute_data
        // - Joints (5) - custom attribute, IN attribute_data
        // - Weights (6) - custom attribute, IN attribute_data
        //
        // However we only care about the `MeshBufferCustomVertexAttributeInfo` ordering here, so that's:
        //
        // - Colors (3) - custom attribute, IN attribute_data
        // - TexCoords (4) - custom attribute, IN attribute_data
        // - Joints (5) - custom attribute, IN attribute_data
        // - Weights (6) - custom attribute, IN attribute_data
        //

        // color sets always starts at 0;
        let mut color_sets_index = 0;

        // uv sets might start at 0 if there's no colors
        // otherwise, it's pushed off by however many color sets there are
        let mut uv_sets_index = 0;
        uv_sets_index += (value.attributes.color_sets.unwrap_or(0) * 4) as u32; // colors use 4 floats each

        let _self = Self {
            texture_atlas_len: value.texture_atlas_len,
            sampler_atlas_len: value.sampler_atlas_len,
            clamp_sampler_index: value.clamp_sampler_index,
            color_sets_index,
            uv_sets_index,
            normals: value.attributes.normals,
            tangents: value.attributes.tangents,
            color_sets: value.attributes.color_sets,
            uv_sets: value.attributes.uv_sets,
            mipmap: MipmapMode::Gradient,
            multisampled_geometry: value.msaa_sample_count > 0,
            msaa_sample_count: value.msaa_sample_count,
            debug: ShaderTemplateMaterialOpaqueDebug {
                mips: false,
                msaa_detect_edges: false, // Disabled - using real MSAA resolve
                ..Default::default()
            },
        };

        Ok(_self)
    }
}

#[derive(Debug)]
enum MipmapMode {
    None,
    Gradient,
}

#[derive(Debug, Default)]
struct ShaderTemplateMaterialOpaqueDebug {
    mips: bool,
    n_dot_v: bool,
    normals: bool,
    solid_color: bool,
    view_direction: bool,
    irradiance_sample: bool,
    msaa_detect_edges: bool,
    lighting: ShaderTemplateMaterialOpaqueDebugLighting,
}

impl ShaderTemplateMaterialOpaqueDebug {
    pub fn any(&self) -> bool {
        self.mips
            || self.n_dot_v
            || self.normals
            || self.solid_color
            || self.view_direction
            || self.irradiance_sample
            || self.msaa_detect_edges
            || !matches!(
                self.lighting,
                ShaderTemplateMaterialOpaqueDebugLighting::None
            )
    }
}

#[derive(Debug, Default)]
enum ShaderTemplateMaterialOpaqueDebugLighting {
    #[default]
    None,
    IblOnly,
    PunctualOnly,
}

impl ShaderTemplateMaterialOpaque {
    pub fn into_source(self) -> Result<String> {
        let source = self.render()?;

        //print_shader_source(&source, false);

        //debug_unique_string(1, &source, || print_shader_source(&source, false));

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}
