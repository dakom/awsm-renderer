use crate::{
    pages::app::scene::editor::{pipelines::EditorPipelines, render::render_grid},
    prelude::*,
};
use anyhow::Result;
use awsm_renderer::{render::RenderHooks, AwsmRenderer};
use dominator_helpers::futures::AsyncLoader;
use futures::StreamExt;

pub mod pipelines;
pub mod render;

#[derive(Clone)]
pub struct AppSceneEditor {
    pub pipelines: Arc<EditorPipelines>,
    pub render_hooks: Arc<std::sync::RwLock<Option<Arc<RenderHooks>>>>,
    grid_enabled: Mutable<bool>,
    gizmos_enabled: Mutable<bool>,
    reactor: AsyncLoader,
}

impl AppSceneEditor {
    pub async fn new(
        renderer: &mut AwsmRenderer,
        grid_enabled: Mutable<bool>,
        gizmos_enabled: Mutable<bool>,
    ) -> Result<Self> {
        let pipelines = Arc::new(EditorPipelines::load(renderer).await?);

        let render_hooks = Arc::new(std::sync::RwLock::new(None));

        let reactor = AsyncLoader::new();

        reactor.load(clone!(grid_enabled, gizmos_enabled, render_hooks, pipelines => async move {

            let mut stream = map_ref! {
                let grid_enabled = grid_enabled.signal(),
                let gizmos_enabled = gizmos_enabled.signal()
                => (grid_enabled.clone(), gizmos_enabled.clone())
            }.to_stream();

            while let Some((grid_enabled, _gizmos_enabled)) = stream.next().await {
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
                    // You can add gizmos rendering here similarly
                    ..Default::default()
                }));
            }
        }));

        Ok(Self {
            render_hooks,
            pipelines,
            grid_enabled,
            gizmos_enabled,
            reactor,
        })
    }

    // pub fn set_grid_render_hook(&self, flag: bool) {
    //     let mut render_hooks = self.render_hooks.write().unwrap();
    //     if flag {
    //         let grid_bind_group = self.pipelines.grid_bind_group.clone();
    //         let grid_pipeline_msaa_4_key = self.pipelines.grid_pipeline_msaa_4_key;
    //         let grid_pipeline_singlesampled_key = self.pipelines.grid_pipeline_singlesampled_key;

    //         *render_hooks = Some(RenderHooks {
    //             after_opaque: Some(Box::new(move |ctx| {
    //                 let grid_pipeline_key = match ctx.anti_aliasing.msaa_sample_count {
    //                     Some(4) => grid_pipeline_msaa_4_key,
    //                     None => grid_pipeline_singlesampled_key,
    //                     _ => panic!("Unsupported MSAA sample count for grid pipeline"),
    //                 };

    //                 crate::pages::app::scene::editor::render::render_grid(
    //                     ctx,
    //                     &grid_bind_group,
    //                     grid_pipeline_key,
    //                 )
    //             })),
    //             ..Default::default()
    //         });
    //     } else {
    //         *render_hooks = None;
    //     }
    // }
}
