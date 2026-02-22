//! Audit log operations.
//!
//! Records API access events for security monitoring and compliance.
//! Each entry captures who performed what action, from where, and when.
//! Supports filtered queries for reviewing activity by user, action type,
//! and time range.

use super::{AuditLogEntry, Database};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;

impl Database {
    /// Insert a new audit log entry.
    ///
    /// Called by middleware or route handlers to record API access events.
    /// Writes go to the primary pool since audit logs are append-only and
    /// must not be lost.
    pub async fn insert_audit_log(
        &self,
        user_id: &str,
        user_email: Option<&str>,
        action: &str,
        resource: Option<&str>,
        method: &str,
        status_code: Option<i32>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        payload: Option<&Value>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO audit_log (user_id, user_email, action, resource, method,
                                    status_code, ip_address, user_agent, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(user_id)
        .bind(user_email)
        .bind(action)
        .bind(resource)
        .bind(method)
        .bind(status_code)
        .bind(ip_address)
        .bind(user_agent)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Query audit log entries with optional filters, ordered by most recent first.
    ///
    /// Supports pagination via `limit` and `offset`, and optional filtering by
    /// `user_id`, `action`, and `since` (minimum timestamp). Reads from the
    /// read replica pool since audit queries are read-only and can tolerate
    /// slight replication lag.
    ///
    /// Uses dynamic query building with parameterized conditions to avoid
    /// the combinatorial explosion of separate query branches.
    pub async fn get_audit_log(
        &self,
        limit: i64,
        offset: i64,
        user_id_filter: Option<&str>,
        action_filter: Option<&str>,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<AuditLogEntry>> {
        // Build dynamic WHERE clause with positional parameters.
        // Each filter appends a condition and increments the parameter index.
        let mut conditions = Vec::new();
        let mut param_idx = 1u32;

        if user_id_filter.is_some() {
            conditions.push(format!("user_id = ${param_idx}"));
            param_idx += 1;
        }
        if action_filter.is_some() {
            conditions.push(format!("action = ${param_idx}"));
            param_idx += 1;
        }
        if since.is_some() {
            conditions.push(format!("created_at >= ${param_idx}"));
            param_idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, user_id, user_email, action, resource, method,
                    status_code, ip_address, user_agent, payload, created_at
             FROM audit_log
             {where_clause}
             ORDER BY created_at DESC
             LIMIT ${} OFFSET ${}",
            param_idx,
            param_idx + 1
        );

        let mut query = sqlx::query_as::<_, AuditLogEntry>(&sql);

        if let Some(uid) = user_id_filter {
            query = query.bind(uid);
        }
        if let Some(action) = action_filter {
            query = query.bind(action);
        }
        if let Some(ts) = since {
            query = query.bind(ts);
        }
        query = query.bind(limit).bind(offset);

        let rows = query.fetch_all(&self.read_pool).await?;
        Ok(rows)
    }

    /// Count total audit log entries matching the given filters.
    ///
    /// Used for pagination — returns the total number of matching rows
    /// so the frontend can compute page count.
    pub async fn count_audit_log(
        &self,
        user_id_filter: Option<&str>,
        action_filter: Option<&str>,
        since: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let mut conditions = Vec::new();
        let mut param_idx = 1u32;

        if user_id_filter.is_some() {
            conditions.push(format!("user_id = ${param_idx}"));
            param_idx += 1;
        }
        if action_filter.is_some() {
            conditions.push(format!("action = ${param_idx}"));
            param_idx += 1;
        }
        if since.is_some() {
            conditions.push(format!("created_at >= ${param_idx}"));
            // param_idx not needed after this, but kept for clarity
            let _ = param_idx;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) as count FROM audit_log {where_clause}");

        let mut query = sqlx::query_scalar::<_, i64>(&sql);

        if let Some(uid) = user_id_filter {
            query = query.bind(uid);
        }
        if let Some(action) = action_filter {
            query = query.bind(action);
        }
        if let Some(ts) = since {
            query = query.bind(ts);
        }

        let count = query.fetch_one(&self.read_pool).await?;
        Ok(count)
    }
}
