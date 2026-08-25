use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::sync::Arc;

/// 服务定位器：内核不依赖 storage 等具体设施，由宿主在组装时注入，
/// 扩展通过类型在运行期取回（依赖倒置）。
#[derive(Default)]
pub struct Services {
    items: BTreeMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Services {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T>(&mut self, service: Arc<T>)
    where
        T: Any + Send + Sync,
    {
        self.items.insert(TypeId::of::<T>(), service);
    }

    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync,
    {
        self.items
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|any| any.downcast::<T>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_recovers_by_type() {
        let mut services = Services::new();
        services.insert(Arc::new(42_u32));
        let value = services.get::<u32>().unwrap();
        assert_eq!(*value, 42);
        assert!(services.get::<u64>().is_none());
    }
}
