use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwsmTransformError {}

pub type Result<T> = std::result::Result<T, AwsmTransformError>;
