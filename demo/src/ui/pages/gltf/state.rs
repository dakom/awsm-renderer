use crate::{prelude::*, config::CONFIG, camera::CameraKind};
use std::{cell::{RefCell, Cell}, collections::HashSet};
use dominator_helpers::futures::AsyncLoader;

pub struct GltfPage {
    pub world: Mutable<Option<Rc<RefCell<World>>>>,
    pub gltf: Mutable<Option<GltfId>>,
    pub camera: Mutable<Option<CameraKind>>,
    pub loader: AsyncLoader,
    pub pointer: Cell<Option<(i32,i32)>>,
    pub keys_down: RefCell<HashSet<String>>,
    _renderer: RefCell<Option<Rc<RefCell<AwsmRenderer>>>>,
}

impl GltfPage {
    pub fn new(id: Option<GltfId>) -> Rc<Self> {
        Rc::new(Self {
            world: Mutable::new(None),
            gltf: Mutable::new(id),
            camera: Mutable::new(None),
            _renderer: RefCell::new(None),
            pointer: Cell::new(None),
            loader: AsyncLoader::new(),
            keys_down: RefCell::new(HashSet::new()),

        })
    }

    pub fn set_renderer(&self, renderer: Rc<RefCell<AwsmRenderer>>) {
        *self._renderer.borrow_mut() = Some(renderer);
    }

    pub fn world_cell(&self) -> Rc<RefCell<World>> {
        self.world.get_cloned().unwrap_ext()
    }

    pub fn renderer_cell(&self) -> Rc<RefCell<AwsmRenderer>> {
        self._renderer.borrow().as_ref().unwrap_ext().clone()
    }
}