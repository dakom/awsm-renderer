use std::collections::{HashMap, HashSet};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    },
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use thiserror::Error;

use crate::{bind_group_layout::AwsmBindGroupLayoutError, render_passes::{composite::bind_group::CompositeBindGroups, display::bind_group::DisplayBindGroups, geometry::bind_group::GeometryBindGroups, light_culling::bind_group::LightCullingBindGroups, material::bind_group::MaterialBindGroups}, AwsmRenderer};

// There are no cache keys for bind groups, they are created on demand
// specifically, they are created according to the needs of the render pass
// and associated storages, uniforms, and textures
impl AwsmRenderer {
    pub fn recreate_marked_bind_groups(&mut self) -> Result<()> {
        if self.bind_groups.create_list.contains(&BindGroupCreate::Camera) || self.bind_groups.create_list.contains(&BindGroupCreate::Lights) {
            self.bind_groups.render_pass.geometry.camera_lights.recreate(&self.gpu, &mut self.bind_group_layouts, &self.camera, &self.lights)?;
        }

        if self.bind_groups.create_list.contains(&BindGroupCreate::Transforms) {
            self.bind_groups.render_pass.geometry.transforms.recreate(&self.gpu, &mut self.bind_group_layouts, &self.transforms)?;
        }

        self.bind_groups.create_list.clear();

        Ok(())
    }
}

// A change in raw buffer size causes a reallocation and so all
// bind groups that use that buffer need to be recreated
//
// however, a buffer can be used in multiple bind groups
// so the "create list" is somewhat agnostic and may affect multiple bind groups
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum BindGroupCreate {
    Camera,
    Lights,
    Transforms,
    MorphTargetWeights,
    MorphTargetValues,
    SkinJointMatrices,
    PbrMaterialUniform,
}

pub struct BindGroups {
    pub render_pass: RenderPassBindGroups,
    create_list: HashSet<BindGroupCreate>
}

impl BindGroups {
    pub fn new() -> Self {
        let mut create_list = HashSet::new();
        // insert all so groups will be created on first use
        // this also helps to debug if we missed anything
        create_list.insert(BindGroupCreate::Camera);
        create_list.insert(BindGroupCreate::Lights);
        create_list.insert(BindGroupCreate::Transforms);
        create_list.insert(BindGroupCreate::MorphTargetWeights);
        create_list.insert(BindGroupCreate::MorphTargetValues);
        create_list.insert(BindGroupCreate::SkinJointMatrices);
        create_list.insert(BindGroupCreate::PbrMaterialUniform);

        Self {
            render_pass: RenderPassBindGroups::default(),
            create_list,
        }
    }

    pub fn mark_create(&mut self, create: BindGroupCreate) {
        self.create_list.insert(create);
    }
}

#[derive(Default)]
pub struct RenderPassBindGroups {
    pub geometry: GeometryBindGroups,
    pub light_culling: LightCullingBindGroups,
    pub material: MaterialBindGroups,
    pub composite: CompositeBindGroups,
    pub display: DisplayBindGroups,
}

pub(super) type Result<T> = std::result::Result<T, AwsmBindGroupError>;

#[derive(Error, Debug)]
pub enum AwsmBindGroupError {
    #[error("[bind group] Error creating buffer for {label}: {err:?}")]
    CreateBuffer {
        label: &'static str,
        err: AwsmCoreError,
    },

    #[error("[bind group] Error writing buffer for {label}: {err:?}")]
    WriteBuffer {
        label: &'static str,
        err: AwsmCoreError,
    },

    #[error("[bind group] layout: {0:?}")]
    Layout(#[from] AwsmBindGroupLayoutError),

    #[error("[bind group] bind group not found for {0}")]
    NotFound(String)
}
