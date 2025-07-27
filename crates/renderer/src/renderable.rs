use crate::{
    error::AwsmError, mesh::{Mesh, MeshKey}, pipeline::RenderPipelineKey, render::context::RenderContext, transform::TransformKey
};

pub enum Renderable<'a> {
    Mesh {
        key: MeshKey,
        mesh: &'a Mesh,
        has_alpha: bool,
    },
}

impl Renderable<'_> {
    pub fn render_pipeline_key(&self) -> RenderPipelineKey {
        match self {
            Self::Mesh { mesh, .. } => mesh.render_pipeline_key,
        }
    }

    pub fn has_alpha(&self) -> bool {
        match self {
            Self::Mesh { has_alpha, .. } => *has_alpha,
        }
    }

    pub fn transform_key(&self) -> Option<TransformKey> {
        match self {
            Self::Mesh { mesh, .. } => Some(mesh.transform_key),
        }
    }

    pub fn push_commands(&self, ctx: &mut RenderContext) -> Result<()> {
        match self {
            Self::Mesh { mesh, key, .. } => mesh.push_commands(ctx, *key),
        }
    }
}

type Result<T> = std::result::Result<T, AwsmError>;
