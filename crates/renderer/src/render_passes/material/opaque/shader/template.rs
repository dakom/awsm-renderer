use askama::Template;
use awsm_renderer_core::texture::mega_texture::MegaTextureBindings;

use crate::{
    debug::{debug_once, debug_unique_string},
    render_passes::material::opaque::shader::cache_key::ShaderCacheKeyMaterialOpaque,
    shaders::{print_shader_source, AwsmShaderError, Result},
};

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaque {
    pub texture_binding_strings: Vec<String>,
    pub texture_load_case_strings: Vec<String>,
    pub has_atlas: bool,
}

impl TryFrom<&ShaderCacheKeyMaterialOpaque> for ShaderTemplateMaterialOpaque {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialOpaque) -> Result<Self> {
        let MegaTextureBindings {
            start_group,
            start_binding,
            bind_group_bindings_len,
        } = &value.texture_bindings;

        tracing::info!("{:#?}", value.texture_bindings);

        let mut texture_binding_strings = Vec::new();

        let mut total_index = 0;
        for (texture_group_index, &len) in bind_group_bindings_len.iter().enumerate() {
            let group_index = texture_group_index as u32 + start_group;

            let mut binding_start = if texture_group_index == 0 {
                *start_binding
            } else {
                0
            };

            for i in 0..len {
                let binding_index = binding_start + i as u32;
                texture_binding_strings.push(format!("@group({group_index}) @binding({binding_index}) var atlas_tex_{total_index}: texture_2d_array<f32>;"));
                total_index += 1;
            }
        }

        let mut texture_load_case_strings = Vec::new();
        for i in 0..total_index {
            texture_load_case_strings.push(format!(
                "case {i}u: {{ return texture_load_atlas_binding(info, atlas_tex_{i}, attribute_uv); }}"
            ));
        }

        tracing::info!("{:#?}", texture_binding_strings);

        Ok(Self {
            texture_binding_strings,
            texture_load_case_strings,
            has_atlas: total_index > 0,
        })
    }
}

impl ShaderTemplateMaterialOpaque {
    pub fn into_source(self) -> Result<String> {
        let source = self.render()?;

        // debug_unique_string(1, &source, || print_shader_source(&source, true));

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}
