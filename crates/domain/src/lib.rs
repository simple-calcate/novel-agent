pub mod actor;
pub mod content;
pub mod events;
pub mod ids;
pub mod jobs;
pub mod patches;
pub mod plugins;
pub mod story;
pub mod work;

pub use actor::*;
pub use content::*;
pub use events::*;
pub use ids::*;
pub use jobs::*;
pub use patches::*;
pub use plugins::*;
pub use story::*;
pub use work::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("entity not found: {0}")]
    NotFound(String),
    #[error("revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}
