use std::sync::Arc;

use super::data::GltfData;

#[derive(Default)]
pub(crate) struct GltfCache {
    pub raw_datas: Vec<Arc<GltfData>>,
}
