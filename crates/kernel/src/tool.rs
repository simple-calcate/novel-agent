use crate::{Kernel, KernelError};
use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 工具：内核可分发的最小能力单元。队列操作、上下文装配、插件调用
/// 都是工具；同名注册可覆盖内置实现。
///
/// 契约：`id` 全局唯一；`summary` / `input_schema` 供 `kernel_tools` 自描述；
/// `execute` 通过 `ToolContext` 访问仓库与其它工具，禁止直接依赖宿主。
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名，例如 `document.save`、`queue.tick`。
    fn id(&self) -> &str;

    /// 一句话说明，会出现在 `kernel.call_tool` 的自描述列表里。
    fn summary(&self) -> &str {
        ""
    }

    /// JSON Schema（或等价 JSON）描述入参。缺省为 `null`。
    fn input_schema(&self) -> Value {
        Value::Null
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError>;
}

/// 工具执行上下文：工具可以通过内核访问其他工具、Provider 和注入的服务。
pub struct ToolContext<'a> {
    kernel: &'a Kernel,
}
impl<'a> ToolContext<'a> {
    pub fn new(kernel: &'a Kernel) -> Self {
        Self { kernel }
    }

    pub fn kernel(&self) -> &Kernel {
        self.kernel
    }

    /// 在工具内调用另一个工具。
    pub async fn call_tool(&self, id: &str, input: Value) -> Result<Value, KernelError> {
        self.kernel.call_tool(id, input).await
    }

    pub fn service<T>(&self) -> Result<Arc<T>, KernelError>
    where
        T: Any + Send + Sync,
    {
        self.kernel.service::<T>()
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册（或覆盖）工具，返回被覆盖的工具。
    pub fn register(&mut self, tool: impl Tool + 'static) -> Option<Arc<dyn Tool>> {
        self.tools.insert(tool.id().to_owned(), Arc::new(tool))
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(id).cloned()
    }

    pub fn ids(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }

    pub fn describe(&self) -> Vec<ToolDescriptor> {
        self.tools
            .values()
            .map(|tool| ToolDescriptor {
                id: tool.id().to_owned(),
                summary: tool.summary().to_owned(),
                input_schema: tool.input_schema(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub id: String,
    pub summary: String,
    pub input_schema: Value,
}

/// 允许以 `Box<dyn Tool>` 直接注册（宿主在运行期构造工具列表时常用）。
#[async_trait]
impl Tool for Box<dyn Tool> {
    fn id(&self) -> &str {
        (**self).id()
    }

    fn summary(&self) -> &str {
        (**self).summary()
    }

    fn input_schema(&self) -> Value {
        (**self).input_schema()
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        (**self).execute(input, ctx).await
    }
}
