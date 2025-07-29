use crate::shaders::{
    fragment::{
        entry::pbr::ShaderCacheKeyFragmentPbr,
        template::{DynamicBufferBinding, ShaderTemplateFragmentLocation},
    },
    vertex::ShaderTemplateVertexToFragmentAssignment,
};
use askama::Template;

#[derive(Template, Debug)]
#[template(path = "fragment/pbr.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateFragmentPbr {
    pub fragment_buffer_bindings: Vec<DynamicBufferBinding>,
    pub fragment_input_locations: Vec<ShaderTemplateFragmentLocation>,
    pub has_alpha_mask: bool,
    pub has_normals: bool,
    // the idea here is that with these gates, we can write normal shader code
    // since the variables are assigned (and from then on, we don't care about the location)
    pub has_base_color_tex: bool,
    pub has_metallic_roughness_tex: bool,
    pub has_emissive_tex: bool,
    pub has_occlusion_tex: bool,
    pub has_normal_tex: bool,
}

impl ShaderTemplateFragmentPbr {
    pub fn new(
        cache_key: &ShaderCacheKeyFragmentPbr,
        vertex_to_fragment_assignments: &mut Vec<ShaderTemplateVertexToFragmentAssignment>,
    ) -> Self {
        let mut fragment_buffer_bindings = Vec::new();
        let mut fragment_input_locations = Vec::new();

        let mut push_texture = |name: &str, uv_index: u32| {
            fragment_buffer_bindings.push(DynamicBufferBinding {
                group: 2,
                index: fragment_buffer_bindings.len() as u32,
                name: format!("{name}_tex"),
                data_type: "texture_2d<f32>".to_string(),
            });

            fragment_buffer_bindings.push(DynamicBufferBinding {
                group: 2,
                index: fragment_buffer_bindings.len() as u32,
                name: format!("{name}_sampler"),
                data_type: "sampler".to_string(),
            });

            fragment_input_locations.push(ShaderTemplateFragmentLocation {
                location: fragment_input_locations.len() as u32,
                name: format!("{name}_uv"),
                data_type: "vec2<f32>".to_string(),
            });

            vertex_to_fragment_assignments.push(ShaderTemplateVertexToFragmentAssignment {
                vertex_name: format!("uv_{uv_index}"),
                fragment_name: format!("{name}_uv"),
            });
        };

        if let Some(uv_index) = cache_key.base_color_uv_index {
            push_texture("base_color", uv_index);
        }

        if let Some(uv_index) = cache_key.metallic_roughness_uv_index {
            push_texture("metallic_roughness", uv_index);
        }

        if let Some(uv_index) = cache_key.normal_uv_index {
            push_texture("normal", uv_index);
        }

        if let Some(uv_index) = cache_key.occlusion_uv_index {
            push_texture("occlusion", uv_index);
        }

        if let Some(uv_index) = cache_key.emissive_uv_index {
            push_texture("emissive", uv_index);
        }

        if cache_key.has_normals {
            fragment_input_locations.push(ShaderTemplateFragmentLocation {
                location: fragment_input_locations.len() as u32,
                name: "world_normal".to_string(),
                data_type: "vec3<f32>".to_string(),
            });
        }

        for location in &mut fragment_input_locations {
            const HARDCODED_LOCATION_LEN: u32 = 2; // account for hardcoded locations like world_position and clip_position
            location.location += HARDCODED_LOCATION_LEN;
        }

        Self {
            has_normals: cache_key.has_normals,
            has_alpha_mask: cache_key.has_alpha_mask,
            has_base_color_tex: cache_key.base_color_uv_index.is_some(),
            has_metallic_roughness_tex: cache_key.metallic_roughness_uv_index.is_some(),
            has_emissive_tex: cache_key.emissive_uv_index.is_some(),
            has_occlusion_tex: cache_key.occlusion_uv_index.is_some(),
            has_normal_tex: cache_key.normal_uv_index.is_some(),
            fragment_buffer_bindings,
            fragment_input_locations,
        }
    }
}
