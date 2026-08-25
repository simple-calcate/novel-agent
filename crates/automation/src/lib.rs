//! 自动化：打字停笔等信号、工作流匹配、持久化操作队列状态机。
//! 队列真正执行在 `novel-extensions` 的 `queue.tick` 工具里。

pub mod queue;
pub mod signals;
pub mod workflow;

pub use queue::*;
pub use signals::*;
pub use workflow::*;
