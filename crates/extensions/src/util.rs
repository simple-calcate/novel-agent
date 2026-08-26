use novel_kernel::{Kernel, KernelError};
use novel_storage::{StorageError, StorageHandle};
use std::sync::Arc;

/// 内置扩展统一从内核取回单写者句柄。
pub fn storage(kernel: &Kernel) -> Result<Arc<StorageHandle>, KernelError> {
    kernel.service::<StorageHandle>()
}

/// 在单写者上执行只读或写入闭包，并把存储层错误映射为内核错误。
pub fn with_repository<T>(
    kernel: &Kernel,
    action: impl FnOnce(&mut novel_storage::Repository) -> Result<T, StorageError>,
) -> Result<T, KernelError> {
    let handle = storage(kernel)?;
    handle
        .execute(action)
        .map_err(|error| KernelError::Storage(error.to_string()))
}

/// 可变访问的别名（与 [`with_repository`] 相同：句柄始终给出 `&mut Repository`）。
pub fn with_repository_mut<T>(
    kernel: &Kernel,
    action: impl FnOnce(&mut novel_storage::Repository) -> Result<T, StorageError>,
) -> Result<T, KernelError> {
    with_repository(kernel, action)
}
