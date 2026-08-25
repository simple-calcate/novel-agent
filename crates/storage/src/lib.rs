//! SQLite 持久化：迁移、单写者仓库、导出。
//!
//! 宿主把 `Arc<Mutex<Repository>>` 注入内核服务表；扩展按类型取回。
//! 公开方法即存储层接口，见 `docs/interfaces.md` 的 Storage 一节。

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
