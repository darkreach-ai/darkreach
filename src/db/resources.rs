//! Resource contribution tracking — per-resource credit accounting.
//!
//! Tracks CPU, GPU, storage, and bandwidth contributions from operators,
//! with configurable credit conversion rates per resource type. Provides
//! fleet-wide resource snapshots for Prometheus metrics and dashboard display.

use super::Database;
use anyhow::Result;
use serde::Serialize;

/// A single resource contribution record from an operator completing a work block.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ResourceContributionRow {
    pub id: i64,
    pub operator_id: uuid::Uuid,
    pub block_id: i64,
    pub cpu_core_hours: f64,
    pub gpu_hours: f64,
    pub storage_gb_hours: f64,
    pub bandwidth_gb: f64,
    pub credits_earned: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Credit conversion rate for a resource type.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ResourceCreditRateRow {
    pub resource_type: String,
    pub credits_per_unit: f64,
    pub unit_label: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Aggregated resource contribution totals for an operator.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceSummary {
    pub total_cpu_core_hours: f64,
    pub total_gpu_hours: f64,
    pub total_storage_gb_hours: f64,
    pub total_bandwidth_gb: f64,
    pub total_credits_earned: f64,
    pub contribution_count: i64,
}

/// Live fleet resource capacity snapshot from active operator nodes.
#[derive(Debug, Clone, Serialize)]
pub struct FleetResourceSnapshot {
    pub gpu_worker_count: i64,
    pub total_storage_gb: f64,
    pub total_relay_bandwidth_mbps: f64,
}

impl Database {
    /// Insert a new resource contribution record for a completed work block.
    pub async fn record_resource_contribution(
        &self,
        operator_id: uuid::Uuid,
        block_id: i64,
        cpu_core_hours: f64,
        gpu_hours: f64,
        storage_gb_hours: f64,
        bandwidth_gb: f64,
        credits_earned: f64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO resource_contributions
               (operator_id, block_id, cpu_core_hours, gpu_hours,
                storage_gb_hours, bandwidth_gb, credits_earned)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(operator_id)
        .bind(block_id)
        .bind(cpu_core_hours)
        .bind(gpu_hours)
        .bind(storage_gb_hours)
        .bind(bandwidth_gb)
        .bind(credits_earned)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get all resource credit conversion rates.
    pub async fn get_resource_credit_rates(&self) -> Result<Vec<ResourceCreditRateRow>> {
        let rows = sqlx::query_as::<_, ResourceCreditRateRow>(
            "SELECT resource_type, credits_per_unit::float8 AS credits_per_unit,
                    unit_label, updated_at
             FROM resource_credit_rates
             ORDER BY resource_type",
        )
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Get aggregated resource summary for a specific operator.
    pub async fn get_operator_resource_summary(
        &self,
        operator_id: uuid::Uuid,
    ) -> Result<ResourceSummary> {
        let row: (f64, f64, f64, f64, f64, i64) = sqlx::query_as(
            "SELECT
               COALESCE(SUM(cpu_core_hours::float8), 0),
               COALESCE(SUM(gpu_hours::float8), 0),
               COALESCE(SUM(storage_gb_hours::float8), 0),
               COALESCE(SUM(bandwidth_gb::float8), 0),
               COALESCE(SUM(credits_earned::float8), 0),
               COUNT(*)
             FROM resource_contributions
             WHERE operator_id = $1",
        )
        .bind(operator_id)
        .fetch_one(&self.read_pool)
        .await?;
        Ok(ResourceSummary {
            total_cpu_core_hours: row.0,
            total_gpu_hours: row.1,
            total_storage_gb_hours: row.2,
            total_bandwidth_gb: row.3,
            total_credits_earned: row.4,
            contribution_count: row.5,
        })
    }

    /// Get a live snapshot of fleet resource capacity from active operator nodes
    /// (heartbeat within the last 5 minutes).
    pub async fn get_fleet_resource_snapshot(&self) -> Result<FleetResourceSnapshot> {
        let row: (i64, f64, f64) = sqlx::query_as(
            "SELECT
               COALESCE(COUNT(*) FILTER (WHERE has_gpu = true), 0),
               COALESCE(SUM(COALESCE(storage_dedicated_gb, 0))::float8, 0),
               COALESCE(SUM(COALESCE(network_upload_mbps, 0))::float8
                 FILTER (WHERE network_role = 'relay'), 0)
             FROM operator_nodes
             WHERE last_heartbeat > NOW() - INTERVAL '5 minutes'",
        )
        .fetch_one(&self.read_pool)
        .await?;
        Ok(FleetResourceSnapshot {
            gpu_worker_count: row.0,
            total_storage_gb: row.1,
            total_relay_bandwidth_mbps: row.2,
        })
    }

    /// Get recent resource contributions for an operator, newest first.
    pub async fn get_resource_contributions(
        &self,
        operator_id: uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<ResourceContributionRow>> {
        let rows = sqlx::query_as::<_, ResourceContributionRow>(
            "SELECT id, operator_id, block_id,
                    cpu_core_hours::float8 AS cpu_core_hours,
                    gpu_hours::float8 AS gpu_hours,
                    storage_gb_hours::float8 AS storage_gb_hours,
                    bandwidth_gb::float8 AS bandwidth_gb,
                    credits_earned::float8 AS credits_earned,
                    created_at
             FROM resource_contributions
             WHERE operator_id = $1
             ORDER BY created_at DESC
             LIMIT $2",
        )
        .bind(operator_id)
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Get the capabilities of the node that claimed a specific work block.
    /// Returns (operator_id, cores, has_gpu) or None if not found.
    pub async fn get_node_for_block(
        &self,
        block_id: i64,
    ) -> Result<Option<(uuid::Uuid, i32, bool)>> {
        let row: Option<(uuid::Uuid, i32, bool)> = sqlx::query_as(
            "SELECT n.operator_id, COALESCE(n.cores, 1), COALESCE(n.has_gpu, false)
             FROM work_blocks b
             JOIN operator_nodes n ON n.worker_id = b.claimed_by
             WHERE b.id = $1",
        )
        .bind(block_id)
        .fetch_optional(&self.read_pool)
        .await?;
        Ok(row)
    }
}
