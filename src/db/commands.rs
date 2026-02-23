//! Node command queue — multi-command delivery with ACK lifecycle.
//!
//! Replaces the single `pending_command TEXT` column on the workers table
//! with a proper queue supporting multiple commands, delivery tracking,
//! retries, and expiration. Commands are delivered FIFO during heartbeats.
//!
//! ## Lifecycle
//!
//! ```text
//! queued → delivered → acked     (happy path)
//! queued → delivered → expired   (timeout + max retries exceeded)
//! queued → cancelled             (admin cancel)
//! ```
//!
//! ## Supported Commands
//!
//! | Command | Effect |
//! |---------|--------|
//! | `stop` | Graceful shutdown via `stop_requested` flag |
//! | `restart` | Stop + signal for supervisor restart |
//! | `update_config` | Apply runtime config from `params` JSONB |
//! | `reassign` | Switch to different search form/params |
//! | `upgrade` | Pull new binary version and restart |

use super::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Command types that can be sent to worker nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Stop,
    Restart,
    UpdateConfig,
    Reassign,
    Upgrade,
}

impl CommandType {
    /// Database string representation (matches CHECK constraint).
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandType::Stop => "stop",
            CommandType::Restart => "restart",
            CommandType::UpdateConfig => "update_config",
            CommandType::Reassign => "reassign",
            CommandType::Upgrade => "upgrade",
        }
    }

    /// Parse from database string. Returns `None` for unknown commands.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stop" => Some(CommandType::Stop),
            "restart" => Some(CommandType::Restart),
            "update_config" => Some(CommandType::UpdateConfig),
            "reassign" => Some(CommandType::Reassign),
            "upgrade" => Some(CommandType::Upgrade),
            _ => None,
        }
    }

    /// Default timeout in seconds for each command type.
    pub fn default_timeout_secs(&self) -> i32 {
        match self {
            CommandType::Stop => 60,
            CommandType::Restart => 120,
            CommandType::UpdateConfig => 30,
            CommandType::Reassign => 300,
            CommandType::Upgrade => 600,
        }
    }
}

/// A command fetched during heartbeat, ready for the worker to process.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PendingCommand {
    pub command_id: i64,
    pub command: String,
    pub params: Option<Value>,
}

/// Full command row for dashboard display and history.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct NodeCommandRow {
    pub id: i64,
    pub node_id: String,
    pub command: String,
    pub params: Option<Value>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub acked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expired_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_after_s: i32,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub created_by: Option<String>,
}

impl Database {
    /// Queue a command for a worker node.
    ///
    /// The command is inserted with status `queued` and will be delivered
    /// on the node's next heartbeat via `fetch_node_commands`. Returns
    /// the command ID for tracking.
    pub async fn queue_node_command(
        &self,
        node_id: &str,
        command: CommandType,
        params: Option<&Value>,
        created_by: Option<&str>,
    ) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO node_commands (node_id, command, params, expires_after_s, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(node_id)
        .bind(command.as_str())
        .bind(params)
        .bind(command.default_timeout_secs())
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Fetch and deliver pending commands for a node.
    ///
    /// Calls the `fetch_node_commands` SQL function which atomically marks
    /// queued commands as delivered and re-delivers stale ones under max_retries.
    /// Returns commands in FIFO order.
    pub async fn fetch_node_commands(&self, node_id: &str) -> Result<Vec<PendingCommand>> {
        let rows = sqlx::query_as::<_, PendingCommand>(
            "SELECT command_id, command, params FROM fetch_node_commands($1)",
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Acknowledge a delivered command after processing.
    ///
    /// Optionally includes an error message if the command failed on the
    /// worker side (the command is still marked acked — the worker tried).
    /// Returns `true` if the command was found and updated.
    pub async fn ack_node_command(
        &self,
        command_id: i64,
        error_message: Option<&str>,
    ) -> Result<bool> {
        let acked: bool = sqlx::query_scalar("SELECT ack_node_command($1, $2)")
            .bind(command_id)
            .bind(error_message)
            .fetch_one(&self.pool)
            .await?;
        Ok(acked)
    }

    /// Expire stale delivered commands that have exceeded their timeout
    /// and max retry count. Called periodically from the background loop.
    /// Returns the number of commands expired.
    pub async fn expire_stale_commands(&self) -> Result<i32> {
        let count: i32 = sqlx::query_scalar("SELECT expire_stale_commands()")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Get command history for a specific node (dashboard display).
    /// Returns commands ordered by creation time (newest first), limited.
    pub async fn get_node_commands(
        &self,
        node_id: &str,
        limit: i64,
    ) -> Result<Vec<NodeCommandRow>> {
        let rows = sqlx::query_as::<_, NodeCommandRow>(
            "SELECT id, node_id, command, params, status, created_at,
                    delivered_at, acked_at, expired_at, expires_after_s,
                    retry_count, max_retries, error_message, created_by
             FROM node_commands
             WHERE node_id = $1
             ORDER BY created_at DESC
             LIMIT $2",
        )
        .bind(node_id)
        .bind(limit)
        .fetch_all(self.read_pool())
        .await?;
        Ok(rows)
    }

    /// Cancel a queued command (before it's delivered).
    /// Returns `true` if the command was found and cancelled.
    pub async fn cancel_node_command(&self, command_id: i64) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE node_commands SET status = 'cancelled' WHERE id = $1 AND status = 'queued'",
        )
        .bind(command_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_type_roundtrip() {
        let types = [
            CommandType::Stop,
            CommandType::Restart,
            CommandType::UpdateConfig,
            CommandType::Reassign,
            CommandType::Upgrade,
        ];
        for ct in types {
            let s = ct.as_str();
            let parsed = CommandType::from_str(s).expect("should parse");
            assert_eq!(ct, parsed);
        }
    }

    #[test]
    fn command_type_from_str_unknown_returns_none() {
        assert_eq!(CommandType::from_str("unknown"), None);
        assert_eq!(CommandType::from_str(""), None);
        assert_eq!(CommandType::from_str("STOP"), None);
    }

    #[test]
    fn command_type_default_timeouts_are_positive() {
        let types = [
            CommandType::Stop,
            CommandType::Restart,
            CommandType::UpdateConfig,
            CommandType::Reassign,
            CommandType::Upgrade,
        ];
        for ct in types {
            assert!(
                ct.default_timeout_secs() > 0,
                "{:?} timeout should be > 0",
                ct
            );
        }
    }

    #[test]
    fn command_type_serde_roundtrip() {
        let ct = CommandType::UpdateConfig;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, r#""update_config""#);
        let parsed: CommandType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ct);
    }
}
