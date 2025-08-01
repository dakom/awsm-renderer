use std::collections::{HashMap, HashSet};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    },
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use strum::{EnumIter, IntoEnumIterator};
use thiserror::Error;

use crate::{bind_group_layout::{AwsmBindGroupLayoutError, BindGroupLayouts}, camera::CameraBuffer, lights::Lights, materials::Materials, mesh::Meshes, render_passes::{composite::bind_group::CompositeBindGroups, display::bind_group::DisplayBindGroups, geometry::bind_group::GeometryBindGroups, light_culling::bind_group::LightCullingBindGroups, material::bind_group::MaterialBindGroups, RenderPasses}, render_textures::RenderTextureViews, textures::Textures, transforms::Transforms, AwsmRenderer};

// There are no cache keys for bind groups, they are created on demand
// Since changes to storages, uniforms, and textures are the reason to recreate bind groups,
// and these may be shared across multiple bind groups, we use a "create list" to track which bind groups need to be recreated
//
// Specifically, typical causes of change are:
// 1. A change in raw buffer size which causes a reallocation
// 2. A change in texture view size which causes new textures to be created
//
// That conscpicuously does not include changes to material textures
// since those are looked up via the material key and do not require a bind group recreation
impl AwsmRenderer {
}

pub struct BindGroupRecreateContext<'a> {
    pub gpu: &'a AwsmRendererWebGpu,
    pub render_texture_views: &'a RenderTextureViews,
    pub textures: &'a Textures,
    pub materials: &'a Materials,
    pub bind_group_layouts: &'a BindGroupLayouts,
    pub meshes: &'a Meshes,
    pub camera: &'a CameraBuffer,
    pub lights: &'a Lights,
    pub transforms: &'a Transforms,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum BindGroupCreate {
    CameraInitOnly,
    LightsResize,
    TransformsResize,
    MorphTargetWeightsResize,
    MorphTargetValuesResize,
    SkinJointMatricesResize,
    PbrMaterialUniformResize,
    TextureViewResize
}

pub struct BindGroups {
    create_list: HashSet<BindGroupCreate>
}

impl BindGroups {
    pub fn new() -> Self {
        Self {
            // startup means all bind groups are "re"created
            create_list: BindGroupCreate::iter().collect::<HashSet<_>>()
        }
    }

    pub fn mark_create(&mut self, create: BindGroupCreate) {
        self.create_list.insert(create);
    }


    pub fn recreate(&mut self, ctx: BindGroupRecreateContext<'_>, render_passes: &mut RenderPasses) -> crate::error::Result<()> {
        if self.create_list.contains(&BindGroupCreate::CameraInitOnly) || self.create_list.contains(&BindGroupCreate::LightsResize) {
            render_passes.geometry.bind_groups.camera_lights.recreate(&ctx)?;
        }

        // for now we also do this for material changes - will be moved to something else later 
        if self.create_list.contains(&BindGroupCreate::TransformsResize) || self.create_list.contains(&BindGroupCreate::PbrMaterialUniformResize) {
            render_passes.geometry.bind_groups.transforms.recreate(&ctx)?;
        }

        if self.create_list.contains(&BindGroupCreate::MorphTargetWeightsResize) 
        || self.create_list.contains(&BindGroupCreate::MorphTargetValuesResize) 
        || self.create_list.contains(&BindGroupCreate::SkinJointMatricesResize) {
            render_passes.geometry.bind_groups.vertex_animation.recreate(&ctx)?;
        }

        if self.create_list.contains(&BindGroupCreate::TextureViewResize) {
            render_passes.display.bind_groups.recreate(&ctx)?;
        }

        self.create_list.clear();

        Ok(())
    }
}

pub(super) type Result<T> = std::result::Result<T, AwsmBindGroupError>;

#[derive(Error, Debug)]
pub enum AwsmBindGroupError {
    #[error("[bind group] bind group not found for {0}")]
    NotFound(String),
}
