use askama::Template;
use awsm_renderer_core::texture::mega_texture::MegaTextureBindings;

use crate::{
    debug::{debug_once, debug_unique_string},
    render_passes::material::opaque::shader::{
        attributes::ShaderMaterialOpaqueVertexAttributes, cache_key::ShaderCacheKeyMaterialOpaque,
    },
    shaders::{print_shader_source, AwsmShaderError, Result},
    textures::SamplerBindings,
};

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaque {
    /// Offset (in floats) within the packed vertex attribute array
    /// where the first UV component lives for each vertex.
    pub uv_sets_index: u32,
    pub total_atlas_index: u32,
    pub texture_bindings: Vec<TextureBinding>,
    pub sampler_bindings: Vec<SamplerBinding>,
    pub default_sampler_index: Option<u32>,
    pub has_atlas: bool,
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

#[derive(Debug)]
pub struct TextureBinding {
    group: u32,
    binding: u32,
    atlas_index: u32,
}

#[derive(Debug)]
pub struct SamplerBinding {
    group: u32,
    binding: u32,
    sampler_index: u32,
}

impl TryFrom<&ShaderCacheKeyMaterialOpaque> for ShaderTemplateMaterialOpaque {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialOpaque) -> Result<Self> {
        let MegaTextureBindings {
            start_group,
            start_binding,
            bind_group_bindings_len,
        } = &value.texture_bindings;

        let mut texture_bindings = Vec::new();

        let mut total_atlas_index = 0;
        for (texture_group_index, &len) in bind_group_bindings_len.iter().enumerate() {
            let group = start_group + texture_group_index as u32;

            let mut binding_start = if texture_group_index == 0 {
                *start_binding
            } else {
                0
            };

            for i in 0..len {
                let binding = binding_start + i as u32;
                texture_bindings.push(TextureBinding {
                    group,
                    binding,
                    atlas_index: total_atlas_index,
                });
                total_atlas_index += 1;
            }
        }

        let SamplerBindings {
            start_group: sampler_start_group,
            start_binding: sampler_start_binding,
            bind_group_bindings_len: sampler_bindings_len,
        } = &value.sampler_bindings;

        let mut sampler_bindings = Vec::new();
        let mut total_sampler_index = 0u32;
        for (sampler_group_index, &len) in sampler_bindings_len.iter().enumerate() {
            if len == 0 {
                continue;
            }

            let group = sampler_start_group + sampler_group_index as u32;
            let binding_start = if sampler_group_index == 0 {
                *sampler_start_binding
            } else {
                0
            };

            for i in 0..len {
                sampler_bindings.push(SamplerBinding {
                    group,
                    binding: binding_start + i as u32,
                    sampler_index: total_sampler_index,
                });
                total_sampler_index += 1;
            }
        }

        let default_sampler_index = sampler_bindings
            .first()
            .map(|binding| binding.sampler_index);

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
            texture_bindings,
            sampler_bindings,
            default_sampler_index,
            total_atlas_index,
            uv_sets_index,
            has_atlas: total_atlas_index > 0,
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
    Gradient,
    Lod,
}

#[derive(Debug, Default)]
struct ShaderTemplateMaterialOpaqueDebug {
    mips: bool,
}

impl ShaderTemplateMaterialOpaque {
    pub fn into_source(self) -> Result<String> {
        let source = self.render()?;

        //debug_unique_string(1, &source, || print_shader_source(&source, false));

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}
