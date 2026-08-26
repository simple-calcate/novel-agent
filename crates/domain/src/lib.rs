//! 领域模型：作品层级、正史、事件、任务与补丁的共享类型。
//!
//! 这一层没有 IO。仓储、内核和 UI 都通过这里的类型交换数据。
//! 层级约定：`Project`（作品）→ `Book`（书）→ 可选 `Volume`（卷）→ `Chapter`（章）→ 可选 `Scene`（场）。
//! 接口说明见仓库根目录 `docs/interfaces.md`。

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
