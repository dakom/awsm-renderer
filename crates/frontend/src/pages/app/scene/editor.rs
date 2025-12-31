use crate::{
    models::collections::GltfId,
    pages::app::scene::editor::{
        pipelines::EditorPipelines, render::render_grid, transform_controller::TransformController,
    },
    prelude::*,
};
use anyhow::Result;
use awsm_renderer::{
    gltf::{
        data::{GltfData, GltfDataHints},
        loader::GltfLoader,
    },
    mesh::MeshKey,
    render::RenderHooks,
    AwsmRenderer,
};
use dominator_helpers::futures::AsyncLoader;
use futures::StreamExt;

pub mod pipelines;
pub mod render;
pub mod transform_controller;

#[derive(Clone)]
pub struct AppSceneEditor {
    pub pipelines: Arc<EditorPipelines>,
    pub render_hooks: Arc<std::sync::RwLock<Option<Arc<RenderHooks>>>>,
    pub gizmo_gltf_data: Arc<GltfData>,
    pub transform_controller: Arc<std::sync::Mutex<Option<TransformController>>>,
    grid_enabled: Mutable<bool>,
    gizmo_translation_enabled: Mutable<bool>,
    gizmo_rotation_enabled: Mutable<bool>,
    gizmo_scale_enabled: Mutable<bool>,
    reactor: AsyncLoader,
}

impl AppSceneEditor {
    pub async fn new(
        renderer: Arc<futures::lock::Mutex<AwsmRenderer>>,
        grid_enabled: Mutable<bool>,
        gizmo_translation_enabled: Mutable<bool>,
        gizmo_rotation_enabled: Mutable<bool>,
        gizmo_scale_enabled: Mutable<bool>,
    ) -> Result<Self> {
        let gizmo_gltf_data = Arc::new(
            GltfLoader::load(&GltfId::AwsmTransformGizmo.url(), None)
                .await?
                .into_data(Some(
                    GltfDataHints::default().with_hud(true).with_hidden(true),
                ))?,
        );

        let pipelines = Arc::new(EditorPipelines::load(&mut *renderer.lock().await).await?);

        let render_hooks = Arc::new(std::sync::RwLock::new(None));

        let transform_controller: Arc<std::sync::Mutex<Option<TransformController>>> =
            Arc::new(std::sync::Mutex::new(None));
        let reactor = AsyncLoader::new();

        reactor.load(clone!(grid_enabled, gizmo_translation_enabled, gizmo_rotation_enabled, gizmo_scale_enabled, render_hooks, pipelines, renderer, transform_controller => async move {

            let mut stream = map_ref! {
                let grid_enabled = grid_enabled.signal(),
                let gizmo_translation_enabled = gizmo_translation_enabled.signal(),
                let gizmo_rotation_enabled = gizmo_rotation_enabled.signal(),
                let gizmo_scale_enabled = gizmo_scale_enabled.signal(),
                => (*grid_enabled, *gizmo_translation_enabled, *gizmo_rotation_enabled, *gizmo_scale_enabled)
            }.to_stream();

            while let Some((grid_enabled, gizmo_translation_enabled, gizmo_rotation_enabled, gizmo_scale_enabled)) = stream.next().await {
                let mut render_hooks = render_hooks.write().unwrap();

                *render_hooks = Some(Arc::new(RenderHooks {
                    after_opaque: if grid_enabled {
                        let grid_bind_group = pipelines.grid_bind_group.clone();
                        let grid_pipeline_msaa_4_key = pipelines.grid_pipeline_msaa_4_key;
                        let grid_pipeline_singlesampled_key = pipelines.grid_pipeline_singlesampled_key;

                        Some(Box::new(move |ctx| {
                            let grid_pipeline_key = match ctx.anti_aliasing.msaa_sample_count {
                                Some(4) => grid_pipeline_msaa_4_key,
                                None => grid_pipeline_singlesampled_key,
                                _ => panic!("Unsupported MSAA sample count for grid pipeline"),
                            };

                            render_grid(
                                ctx,
                                &grid_bind_group,
                                grid_pipeline_key,
                            )
                        }))
                    } else {
                        None
                    },
                }));


                if let Some(transform_controller) = transform_controller.lock().unwrap().as_mut() {
                    if let Err(err) = transform_controller.set_hidden(&mut *renderer.lock().await, !gizmo_translation_enabled, !gizmo_rotation_enabled, !gizmo_scale_enabled) {
                        tracing::error!("Error setting transform controller enabled state: {}", err);
                    }
                }
            }
        }));

        Ok(Self {
            render_hooks,
            pipelines,
            grid_enabled,
            gizmo_translation_enabled,
            gizmo_rotation_enabled,
            gizmo_scale_enabled,
            reactor,
            gizmo_gltf_data,
            transform_controller,
        })
    }

    pub fn start_pick(&self, mesh_key: MeshKey, x: i32, y: i32) {
        if let Some(transform_controller) = self.transform_controller.lock().unwrap().as_mut() {
            transform_controller.start_pick(mesh_key, x, y);
        }
    }
}
