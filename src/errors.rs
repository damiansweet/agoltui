use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("AGOL Lib error: {0}")]
    Agol(#[from] agol::error::ArcGISLibError),
    #[error("Ratatui error: {0}")]
    Ratatui(#[from] std::io::Error),
    #[error("AGOL Data Fetch error: {0}")]
    FetchAll(#[from] std::boxed::Box<dyn std::error::Error + Send + Sync>),
}
