use crate::prelude::*;

use super::scene::{camera::CameraId, AppScene};

#[derive(Clone, Default)]
pub struct AppContext {
    pub camera_id: Mutable<CameraId>,
    pub scene: Mutable<Option<Arc<AppScene>>>,
}
