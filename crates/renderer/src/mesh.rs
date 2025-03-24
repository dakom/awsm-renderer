use awsm_renderer_core::command::render_pass::{ColorAttachment, RenderPassDescriptor};
use awsm_renderer_core::command::{LoadOp, StoreOp};

use crate::render::RenderContext;
use crate::error::Result;

#[derive(Default)]
pub struct Meshes {
    // TODO - replace with slotmap
    lookup: Vec<Mesh>
}

// TODO - replace with slotmap
pub type MeshKey = usize;

impl Meshes {
    pub fn add(&mut self, mesh: Mesh) -> MeshKey {
        let key = self.lookup.len();
        self.lookup.push(mesh);
        key
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mesh> {
        self.lookup.iter()
    }

    pub fn iter_with_key(&self) -> impl Iterator<Item = (MeshKey, &Mesh)> {
        self.lookup.iter().enumerate()
    }
}

pub struct Mesh {
    pub pipeline: web_sys::GpuRenderPipeline
}

impl Mesh {
    pub fn new(pipeline: web_sys::GpuRenderPipeline) -> Self {
        Self {
            pipeline
        }
    }

    pub fn push_commands(&self, key: MeshKey, ctx: &mut RenderContext) -> Result<()> {
        tracing::info!("Rendering mesh: {key}");

        let RenderContext {
            current_texture_view,
            command_encoder
        } = ctx;


        let render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![ColorAttachment::new(
                    &current_texture_view,
                    LoadOp::Clear,
                    StoreOp::Store,
                )],
                ..Default::default()
            }
            .into(),
        )?;

        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(3);
        render_pass.end();

        Ok(())
    }

}