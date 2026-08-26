//! 内置扩展集：把模型提供方、工作流引擎、队列、核心工具、上下文浮带、
//! 上下文装配与插件宿主全部注册进内核。宿主也可以逐个挑选或覆盖。
//! 对外契约见仓库 `docs/interfaces.md`。

pub mod blocks;
pub mod context;
pub mod core_tools;
pub mod hints;
pub mod plugins;
pub mod providers;
pub mod queue;
pub mod secrets;
pub mod util;
pub mod workflow;
pub mod workspace;

pub use blocks::{
    build_training_examples, serialize_examples, BlockEditTool, BlockSaveTool, BlocksExtension,
    TrainingExample, TrainingExportTool,
};

pub use context::{ContextAssembleTool, ContextAssemblyExtension};
pub use core_tools::{
    AgentContinuationTool, AgentRunTool, ContinuityCheckTool, CoreToolsExtension, CreateBackupTool,
    DocumentSaveTool, RebuildIndexTool,
};
pub use hints::{ContextHintsTool, HintsExtension};
pub use plugins::{PluginHostExtension, PluginInstallTool, PluginOperationTool};
pub use providers::{
    resolve_provider_name, EchoProvider, OpenAICompatibleProvider, ProvidersExtension, SseParser,
};
pub use queue::{QueueExtension, QueuePolicy, QueueTickTool};
pub use secrets::{SecretVault, MODEL_API_KEY};
pub use workflow::{WorkflowEngineExtension, WorkflowEngineSubscriber};
pub use workspace::{
    load_provider_config_from_kernel, read_chapter_body, ModelConfigView, Workspace, WorkspaceError,
};

use novel_kernel::{Extension, KernelBuilder, KernelError};

/// 一站式内置扩展：等价于逐个注册全部内置扩展。
pub struct BuiltinsExtension;

impl Extension for BuiltinsExtension {
    fn id(&self) -> &str {
        "builtin"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_extension(ProvidersExtension)?;
        builder.register_extension(CoreToolsExtension)?;
        builder.register_extension(BlocksExtension)?;
        builder.register_extension(WorkflowEngineExtension)?;
        builder.register_extension(QueueExtension)?;
        builder.register_extension(HintsExtension)?;
        builder.register_extension(ContextAssemblyExtension)?;
        builder.register_extension(PluginHostExtension)?;
        Ok(())
    }
}
