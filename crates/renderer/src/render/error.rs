use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwsmRenderError {
}

pub type Result<T> = std::result::Result<T, AwsmRenderError>;