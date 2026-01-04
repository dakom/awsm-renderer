use anyhow::{Context, Result};
use awsm_renderer::{
    debug::debug_unique_string, gltf::GltfKeyLookups, mesh::MeshKey, transforms::TransformKey,
    AwsmRenderer,
};
use glam::Vec3;

use crate::{config::CONFIG, pages::app::scene::camera::Camera};

#[derive(Clone, Debug)]
pub struct TransformController {
    pub mesh_keys: TransformControllerMeshKeys,
    pub transform_keys: TransformControllerTransformKeys,
}

#[derive(Clone, Debug)]
pub struct TransformControllerMeshKeys {
    pub cube_x: MeshKey,
    pub cube_y: MeshKey,
    pub cube_z: MeshKey,

    pub ring_x: MeshKey,
    pub ring_y: MeshKey,
    pub ring_z: MeshKey,

    pub arrow_x: MeshKey,
    pub arrow_y: MeshKey,
    pub arrow_z: MeshKey,
}

#[derive(Clone, Debug)]
pub struct TransformControllerTransformKeys {
    pub root: TransformKey,
}

#[derive(Clone, Debug)]
enum MeshKind {
    TranslationX,
    TranslationY,
    TranslationZ,
    RotationX,
    RotationY,
    RotationZ,
    ScaleX,
    ScaleY,
    ScaleZ,
}

impl TransformController {
    pub fn new(renderer: &mut AwsmRenderer, lookups: &GltfKeyLookups) -> Result<Self> {
        let _self = Self {
            mesh_keys: TransformControllerMeshKeys::new(lookups)?,
            transform_keys: TransformControllerTransformKeys::new(lookups)?,
        };

        _self.set_hidden(
            renderer,
            !CONFIG.initial_show_gizmo_translation,
            !CONFIG.initial_show_gizmo_rotation,
            !CONFIG.initial_show_gizmo_scale,
        )?;

        Ok(_self)
    }

    pub fn zoom_gizmo_transforms(
        &self,
        renderer: &mut AwsmRenderer,
        camera: &Camera,
    ) -> awsm_renderer::error::Result<()> {
        let mut transform = renderer
            .transforms
            .get_local(self.transform_keys.root)?
            .clone();

        const DESIRED_PIXEL_SIZE: f32 = 100.0; // Desired size in pixels
        const REFERENCE_SIZE: f32 = 1.0; // Reference size of the gizmo in world

        let (_, viewport_y) = renderer.gpu.canvas_size(false);

        let desired_ndc = 2.0 * DESIRED_PIXEL_SIZE / viewport_y as f32;
        let proj11 = camera.projection_matrix().y_axis.y;

        let depth = if camera.is_orthographic() {
            1.0
        } else {
            let cam_pos = camera.position_world();
            let gizmo_pos = transform.translation;
            (gizmo_pos - cam_pos).length()
        };

        debug_unique_string(1, &transform.translation.to_string(), || {
            tracing::info!("Gizmo position: {:?}", transform.translation);
        });
        let scale = (desired_ndc * depth / proj11) / REFERENCE_SIZE;

        transform.scale = Vec3::new(scale, scale, scale);

        renderer
            .transforms
            .set_local(self.transform_keys.root, transform)?;

        Ok(())
    }

    fn get_mesh_kind(&self, mesh_key: MeshKey) -> Option<MeshKind> {
        if mesh_key == self.mesh_keys.arrow_x {
            Some(MeshKind::TranslationX)
        } else if mesh_key == self.mesh_keys.arrow_y {
            Some(MeshKind::TranslationY)
        } else if mesh_key == self.mesh_keys.arrow_z {
            Some(MeshKind::TranslationZ)
        } else if mesh_key == self.mesh_keys.ring_x {
            Some(MeshKind::RotationX)
        } else if mesh_key == self.mesh_keys.ring_y {
            Some(MeshKind::RotationY)
        } else if mesh_key == self.mesh_keys.ring_z {
            Some(MeshKind::RotationZ)
        } else if mesh_key == self.mesh_keys.cube_x {
            Some(MeshKind::ScaleX)
        } else if mesh_key == self.mesh_keys.cube_y {
            Some(MeshKind::ScaleY)
        } else if mesh_key == self.mesh_keys.cube_z {
            Some(MeshKind::ScaleZ)
        } else {
            None
        }
    }

    pub fn start_pick(&self, mesh_key: MeshKey, x: i32, y: i32) -> bool {
        let mesh_kind = match self.get_mesh_kind(mesh_key) {
            Some(kind) => kind,
            None => return false,
        };

        tracing::info!("Start pick: {:?} at {},{}", mesh_kind, x, y);

        true
    }

    pub fn set_hidden(
        &self,
        renderer: &mut AwsmRenderer,
        translation_hidden: bool,
        rotation_hidden: bool,
        scale_hidden: bool,
    ) -> Result<()> {
        for mesh_key in self.translation_keys() {
            let mesh = renderer.meshes.get_mut(*mesh_key)?;
            mesh.hidden = translation_hidden;
        }

        for mesh_key in self.rotation_keys() {
            let mesh = renderer.meshes.get_mut(*mesh_key)?;
            mesh.hidden = rotation_hidden;
        }

        for mesh_key in self.scale_keys() {
            let mesh = renderer.meshes.get_mut(*mesh_key)?;
            mesh.hidden = scale_hidden;
        }

        Ok(())
    }

    fn translation_keys(&self) -> impl Iterator<Item = &MeshKey> {
        [
            &self.mesh_keys.arrow_x,
            &self.mesh_keys.arrow_y,
            &self.mesh_keys.arrow_z,
        ]
        .into_iter()
    }

    fn rotation_keys(&self) -> impl Iterator<Item = &MeshKey> {
        [
            &self.mesh_keys.ring_x,
            &self.mesh_keys.ring_y,
            &self.mesh_keys.ring_z,
        ]
        .into_iter()
    }

    fn scale_keys(&self) -> impl Iterator<Item = &MeshKey> {
        [
            &self.mesh_keys.cube_x,
            &self.mesh_keys.cube_y,
            &self.mesh_keys.cube_z,
        ]
        .into_iter()
    }
}

impl TransformControllerMeshKeys {
    pub fn new(lookups: &GltfKeyLookups) -> Result<Self> {
        let get_mesh_key = |node_name: &str| -> Result<MeshKey> {
            lookups
                .meshes_for_node_iter(node_name)
                .next()
                .cloned()
                .context(format!("No mesh for node '{}'", node_name))
        };

        Ok(Self {
            cube_x: get_mesh_key("Cube_X")?,
            cube_y: get_mesh_key("Cube_Y")?,
            cube_z: get_mesh_key("Cube_Z")?,
            ring_x: get_mesh_key("Ring_X")?,
            ring_y: get_mesh_key("Ring_Y")?,
            ring_z: get_mesh_key("Ring_Z")?,
            arrow_x: get_mesh_key("Arrow_X")?,
            arrow_y: get_mesh_key("Arrow_Y")?,
            arrow_z: get_mesh_key("Arrow_Z")?,
        })
    }
}

impl TransformControllerTransformKeys {
    pub fn new(lookups: &GltfKeyLookups) -> Result<Self> {
        let get_transform_key = |node_name: &str| -> Result<TransformKey> {
            lookups
                .node_transforms
                .get(node_name)
                .cloned()
                .context(format!("No transform for node '{}'", node_name))
        };

        Ok(Self {
            root: get_transform_key("GizmoRoot")?,
        })
    }
}
