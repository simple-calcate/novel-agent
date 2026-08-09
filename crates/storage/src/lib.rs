pub mod export;
pub mod migrations;
pub mod repository;

pub use export::*;
pub use migrations::run_migrations;
pub use repository::*;

use novel_domain::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("migration failed: {0}")]
    Migration(String),
}
