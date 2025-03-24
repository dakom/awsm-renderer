use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwsmGltfError {
    #[error("Error loading glTF file")]
    Load,
    #[error("No scene at index {0}")]
    InvalidScene(usize),
    #[error("No default scene (or explicitly provided scene)")]
    NoDefaultScene,
}
