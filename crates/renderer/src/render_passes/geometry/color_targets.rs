use awsm_renderer_core::pipeline::fragment::ColorTargetState;

use crate::render_textures::RenderTextureFormats;

pub fn geometry_fragment_color_targets(formats: &RenderTextureFormats) -> [ColorTargetState; 4] {
    [
        ColorTargetState::new(formats.material_offset),
        ColorTargetState::new(formats.world_normal),
        ColorTargetState::new(formats.screen_pos),
        ColorTargetState::new(formats.motion_vector),
    ]
}
