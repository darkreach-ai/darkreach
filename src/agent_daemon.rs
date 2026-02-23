//! # Agent Daemon — Task Runner & Budget Enforcement
//!
//! Extracted from the inline `tokio::spawn` closure in `dashboard/mod.rs`.
//! Manages the agent execution lifecycle: polling for completed agents,
//! enforcing global budgets, claiming new tasks, and reclaiming orphans.
//!
//! ## Two-Frequency Loop
//!
//! The daemon uses `tokio::select!` on two interval timers:
//! - **Fast tick** (5s): polls for completed agent subprocesses via
//!   [`AgentManager::poll_completed`](crate::agent::AgentManager::poll_completed),
//!   then processes each result (DB update, budget tracking, parent cascading).
//! - **Slow tick** (10s): enforces the global budget, reclaims stale tasks,
//!   and claims + spawns new agent subprocesses.
//!
//! ## Orphan Recovery
//!
//! At startup and every 5 minutes, the daemon calls
//! [`Database::reclaim_stale_agent_tasks`] to reset `in_progress` tasks whose
//! `started_at` exceeds the configured staleness threshold. This handles
//! coordinator restarts where running agents were lost.

use crate::agent::{self, AgentStatus, CompletedAgent};
use crate::dashboard::{gethostname, lock_or_recover, AppState};
use crate::db::AgentScheduleRow;
use crate::prom_metrics::AgentStatusLabel;
use std::sync::Arc;
use tracing::{info, warn};

/// Configuration for the agent daemon background loop.
pub struct AgentDaemonConfig {
    /// Interval in seconds for polling completed agents (fast tick).
    pub poll_interval_secs: u64,
    /// Interval in seconds for claiming new tasks (slow tick).
    pub claim_interval_secs: u64,
    /// Whether the daemon is enabled.
    pub enabled: bool,
    /// Threshold in seconds before an in-progress task is considered stale.
    pub stale_task_secs: i64,
    /// Identifier for this daemon instance (e.g., "coordinator@hostname").
    pub agent_name: String,
}

impl AgentDaemonConfig {
    /// Build configuration from environment variables.
    ///
    /// - `DARKREACH_AGENT_DAEMON`: "false" disables the daemon (default: enabled)
    /// - `DARKREACH_AGENT_POLL_SECS`: fast tick interval (default: 5)
    /// - `DARKREACH_AGENT_CLAIM_SECS`: slow tick interval (default: 10)
    /// - `DARKREACH_AGENT_STALE_SECS`: orphan reclamation threshold (default: 3600)
    pub fn from_env() -> Self {
        let enabled = std::env::var("DARKREACH_AGENT_DAEMON")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);
        let poll_interval_secs = std::env::var("DARKREACH_AGENT_POLL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let claim_interval_secs = std::env::var("DARKREACH_AGENT_CLAIM_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        let stale_task_secs = std::env::var("DARKREACH_AGENT_STALE_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600);
        let agent_name = format!("coordinator@{}", gethostname());

        Self {
            poll_interval_secs,
            claim_interval_secs,
            enabled,
            stale_task_secs,
            agent_name,
        }
    }
}

/// Process a completed agent: update the DB, track budget spending,
/// cascade to parent tasks.
pub async fn handle_completed_agent(completed: CompletedAgent, state: &AppState) {
    let status_str = status_to_db_str(&completed.status);
    let reason = match &completed.status {
        AgentStatus::Failed { reason } => Some(reason.clone()),
        AgentStatus::TimedOut => Some("Timed out".to_string()),
        _ => None,
    };
    let (result_json, tokens, cost) = match completed.result {
        Some(ref r) => (
            Some(serde_json::json!({"text": r.result_text})),
            r.tokens_used,
            r.cost_usd,
        ),
        None => (
            reason.as_ref().map(|r| serde_json::json!({"error": r})),
            0,
            0.0,
        ),
    };

    // 1. Mark task completed in DB
    if let Err(e) = state
        .db
        .complete_agent_task(
            completed.task_id,
            status_str,
            result_json.as_ref(),
            tokens,
            cost,
        )
        .await
    {
        warn!(task_id = completed.task_id, error = %e, "agent: failed to complete task");
    }

    // 2. Insert completion event
    let summary = match &completed.status {
        AgentStatus::Completed => "Task completed".to_string(),
        AgentStatus::Failed { reason } => format!("Task failed: {}", reason),
        AgentStatus::TimedOut => "Task timed out".to_string(),
        AgentStatus::Cancelled => "Task cancelled".to_string(),
        _ => "Task finished".to_string(),
    };
    let _ = state
        .db
        .insert_agent_event(
            Some(completed.task_id),
            status_str,
            Some("system"),
            &summary,
            None,
        )
        .await;

    // 3. Track budget spending
    if tokens > 0 || cost > 0.0 {
        let _ = state.db.update_agent_budget_spending(tokens, cost).await;
    }

    // 3a. Prometheus metrics: task completion, cost, tokens, duration
    let status_label = AgentStatusLabel {
        status: status_str.to_string(),
    };
    state
        .prom_metrics
        .agent_tasks_completed
        .get_or_create(&status_label)
        .inc();
    if cost > 0.0 {
        state.prom_metrics.agent_cost_usd.inc_by(cost);
    }
    if tokens > 0 {
        state
            .prom_metrics
            .agent_tokens_used
            .inc_by(tokens as u64);
    }
    // Compute task duration from started_at → completed_at (set by complete_agent_task)
    if let Ok(Some(finished_task)) = state.db.get_agent_task(completed.task_id).await {
        if let (Some(started), Some(ended)) =
            (finished_task.started_at, finished_task.completed_at)
        {
            let duration_secs = (ended - started).num_seconds().max(0) as f64;
            state
                .prom_metrics
                .agent_task_duration
                .get_or_create(&status_label)
                .observe(duration_secs);
        }
    }

    info!(
        task_id = completed.task_id,
        status = status_str,
        tokens,
        cost,
        "agent task finished"
    );

    // 4. Parent task cascading
    if let Ok(Some(completed_task)) = state.db.get_agent_task(completed.task_id).await {
        if let Some(parent_id) = completed_task.parent_task_id {
            // On child failure, cancel pending siblings if policy says "fail"
            if status_str == "failed" {
                if let Ok(Some(parent)) = state.db.get_agent_task(parent_id).await {
                    if parent.on_child_failure == "fail" {
                        let cancelled = state
                            .db
                            .cancel_pending_siblings(parent_id)
                            .await
                            .unwrap_or(0);
                        if cancelled > 0 {
                            info!(parent_id, cancelled, "agent: cancelled pending siblings");
                        }
                    }
                }
            }

            // Try to auto-complete parent if all children are terminal
            if let Ok(Some(parent)) = state.db.try_complete_parent(parent_id).await {
                let event_type = if parent.status == "failed" {
                    "parent_failed"
                } else {
                    "parent_completed"
                };
                let _ = state
                    .db
                    .insert_agent_event(
                        Some(parent_id),
                        event_type,
                        None,
                        &format!("Parent task '{}' auto-{}", parent.title, parent.status),
                        None,
                    )
                    .await;
                info!(parent_id, status = %parent.status, "agent: parent task auto-completed");
            }
        }
    }
}

/// Enforce global budget: kill all running agents if over budget.
///
/// Returns `true` if budget is OK (spending allowed), `false` if over budget.
pub async fn enforce_global_budget(state: &AppState) -> bool {
    let budget_ok = state.db.check_agent_budget().await.unwrap_or(true);
    if !budget_ok {
        let killed = lock_or_recover(&state.agents).kill_all();
        for task_id in &killed {
            let _ = state
                .db
                .complete_agent_task(
                    *task_id,
                    "failed",
                    Some(&serde_json::json!({"error": "Global budget exceeded"})),
                    0,
                    0.0,
                )
                .await;
            let _ = state
                .db
                .insert_agent_event(
                    Some(*task_id),
                    "budget_exceeded",
                    Some("system"),
                    "Killed: global budget exceeded",
                    None,
                )
                .await;
        }
        if !killed.is_empty() {
            warn!(count = killed.len(), task_ids = ?killed, "agent: global budget exceeded, killed agents");
        }
    }
    budget_ok
}

/// Attempt to claim a pending task and spawn an agent subprocess.
///
/// Checks concurrency limits, claims the highest-priority pending task
/// atomically, auto-selects the model, assembles context, and spawns
/// the Claude CLI subprocess.
pub async fn try_claim_and_spawn(state: &AppState, agent_name: &str) {
    let active = lock_or_recover(&state.agents).active_count();
    if active >= agent::MAX_AGENTS {
        return;
    }

    let task = match state.db.claim_pending_agent_task(agent_name).await {
        Ok(Some(t)) => t,
        Ok(None) => return,
        Err(e) => {
            warn!(error = %e, "agent: failed to claim task");
            return;
        }
    };

    info!(task_id = task.id, title = %task.title, priority = task.priority, model = ?task.agent_model, "agent: claimed task");
    let _ = state
        .db
        .insert_agent_event(
            Some(task.id),
            "claimed",
            Some(agent_name),
            &format!("Task claimed by {}", agent_name),
            None,
        )
        .await;

    let role = if let Some(ref rn) = task.role_name {
        state.db.get_role_by_name(rn).await.ok().flatten()
    } else {
        None
    };

    // Auto-select model if not explicitly set
    let selected_model = agent::auto_select_model(&task, role.as_ref());
    let mut task = task;
    if task.agent_model.is_none() {
        let _ = state
            .db
            .update_agent_task_model(task.id, selected_model)
            .await;
        task.agent_model = Some(selected_model.to_string());
    }

    let context_prompts = agent::assemble_context(&task, &state.db, role.as_ref()).await;
    let db_clone = state.db.clone();
    let spawn_result = {
        lock_or_recover(&state.agents).spawn_agent(&task, db_clone, task.max_cost_usd, context_prompts)
    };

    match spawn_result {
        Ok(_info) => {}
        Err(e) => {
            state.prom_metrics.agent_spawn_failures.inc();
            warn!(task_id = task.id, error = %e, "agent: failed to spawn");
            let _ = state
                .db
                .complete_agent_task(
                    task.id,
                    "failed",
                    Some(&serde_json::json!({"error": e})),
                    0,
                    0.0,
                )
                .await;
            let _ = state
                .db
                .insert_agent_event(
                    Some(task.id),
                    "failed",
                    Some("system"),
                    &format!("Failed to spawn: {}", e),
                    None,
                )
                .await;
        }
    }
}

/// Evaluate all enabled cron schedules and fire matching ones.
///
/// Called every 60 seconds from the daemon's `tokio::select!` loop.
/// For each enabled schedule with `trigger_type = "cron"`, checks
/// whether the cron expression matches the current time (and hasn't
/// already fired this minute), then delegates to [`fire_schedule`].
async fn evaluate_cron_schedules(state: &AppState) -> Result<(), String> {
    let schedules = state
        .db
        .get_enabled_schedules()
        .await
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now();
    for sched in schedules {
        if sched.trigger_type != "cron" {
            continue;
        }
        if let Some(ref expr) = sched.cron_expr {
            if crate::schedule::cron_should_fire(expr, sched.last_fired_at.as_ref(), &now) {
                fire_schedule(state, &sched).await;
            }
        }
        let _ = state.db.mark_schedule_checked(sched.id).await;
    }
    Ok(())
}

/// Fire a schedule: create task(s) based on `action_type`.
///
/// - `action_type = "template"`: expand a named template into a parent + child
///   task tree via [`Database::expand_template`].
/// - Any other value (including `"direct"` or empty): create a single flat task
///   via [`Database::create_agent_task`].
///
/// Increments the `agent_schedule_fires` Prometheus counter on success and
/// updates `last_fired_at` via [`Database::fire_schedule`].
pub(crate) async fn fire_schedule(state: &AppState, sched: &AgentScheduleRow) {
    let source = "schedule";
    let result = if sched.action_type == "template" {
        if let Some(ref tpl) = sched.template_name {
            state
                .db
                .expand_template(
                    tpl,
                    &sched.task_title,
                    &sched.task_description,
                    &sched.priority,
                    sched.max_cost_usd,
                    sched.permission_level,
                    sched.role_name.as_deref(),
                )
                .await
        } else {
            Err(anyhow::anyhow!(
                "template schedule '{}' missing template_name",
                sched.name
            ))
        }
    } else {
        state
            .db
            .create_agent_task(
                &sched.task_title,
                &sched.task_description,
                &sched.priority,
                None,
                &source,
                sched.max_cost_usd,
                sched.permission_level,
                sched.role_name.as_deref(),
            )
            .await
            .map(|task| task.id)
    };

    match result {
        Ok(task_id) => {
            info!(schedule = %sched.name, task_id, "schedule: fired");
            state.prom_metrics.agent_schedule_fires.inc();
        }
        Err(e) => {
            warn!(schedule = %sched.name, error = %e, "schedule: failed to create task");
        }
    }
    let _ = state.db.fire_schedule(sched.id).await;
}

/// Main daemon loop with two-frequency `tokio::select!`.
///
/// - Fast tick (`poll_interval_secs`): poll for completed agents
/// - Slow tick (`claim_interval_secs`): enforce budget, reclaim orphans, claim + spawn
///
/// At startup, reclaims any stale in-progress tasks left from a previous
/// coordinator instance.
pub async fn run_daemon(state: Arc<AppState>, config: AgentDaemonConfig) {
    // Startup: reclaim orphaned tasks from previous coordinator runs
    match state
        .db
        .reclaim_stale_agent_tasks(config.stale_task_secs)
        .await
    {
        Ok(n) if n > 0 => info!(count = n, "agent daemon: reclaimed stale tasks at startup"),
        Err(e) => warn!(error = %e, "agent daemon: failed to reclaim stale tasks at startup"),
        _ => {}
    }

    let mut poll_interval =
        tokio::time::interval(std::time::Duration::from_secs(config.poll_interval_secs));
    let mut claim_interval =
        tokio::time::interval(std::time::Duration::from_secs(config.claim_interval_secs));

    // Stale task reclamation runs every 5 minutes
    let mut reclaim_interval = tokio::time::interval(std::time::Duration::from_secs(300));

    // Budget period rotation runs every 60 seconds
    let mut budget_rotation_interval = tokio::time::interval(std::time::Duration::from_secs(60));

    // Schedule evaluation runs every 60 seconds
    let mut schedule_interval = tokio::time::interval(std::time::Duration::from_secs(60));

    // Consume the initial ticks
    poll_interval.tick().await;
    claim_interval.tick().await;
    reclaim_interval.tick().await;
    budget_rotation_interval.tick().await;
    schedule_interval.tick().await;

    loop {
        tokio::select! {
            _ = poll_interval.tick() => {
                // Fast path: check for completed agents
                let completed = lock_or_recover(&state.agents).poll_completed();
                for c in completed {
                    handle_completed_agent(c, &state).await;
                }
            }
            _ = claim_interval.tick() => {
                // Slow path: budget check → claim → spawn
                if enforce_global_budget(&state).await {
                    try_claim_and_spawn(&state, &config.agent_name).await;
                }
            }
            _ = reclaim_interval.tick() => {
                // Periodic orphan recovery
                match state.db.reclaim_stale_agent_tasks(config.stale_task_secs).await {
                    Ok(n) if n > 0 => info!(count = n, "agent daemon: reclaimed stale tasks"),
                    Err(e) => warn!(error = %e, "agent daemon: failed to reclaim stale tasks"),
                    _ => {}
                }
            }
            _ = budget_rotation_interval.tick() => {
                // Reset spent_usd/tokens_used for expired budget periods
                match state.db.rotate_agent_budget_periods().await {
                    Ok(n) if n > 0 => info!(count = n, "agent daemon: rotated expired budget periods"),
                    Err(e) => warn!(error = %e, "agent daemon: failed to rotate budget periods"),
                    _ => {}
                }
            }
            _ = schedule_interval.tick() => {
                // Evaluate cron schedules and fire matching ones as agent tasks
                if let Err(e) = evaluate_cron_schedules(&state).await {
                    warn!(error = %e, "agent daemon: schedule evaluation failed");
                }
            }
        }
    }
}

/// Map an [`AgentStatus`] variant to the database status string.
///
/// The `agent_tasks.status` column uses these values:
/// - `"completed"` — agent finished successfully
/// - `"failed"` — agent exited with error or timed out
/// - `"cancelled"` — agent was cancelled by user or budget
/// - `"in_progress"` — agent is still running (shouldn't normally appear here)
pub fn status_to_db_str(status: &AgentStatus) -> &'static str {
    match status {
        AgentStatus::Completed => "completed",
        AgentStatus::Failed { .. } => "failed",
        AgentStatus::TimedOut => "failed",
        AgentStatus::Cancelled => "cancelled",
        AgentStatus::Running => "in_progress",
    }
}

#[cfg(test)]
mod tests {
    //! Tests for the agent daemon configuration and helpers.
    //!
    //! Since the daemon functions are async and require a Database + AppState,
    //! these tests focus on the pure logic: config parsing, status mapping,
    //! and result JSON construction.

    use super::*;
    use crate::agent::{AgentStatus, StdoutResult};

    // Mutex to serialize tests that mutate environment variables.
    // Rust runs tests in parallel, so concurrent set_var/remove_var races
    // would cause intermittent failures.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // ── Config from environment ────────────────────────────────────

    /// All env-dependent config tests run under a single lock to avoid
    /// racing on `DARKREACH_AGENT_*` environment variables.
    #[test]
    fn config_from_env_variants() {
        let _lock = ENV_LOCK.lock().unwrap();

        // 1. Defaults when no env vars are set
        std::env::remove_var("DARKREACH_AGENT_DAEMON");
        std::env::remove_var("DARKREACH_AGENT_POLL_SECS");
        std::env::remove_var("DARKREACH_AGENT_CLAIM_SECS");
        std::env::remove_var("DARKREACH_AGENT_STALE_SECS");

        let config = AgentDaemonConfig::from_env();
        assert!(config.enabled, "daemon should be enabled by default");
        assert_eq!(config.poll_interval_secs, 5);
        assert_eq!(config.claim_interval_secs, 10);
        assert_eq!(config.stale_task_secs, 3600);
        assert!(config.agent_name.starts_with("coordinator@"));

        // 2. DARKREACH_AGENT_DAEMON=false disables the daemon
        std::env::set_var("DARKREACH_AGENT_DAEMON", "false");
        let config = AgentDaemonConfig::from_env();
        assert!(!config.enabled, "daemon should be disabled by 'false'");

        // 3. DARKREACH_AGENT_DAEMON=0 also disables
        std::env::set_var("DARKREACH_AGENT_DAEMON", "0");
        let config = AgentDaemonConfig::from_env();
        assert!(!config.enabled, "daemon should be disabled by '0'");

        // 4. Any other value keeps it enabled
        std::env::set_var("DARKREACH_AGENT_DAEMON", "true");
        let config = AgentDaemonConfig::from_env();
        assert!(config.enabled, "daemon should be enabled by 'true'");

        // Cleanup
        std::env::remove_var("DARKREACH_AGENT_DAEMON");
    }

    // ── Status mapping ────────────────────────────────────────────

    /// All AgentStatus variants map to the correct DB status string.
    #[test]
    fn status_mapping_completed() {
        assert_eq!(status_to_db_str(&AgentStatus::Completed), "completed");
    }

    #[test]
    fn status_mapping_failed() {
        assert_eq!(
            status_to_db_str(&AgentStatus::Failed {
                reason: "exit 1".into()
            }),
            "failed"
        );
    }

    #[test]
    fn status_mapping_timed_out_is_failed() {
        assert_eq!(status_to_db_str(&AgentStatus::TimedOut), "failed");
    }

    #[test]
    fn status_mapping_cancelled() {
        assert_eq!(status_to_db_str(&AgentStatus::Cancelled), "cancelled");
    }

    #[test]
    fn status_mapping_running() {
        assert_eq!(status_to_db_str(&AgentStatus::Running), "in_progress");
    }

    // ── Result JSON construction ──────────────────────────────────

    /// A completed agent with stdout result produces {"text": "..."} JSON.
    #[test]
    fn result_json_from_stdout() {
        let result = StdoutResult {
            result_text: "All tests pass".to_string(),
            tokens_used: 1500,
            cost_usd: 0.05,
        };
        let json = serde_json::json!({"text": result.result_text});
        assert_eq!(json["text"], "All tests pass");
    }

    /// A failed agent with no stdout produces {"error": "reason"} JSON.
    #[test]
    fn result_json_from_error() {
        let reason = "Timed out".to_string();
        let json = serde_json::json!({"error": reason});
        assert_eq!(json["error"], "Timed out");
    }
}
