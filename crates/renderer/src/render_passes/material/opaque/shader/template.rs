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
    pub normals: bool,
    pub tangents: bool,
    pub color_sets: Option<u32>,
    /// Number of UV sets available on the mesh.
    /// `None` means the mesh supplied no TEXCOORD attributes, which triggers the
    /// `pbr_material_has_any_uvs` branch inside `pbr_should_run`.
    pub uv_sets: Option<u32>,
    pub debug: ShaderTemplateMaterialOpaqueDebug,
    pub mipmap: MipmapMode,
}

impl TryFrom<&ShaderCacheKeyMaterialOpaque> for ShaderTemplateMaterialOpaque {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialOpaque) -> Result<Self> {
        // see `impl Ord for MeshBufferVertexAttributeInfo`
        // for ordering here
        //
        let mut uv_sets_index = 0;
        if value.attributes.normals {
            uv_sets_index += 3; // normals always consume 3 floats
        }
        if value.attributes.tangents {
            uv_sets_index += 4; // tangents use 4 floats
        }
        uv_sets_index += (value.attributes.color_sets.unwrap_or(0) * 4) as u32; // colors use 4 floats each

        let _self = Self {
            texture_atlas_len: value.texture_atlas_len,
            sampler_atlas_len: value.sampler_atlas_len,
            uv_sets_index,
            normals: value.attributes.normals,
            tangents: value.attributes.tangents,
            color_sets: value.attributes.color_sets,
            uv_sets: value.attributes.uv_sets,
            mipmap: MipmapMode::Lod,
            debug: ShaderTemplateMaterialOpaqueDebug {
                ..Default::default()
            },
        };

        Ok(_self)
    }
}

#[derive(Debug)]
enum MipmapMode {
    None,
    Lod,
}

#[derive(Debug, Default)]
struct ShaderTemplateMaterialOpaqueDebug {
    mips: bool,
}

impl ShaderTemplateMaterialOpaque {
    pub fn into_source(self) -> Result<String> {
        let source = self.render()?;

        //debug_unique_string(1, &source, || print_shader_source(&source, true));

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}
