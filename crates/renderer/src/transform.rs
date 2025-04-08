use std::collections::HashSet;
use thiserror::Error;

use slotmap::{new_key_type, SecondaryMap, SlotMap};

pub struct Transforms {
    locals: SlotMap<TransformKey, Transform>,
    world_matrices: SecondaryMap<TransformKey, glam::Mat4>,
    children: SecondaryMap<TransformKey, Vec<TransformKey>>,
    parents: SecondaryMap<TransformKey, TransformKey>,
    dirties: HashSet<TransformKey>,
    root_node: TransformKey,
}

new_key_type! {
    pub struct TransformKey;
}

impl Default for Transforms {
    fn default() -> Self {
        let mut locals = SlotMap::with_key();
        let mut world_matrices = SecondaryMap::new();
        let mut children = SecondaryMap::new();

        let root_node = locals.insert(Transform::default());
        world_matrices.insert(root_node, glam::Mat4::IDENTITY);
        children.insert(root_node, Vec::new());

        Self {
            locals,
            world_matrices,
            children,
            parents: SecondaryMap::new(),
            dirties: HashSet::new(),
            root_node,
        }
    }
}

impl Transforms {
    pub fn insert(&mut self, transform: Transform) -> TransformKey {
        let world_matrix = transform.to_matrix();

        let key = self.locals.insert(transform);

        self.world_matrices.insert(key, world_matrix);
        self.children.insert(key, Vec::new());
        self.dirties.insert(key);

        self.set_parent(key, None);

        key
    }

    pub fn remove(&mut self, key: TransformKey) {
        if key == self.root_node {
            return;
        }

        // happens separately so that we can remove the node from the parent's children list
        self.unset_parent(key);

        self.locals.remove(key);
        self.world_matrices.remove(key);
        self.children.remove(key);
        self.dirties.remove(&key);
    }

    // This is the only way to update the matrices
    // world transforms are updated by walking the hierarchy
    pub fn update_local(&mut self, key: TransformKey, transform: Transform) -> Result<()> {
        match self.locals.get_mut(key) {
            Some(existing) => {
                *existing = transform;
                self.dirties.insert(key);
                Ok(())
            }
            None => Err(AwsmTransformError::LocalNotFound(key)),
        }
    }

    // if parent is None then the parent is actually the root node
    pub fn set_parent(&mut self, child: TransformKey, parent: Option<TransformKey>) {
        if child == self.root_node {
            return;
        }

        let parent = parent.unwrap_or(self.root_node);

        if let Some(existing_parent) = self.parents.get(child) {
            if *existing_parent == parent {
                return;
            } else {
                self.unset_parent(child);
            }
        }

        // safe because all transforms have children vec when created
        self.children.get_mut(parent).unwrap().push(child);

        self.parents.insert(child, parent);
    }

    pub fn get_local(&self, key: TransformKey) -> Result<&Transform> {
        self.locals
            .get(key)
            .ok_or(AwsmTransformError::LocalNotFound(key))
    }

    pub fn get_world(&self, key: TransformKey) -> Result<&glam::Mat4> {
        self.world_matrices
            .get(key)
            .ok_or(AwsmTransformError::WorldNotFound(key))
    }

    // This is the only way to update the world matrices
    // See: https://gameprogrammingpatterns.com/dirty-flag.html
    // the overall idea is we walk the tree and skip over nodes that are not dirty
    // whenever we encounter a dirty node, we must also mark all of its children dirty
    // finally, for each dirty node, its world transform is its parent's world transform
    // multiplied by its local transform
    // or in other words, it's the local transform, offset by its parent in world space
    pub fn propogate(
        &mut self,
        // if None then will start from the root node
        _key: Option<TransformKey>,
    ) -> Result<()> {
        // TODO - implement!
        // see example at https://github.com/dakom/shipyard-scenegraph/blob/babc8de4af51408ec36a575eeb28f3fdb7d0ca57/crate/src/systems.rs#L94

        Ok(())
    }

    // internal-only function - leaves the node dangling
    // after this call, the node should either be immediately removed or reparented
    fn unset_parent(&mut self, child: TransformKey) {
        if let Some(parent) = self.parents.remove(child) {
            if let Some(children) = self.children.get_mut(parent) {
                children.retain(|&c| c != child);
            }
        }
    }
}

pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    const IDENTITY: Self = Self {
        translation: glam::Vec3::ZERO,
        rotation: glam::Quat::IDENTITY,
        scale: glam::Vec3::ONE,
    };

    pub fn with_translation(mut self, translation: glam::Vec3) -> Self {
        self.translation = translation;
        self
    }
    pub fn with_rotation(mut self, rotation: glam::Quat) -> Self {
        self.rotation = rotation;
        self
    }
    pub fn with_scale(mut self, scale: glam::Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn from_matrix(matrix: glam::Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

pub type Result<T> = std::result::Result<T, AwsmTransformError>;

#[derive(Error, Debug)]
pub enum AwsmTransformError {
    #[error("[transform] local transform does not exist {0:?}")]
    LocalNotFound(TransformKey),

    #[error("[transform] world transform does not exist {0:?}")]
    WorldNotFound(TransformKey),
}
