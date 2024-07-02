use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}
