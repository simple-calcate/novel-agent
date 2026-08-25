use crate::agent::AgentBudget;
use std::time::{Duration, Instant};

/// 预算守卫：在内核层强制时间与输出 token 上限，不依赖 Provider 自觉。
pub struct BudgetGuard {
    deadline: Instant,
    max_output_tokens: u32,
    used_output_tokens: u32,
}

impl BudgetGuard {
    pub fn new(budget: &AgentBudget) -> Self {
        // max_seconds 至少给 1 秒，避免 0 值让所有调用立即超时。
        let seconds = budget.max_seconds.max(1);
        Self {
            deadline: Instant::now() + Duration::from_secs(seconds),
            max_output_tokens: budget.max_tokens.max(1),
            used_output_tokens: 0,
        }
    }

    /// 剩余时间；耗尽后返回 None。
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline.checked_duration_since(Instant::now())
    }

    /// 记录一段输出 token，返回 false 表示已达到输出上限。
    pub fn record_output(&mut self, tokens: u32) -> bool {
        self.used_output_tokens += tokens;
        self.used_output_tokens < self.max_output_tokens
    }

    pub fn is_over_tokens(&self) -> bool {
        self.used_output_tokens >= self.max_output_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(max_tokens: u32, max_seconds: u64) -> AgentBudget {
        AgentBudget {
            max_rounds: 1,
            max_tokens,
            max_cost_micros: 0,
            max_seconds,
        }
    }

    #[test]
    fn enforces_token_cap() {
        let mut guard = BudgetGuard::new(&budget(10, 60));
        assert!(guard.record_output(5));
        assert!(!guard.record_output(5));
        assert!(guard.is_over_tokens());
    }

    #[test]
    fn remaining_depletes_with_time() {
        let guard = BudgetGuard::new(&budget(100, 60));
        let remaining = guard.remaining().expect("应有剩余时间");
        assert!(remaining <= Duration::from_secs(60));
    }
}
