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

use crate::{
    anti_alias::AntiAliasing, bind_group_layout::BindGroupLayouts, camera::CameraBuffer,
    environment::Environment, lights::Lights, materials::Materials, mesh::Meshes,
    render_passes::RenderPasses, render_textures::RenderTextureViews, textures::Textures,
    transforms::Transforms,
};

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
pub struct BindGroupRecreateContext<'a> {
    pub gpu: &'a AwsmRendererWebGpu,
    pub render_texture_views: &'a RenderTextureViews,
    pub textures: &'a Textures,
    pub materials: &'a Materials,
    pub bind_group_layouts: &'a BindGroupLayouts,
    pub meshes: &'a Meshes,
    pub camera: &'a CameraBuffer,
    pub environment: &'a Environment,
    pub lights: &'a Lights,
    pub transforms: &'a Transforms,
    pub anti_aliasing: &'a AntiAliasing,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, EnumIter)]
pub enum BindGroupCreate {
    CameraInitOnly,
    LightsResize,
    LightsInfoCreate,
    BrdfLutTextures,
    IblTextures,
    EnvironmentSkyboxCreate,
    TransformsResize,
    TransformNormalsResize,
    GeometryMorphTargetWeightsResize,
    GeometryMorphTargetValuesResize,
    MaterialMorphTargetWeightsResize,
    MaterialMorphTargetValuesResize,
    SkinJointMatricesResize,
    SkinJointIndexAndWeightsResize,
    GeometryMeshMetaResize,
    MaterialMeshMetaResize,
    MeshAttributeDataResize,
    MeshAttributeIndexResize,
    PbrMaterialResize,
    TextureViewResize,
    TexturePool,
    TextureTransformsResize,
    AntiAliasingChange,
}

pub struct BindGroups {
    create_list: HashSet<BindGroupCreate>,
}

impl BindGroups {
    pub fn new() -> Self {
        Self {
            // startup means all bind groups are "re"created
            create_list: BindGroupCreate::iter().collect::<HashSet<_>>(),
        }
    }

    pub fn mark_create(&mut self, create: BindGroupCreate) {
        self.create_list.insert(create);
    }

    pub fn recreate(
        &mut self,
        ctx: BindGroupRecreateContext<'_>,
        render_passes: &mut RenderPasses,
    ) -> crate::error::Result<()> {
        if self.create_list.is_empty() {
            return Ok(());
        }

        #[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
        enum FunctionToCall {
            GeometryCamera,
            GeometryTransformMaterials,
            GeometryMeta,
            GeometryAnimation,
            OpaqueMain,
            OpaqueLights,
            OpaqueTextures,
            OpaqueSamplers,
            TransparentMain,
            TransparentLights,
            TransparentTextures,
            TransparentSamplers,
            LightCulling,
            Composite,
            Display,
        }

        let mut functions_to_call = HashSet::new();

        for create in self.create_list.drain() {
            match create {
                BindGroupCreate::CameraInitOnly => {
                    functions_to_call.insert(FunctionToCall::GeometryCamera);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::LightsInfoCreate => {
                    functions_to_call.insert(FunctionToCall::OpaqueLights);
                    functions_to_call.insert(FunctionToCall::TransparentLights);
                }
                BindGroupCreate::LightsResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueLights);
                    functions_to_call.insert(FunctionToCall::TransparentLights);
                }
                BindGroupCreate::TransformsResize => {
                    functions_to_call.insert(FunctionToCall::GeometryTransformMaterials);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::PbrMaterialResize => {
                    functions_to_call.insert(FunctionToCall::GeometryTransformMaterials);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::GeometryMeshMetaResize => {
                    functions_to_call.insert(FunctionToCall::GeometryMeta);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::GeometryMorphTargetWeightsResize
                | BindGroupCreate::GeometryMorphTargetValuesResize
                | BindGroupCreate::SkinJointMatricesResize
                | BindGroupCreate::SkinJointIndexAndWeightsResize => {
                    functions_to_call.insert(FunctionToCall::GeometryAnimation);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::TextureViewResize => {
                    functions_to_call.insert(FunctionToCall::LightCulling);
                    functions_to_call.insert(FunctionToCall::Composite);
                    functions_to_call.insert(FunctionToCall::Display);
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::TexturePool => {
                    functions_to_call.insert(FunctionToCall::OpaqueTextures);
                    functions_to_call.insert(FunctionToCall::OpaqueSamplers);
                    functions_to_call.insert(FunctionToCall::TransparentTextures);
                    functions_to_call.insert(FunctionToCall::TransparentSamplers);
                }
                BindGroupCreate::TextureTransformsResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueTextures);
                    functions_to_call.insert(FunctionToCall::TransparentTextures);
                }
                BindGroupCreate::BrdfLutTextures => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::IblTextures => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::EnvironmentSkyboxCreate => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                }
                BindGroupCreate::TransformNormalsResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::MaterialMorphTargetWeightsResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::MaterialMorphTargetValuesResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::MaterialMeshMetaResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::MeshAttributeDataResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::MeshAttributeIndexResize => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
                BindGroupCreate::AntiAliasingChange => {
                    functions_to_call.insert(FunctionToCall::OpaqueMain);
                    functions_to_call.insert(FunctionToCall::TransparentMain);
                }
            }
        }

        for f in functions_to_call {
            match f {
                FunctionToCall::GeometryCamera => {
                    render_passes.geometry.bind_groups.camera.recreate(&ctx)?;
                }
                FunctionToCall::GeometryTransformMaterials => {
                    render_passes
                        .geometry
                        .bind_groups
                        .transform_materials
                        .recreate(&ctx)?;
                }
                FunctionToCall::GeometryMeta => {
                    render_passes.geometry.bind_groups.meta.recreate(&ctx)?;
                }
                FunctionToCall::GeometryAnimation => {
                    render_passes
                        .geometry
                        .bind_groups
                        .animation
                        .recreate(&ctx)?;
                }
                FunctionToCall::OpaqueMain => {
                    render_passes
                        .material_opaque
                        .bind_groups
                        .recreate_main(&ctx)?;
                }
                FunctionToCall::OpaqueLights => {
                    render_passes
                        .material_opaque
                        .bind_groups
                        .recreate_lights(&ctx)?;
                }
                FunctionToCall::OpaqueTextures => {
                    render_passes
                        .material_opaque
                        .bind_groups
                        .recreate_texture_pool_textures(&ctx)?;
                }
                FunctionToCall::OpaqueSamplers => {
                    render_passes
                        .material_opaque
                        .bind_groups
                        .recreate_texture_pool_samplers(&ctx)?;
                }
                FunctionToCall::TransparentMain => {
                    render_passes
                        .material_transparent
                        .bind_groups
                        .recreate_main(&ctx)?;
                }
                FunctionToCall::TransparentLights => {
                    render_passes
                        .material_transparent
                        .bind_groups
                        .recreate_lights(&ctx)?;
                }
                FunctionToCall::TransparentTextures => {
                    render_passes
                        .material_transparent
                        .bind_groups
                        .recreate_texture_pool_textures(&ctx)?;
                }
                FunctionToCall::TransparentSamplers => {
                    render_passes
                        .material_transparent
                        .bind_groups
                        .recreate_texture_pool_samplers(&ctx)?;
                }
                FunctionToCall::LightCulling => {
                    render_passes.light_culling.bind_groups.recreate(&ctx)?;
                }
                FunctionToCall::Composite => {
                    render_passes.composite.bind_groups.recreate(&ctx)?;
                }
                FunctionToCall::Display => {
                    render_passes.display.bind_groups.recreate(&ctx)?;
                }
            }
        }

        Ok(())
    }
}

pub(super) type Result<T> = std::result::Result<T, AwsmBindGroupError>;

#[derive(Error, Debug)]
pub enum AwsmBindGroupError {
    #[error("[bind group] bind group not found for {0}")]
    NotFound(String),
}
