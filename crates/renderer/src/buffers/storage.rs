use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

pub struct StorageBuffers {
    buffers: SlotMap<StorageBufferKey, web_sys::GpuBuffer>,
}

impl StorageBuffers {
    pub fn new() -> Self {
        Self {
            buffers: SlotMap::with_key(),
        }
    }

    pub fn get(&self, key: StorageBufferKey) -> Result<&web_sys::GpuBuffer> {
        self.buffers
            .get(key)
            .ok_or_else(|| AwsmStorageError::KeyNotFound(key))
    }

    pub fn insert(&mut self, buffer: web_sys::GpuBuffer) -> StorageBufferKey {
        self.buffers.insert(buffer)
    }

    pub fn remove(&mut self, key: StorageBufferKey) -> Option<web_sys::GpuBuffer> {
        self.buffers.remove(key)
    }
}

new_key_type! {
    pub struct StorageBufferKey;
}

type Result<T> = std::result::Result<T, AwsmStorageError>;
#[derive(Error, Debug)]
pub enum AwsmStorageError {
    #[error("[storage] key not found: {0:?}")]
    KeyNotFound(StorageBufferKey),
}
