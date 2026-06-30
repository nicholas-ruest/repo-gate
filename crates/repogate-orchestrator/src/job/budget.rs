//! Token-budget tracking with thread-safe accumulation (ADR-013).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use repogate_core::TokenBudget;

/// Result of recording usage against the budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    Ok,
    Warning,
    Exceeded,
}

/// Thread-safe cumulative token tracker for a job.
///
/// Cache-read tokens are billed at 10% of their nominal count.
#[derive(Clone)]
pub struct BudgetTracker {
    budget: TokenBudget,
    used: Arc<AtomicU64>,
}

impl BudgetTracker {
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            budget,
            used: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record token usage and report the resulting [`BudgetStatus`].
    pub fn record_usage(&self, input: u64, output: u64, cache_read: u64) -> BudgetStatus {
        let cost = input + output + (cache_read / 10);
        let new_total = self.used.fetch_add(cost, Ordering::SeqCst) + cost;

        if new_total >= self.budget.total_limit {
            BudgetStatus::Exceeded
        } else if new_total as f32 >= self.budget.total_limit as f32 * self.budget.warn_threshold {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Ok
        }
    }

    /// Total tokens recorded so far.
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::SeqCst)
    }

    /// True once the recorded usage meets or exceeds the total budget.
    pub fn is_exceeded(&self) -> bool {
        self.used.load(Ordering::SeqCst) >= self.budget.total_limit
    }

    /// Tokens remaining before the total budget is reached.
    pub fn remaining(&self) -> u64 {
        self.budget.total_limit.saturating_sub(self.used())
    }

    /// Rough dollar-cost estimate for the tokens recorded so far.
    pub fn estimated_cost_usd(&self) -> f64 {
        // Simplified: blended ~$3 per 1M tokens. Per-model pricing lands with
        // the cost-estimation work in P13/P14.
        (self.used() as f64) * 3.0 / 1_000_000.0
    }
}
