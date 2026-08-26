//! 单写者句柄：序列化所有 SQLite 访问，并拒绝同线程重入。
//!
//! ADR 0002 要求写路径经过唯一 writer。这里不用「自觉先放锁再 dispatch」，
//! 而是：同线程嵌套 `with` 立即返回 [`StorageError::Reentrancy`]，避免
//! `kernel.dispatch` 的订阅者再次抢锁导致死锁。

use crate::{Repository, StorageError};
use std::cell::Cell;
use std::path::Path;
use std::sync::Mutex;

thread_local! {
    static WRITER_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// 注入内核服务表的单写者。调用方只通过 [`StorageHandle::with`] 访问仓库。
pub struct StorageHandle {
    inner: Mutex<Repository>,
}

struct WriterGuard;

impl Drop for WriterGuard {
    fn drop(&mut self) {
        WRITER_DEPTH.with(|cell| cell.set(0));
    }
}

fn enter_writer() -> Result<WriterGuard, StorageError> {
    WRITER_DEPTH.with(|cell| {
        if cell.get() > 0 {
            Err(StorageError::Reentrancy)
        } else {
            cell.set(1);
            Ok(WriterGuard)
        }
    })
}

impl StorageHandle {
    pub fn new(repository: Repository) -> Self {
        Self {
            inner: Mutex::new(repository),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self::new(Repository::open(path)?))
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        Ok(Self::new(Repository::open_in_memory()?))
    }

    /// 在唯一 writer 上执行 `action`。
    ///
    /// 同线程嵌套调用返回 [`StorageError::Reentrancy`]，而不是卡住。
    /// 跨线程调用会排队等待当前 writer 结束。
    pub fn with<T, F>(&self, action: F) -> Result<T, StorageError>
    where
        F: FnOnce(&mut Repository) -> T,
    {
        let _guard = enter_writer()?;
        let mut repository = self
            .inner
            .lock()
            .map_err(|_| StorageError::Unavailable("storage mutex poisoned".into()))?;
        Ok(action(&mut repository))
    }

    /// [`Self::with`] 的结果版：闭包自身也可以返回 [`StorageError`]。
    pub fn execute<T, F>(&self, action: F) -> Result<T, StorageError>
    where
        F: FnOnce(&mut Repository) -> Result<T, StorageError>,
    {
        self.with(action)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_with_on_same_thread_is_reentrancy() {
        let handle = StorageHandle::open_in_memory().unwrap();
        let nested = handle.with(|_repo| handle.with(|_inner| ())).unwrap();
        assert!(matches!(nested, Err(StorageError::Reentrancy)));
    }

    #[test]
    fn sequential_with_calls_succeed() {
        let handle = StorageHandle::open_in_memory().unwrap();
        let first = handle.execute(|repo| repo.create_project("甲")).unwrap();
        let second = handle.execute(|repo| repo.create_project("乙")).unwrap();
        assert_ne!(first.id, second.id);
        let titles = handle.execute(|repo| repo.list_projects()).unwrap();
        assert_eq!(titles.len(), 2);
    }
}
