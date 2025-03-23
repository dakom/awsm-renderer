use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwsmGltfError {
    #[error("Error loading glTF file")]
    Load,
}