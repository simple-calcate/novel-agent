use novel_kernel::{Kernel, KernelError};
use novel_storage::Repository;
use std::sync::{Arc, Mutex};

/// 内置扩展统一从内核取回注入的 SQLite 仓库。
pub fn repository(kernel: &Kernel) -> Result<Arc<Mutex<Repository>>, KernelError> {
    kernel.service::<Mutex<Repository>>()
}

/// 锁仓库并把存储层错误映射为内核错误。
pub fn with_repository<T>(
    kernel: &Kernel,
    action: impl FnOnce(&Repository) -> Result<T, novel_storage::StorageError>,
) -> Result<T, KernelError> {
    let repository = repository(kernel)?;
    let guard = repository
        .lock()
        .map_err(|_| KernelError::Storage("repository mutex poisoned".into()))?;
    action(&guard).map_err(|error| KernelError::Storage(error.to_string()))
}

/// 可变锁仓库。
pub fn with_repository_mut<T>(
    kernel: &Kernel,
    action: impl FnOnce(&mut Repository) -> Result<T, novel_storage::StorageError>,
) -> Result<T, KernelError> {
    let repository = repository(kernel)?;
    let mut guard = repository
        .lock()
        .map_err(|_| KernelError::Storage("repository mutex poisoned".into()))?;
    action(&mut guard).map_err(|error| KernelError::Storage(error.to_string()))
}
