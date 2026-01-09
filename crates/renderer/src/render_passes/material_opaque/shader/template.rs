use askama::Template;

use crate::{
    render_passes::material_opaque::shader::cache_key::{
        ShaderCacheKeyMaterialOpaque, ShaderCacheKeyMaterialOpaqueEmpty,
    },
    shaders::{AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateMaterialOpaque {
    pub bind_groups: ShaderTemplateMaterialOpaqueBindGroups,
    pub compute: ShaderTemplateMaterialOpaqueCompute,
}

#[derive(Template, Debug)]
#[template(
    path = "material_opaque_wgsl/bind_groups.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateMaterialOpaqueBindGroups {
    /// Offset (in floats) within the packed vertex attribute array
    /// where the first UV component lives for each vertex.
    pub uv_sets_index: u32,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
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
    pub msaa_sample_count: u32, // 0 if no MSAA
    pub unlit: bool,
}

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaqueCompute {
    /// Offset (in floats) within the packed vertex attribute array
    /// where the first UV component lives for each vertex.
    pub uv_sets_index: u32,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
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
    pub msaa_sample_count: u32, // 0 if no MSAA
    pub unlit: bool,
}

impl ShaderTemplateMaterialOpaqueCompute {
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
        let color_sets_index = 0;

        // uv sets might start at 0 if there's no colors
        // otherwise, it's pushed off by however many color sets there are
        let mut uv_sets_index = 0;
        uv_sets_index += value.attributes.color_sets.unwrap_or(0) * 4; // colors use 4 floats each

        // for easy copy/paste
        let texture_pool_arrays_len = value.texture_pool_arrays_len;
        let texture_pool_samplers_len = value.texture_pool_samplers_len;
        let normals = value.attributes.normals;
        let tangents = value.attributes.tangents;
        let color_sets = value.attributes.color_sets;
        let uv_sets = value.attributes.uv_sets;
        let mipmap = if value.mipmaps {
            MipmapMode::Gradient
        } else {
            MipmapMode::None
        };
        let multisampled_geometry = value.msaa_sample_count.is_some();
        let msaa_sample_count = value.msaa_sample_count.unwrap_or_default();
        let debug = ShaderTemplateMaterialOpaqueDebug::new();

        let _self = Self {
            bind_groups: ShaderTemplateMaterialOpaqueBindGroups {
                texture_pool_arrays_len,
                texture_pool_samplers_len,
                color_sets_index,
                uv_sets_index,
                normals,
                tangents,
                color_sets,
                uv_sets,
                mipmap,
                multisampled_geometry,
                msaa_sample_count,
                unlit: value.unlit,
                debug,
            },
            compute: ShaderTemplateMaterialOpaqueCompute {
                texture_pool_arrays_len,
                texture_pool_samplers_len,
                color_sets_index,
                uv_sets_index,
                normals,
                tangents,
                color_sets,
                uv_sets,
                mipmap,
                multisampled_geometry,
                msaa_sample_count,
                debug,
                unlit: value.unlit,
            },
        };

        Ok(_self)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MipmapMode {
    None,
    Gradient,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShaderTemplateMaterialOpaqueDebug {
    mips: bool,
    n_dot_v: bool,
    normals: bool,
    base_color: bool,
    view_direction: bool,
    irradiance_sample: bool,
    msaa_detect_edges: bool,
    lighting: ShaderTemplateMaterialOpaqueDebugLighting,
}

impl ShaderTemplateMaterialOpaqueDebug {
    pub fn new() -> Self {
        Self { ..Self::default() }
    }
    pub fn any(&self) -> bool {
        self.mips
            || self.n_dot_v
            || self.normals
            || self.base_color
            || self.view_direction
            || self.irradiance_sample
            || self.msaa_detect_edges
            || !matches!(
                self.lighting,
                ShaderTemplateMaterialOpaqueDebugLighting::None
            )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ShaderTemplateMaterialOpaqueDebugLighting {
    #[default]
    None,
    IblOnly,
    PunctualOnly,
}

impl ShaderTemplateMaterialOpaque {
    pub fn into_source(self) -> Result<String> {
        let bind_groups_source = self.bind_groups.render()?;
        let compute_source = self.compute.render()?;

        let source = format!("{}\n{}", bind_groups_source, compute_source);
        // print_shader_source(&source, true);

        //debug_unique_string(1, &source, || print_shader_source(&source, false));

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}

impl TryFrom<&ShaderCacheKeyMaterialOpaqueEmpty> for ShaderTemplateMaterialOpaqueEmpty {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialOpaqueEmpty) -> Result<Self> {
        Ok(Self {
            texture_pool_arrays_len: value.texture_pool_arrays_len,
            texture_pool_samplers_len: value.texture_pool_samplers_len,
            multisampled_geometry: value.msaa_sample_count.is_some(),
            unlit: true,
        })
    }
}

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/empty.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaqueEmpty {
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub multisampled_geometry: bool,
    pub unlit: bool,
}

impl ShaderTemplateMaterialOpaqueEmpty {
    pub fn into_source(self) -> Result<String> {
        let source = self.render()?;
        // print_shader_source(&source, true);

        //debug_unique_string(1, &source, || print_shader_source(&source, false));

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque Empty")
    }
}
