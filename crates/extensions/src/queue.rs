//! 队列执行扩展：`queue.tick` 工具原子领取一个到期任务，分发给
//! 内核工具注册表里同名工具执行，并按结果推进状态机
//! （succeeded / 指数退避重试 / 死信）。

use async_trait::async_trait;
use chrono::{Duration, Utc};
use novel_kernel::{Extension, KernelBuilder, KernelError, Tool, ToolContext};
use serde_json::{json, Value};
use std::sync::Arc;

/// 队列执行策略：宿主可注册 `Arc<QueuePolicy>` 服务覆盖默认值
/// （例如测试里把退避归零）。
#[derive(Debug, Clone)]
pub struct QueuePolicy {
    /// 卡在 running 超过该时长的任务视为进程崩溃遗留，重新回到 pending。
    pub stale_running_after: Duration,
    /// 重试退避基数：第 n 次失败后等待 base * 2^(n-1)。
    pub backoff_base: Duration,
}

impl Default for QueuePolicy {
    fn default() -> Self {
        Self {
            stale_running_after: Duration::minutes(10),
            backoff_base: Duration::seconds(5),
        }
    }
}

pub struct QueueTickTool;

#[async_trait]
impl Tool for QueueTickTool {
    fn id(&self) -> &str {
        "queue.tick"
    }

    fn summary(&self) -> &str {
        "领取并执行一个到期队列任务"
    }

    async fn execute(&self, _input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let kernel = ctx.kernel();
        let policy: QueuePolicy = kernel
            .service::<QueuePolicy>()
            .map(Arc::unwrap_or_clone)
            .unwrap_or_default();

        let job = crate::util::with_repository(kernel, |repository| {
            repository.claim_next_job(Utc::now(), policy.stale_running_after)
        })?;

        let Some(job) = job else {
            return Ok(json!({"executed": false}));
        };

        // 任务操作名即工具名；找不到工具按失败处理进入重试/死信。
        let outcome = kernel.call_tool(&job.operation, job.payload.clone()).await;
        let now = Utc::now();

        match outcome {
            Ok(output) => {
                crate::util::with_repository(kernel, |repository| {
                    repository.complete_job(&job.id, &output, now)
                })?;
                Ok(json!({
                    "executed": true,
                    "jobId": job.id.to_string(),
                    "operation": job.operation,
                    "success": true,
                    "attempts": job.attempts,
                    "status": "succeeded",
                }))
            }
            Err(error) => {
                let backoff = retry_backoff(job.attempts, policy.backoff_base);
                let dead = crate::util::with_repository(kernel, |repository| {
                    repository.fail_job(&job, &error.to_string(), backoff, now)
                })?;
                Ok(json!({
                    "executed": true,
                    "jobId": job.id.to_string(),
                    "operation": job.operation,
                    "success": false,
                    "attempts": job.attempts,
                    "status": if dead { "deadLetter" } else { "pending" },
                    "retryInMs": if dead { Value::Null } else { json!(backoff.num_milliseconds()) },
                    "error": error.to_string(),
                }))
            }
        }
    }
}

/// 指数退避：base、2*base、4*base……封顶 64 倍防止溢出。
fn retry_backoff(attempts: u32, base: Duration) -> Duration {
    let factor = 1u64 << (attempts - 1).min(6);
    base * (factor as i32)
}

/// 队列扩展：注册队列驱动工具。
pub struct QueueExtension;

impl Extension for QueueExtension {
    fn id(&self) -> &str {
        "builtin.queue"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(QueueTickTool);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially() {
        let base = Duration::seconds(5);
        assert_eq!(retry_backoff(1, base), Duration::seconds(5));
        assert_eq!(retry_backoff(2, base), Duration::seconds(10));
        assert_eq!(retry_backoff(3, base), Duration::seconds(20));
        // 封顶防止溢出
        assert_eq!(retry_backoff(30, base), Duration::seconds(5 * 64));
        assert_eq!(retry_backoff(2, Duration::zero()), Duration::zero());
    }
}
