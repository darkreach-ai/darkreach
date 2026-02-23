//! # Shared Sieve Cache — Database Operations
//!
//! CRUD methods for the `shared_sieves` table, which stores hash-addressed
//! immutable sieve blobs. Workers upload computed BSGS sieves so subsequent
//! workers can download them instead of recomputing.
//!
//! ## Table Schema
//!
//! - `hash` (PK): SHA-256 of sieve parameters (form, k, base, min_n, max_n, sieve_limit)
//! - `blob`: serialized `(BitSieve, BitSieve)` pair
//! - `hit_count`: incremented on each cache hit for analytics
//! - `created_at`: used by pruning to reclaim old entries

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Database;

/// Row type for shared sieve listing (excludes the blob for efficiency).
#[derive(Serialize, sqlx::FromRow)]
pub struct SharedSieveRow {
    pub hash: String,
    pub form: String,
    pub k: i64,
    pub base: i32,
    pub min_n: i64,
    pub max_n: i64,
    pub sieve_limit: i64,
    pub size_bytes: i64,
    pub uploaded_by: Option<String>,
    pub hit_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    /// Look up a cached sieve by identity hash. Returns the raw blob if found.
    pub async fn get_shared_sieve(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT blob FROM shared_sieves WHERE hash = $1")
                .bind(hash)
                .fetch_optional(self.read_pool())
                .await?;
        Ok(row.map(|(blob,)| blob))
    }

    /// Store a computed sieve blob. No-op if hash already exists (ON CONFLICT DO NOTHING).
    pub async fn insert_shared_sieve(
        &self,
        hash: &str,
        form: &str,
        k: u64,
        base: u32,
        min_n: u64,
        max_n: u64,
        sieve_limit: u64,
        blob: &[u8],
        uploaded_by: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO shared_sieves (hash, form, k, base, min_n, max_n, sieve_limit, blob, size_bytes, uploaded_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (hash) DO NOTHING",
        )
        .bind(hash)
        .bind(form)
        .bind(k as i64)
        .bind(base as i32)
        .bind(min_n as i64)
        .bind(max_n as i64)
        .bind(sieve_limit as i64)
        .bind(blob)
        .bind(blob.len() as i64)
        .bind(uploaded_by)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Increment hit count for cache analytics.
    pub async fn increment_sieve_hit_count(&self, hash: &str) -> Result<()> {
        sqlx::query("UPDATE shared_sieves SET hit_count = hit_count + 1 WHERE hash = $1")
            .bind(hash)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// List shared sieves for a form (for API/dashboard), excluding the blob column.
    pub async fn list_shared_sieves(&self, form: &str, limit: i64) -> Result<Vec<SharedSieveRow>> {
        let rows = sqlx::query_as::<_, SharedSieveRow>(
            "SELECT hash, form, k, base, min_n, max_n, sieve_limit, size_bytes, uploaded_by, hit_count, created_at
             FROM shared_sieves
             WHERE form = $1
             ORDER BY created_at DESC
             LIMIT $2",
        )
        .bind(form)
        .bind(limit)
        .fetch_all(self.read_pool())
        .await?;
        Ok(rows)
    }

    /// Delete old sieves to reclaim storage. Returns number of rows deleted.
    pub async fn prune_shared_sieves(&self, older_than_days: i32) -> Result<i64> {
        let result = sqlx::query(
            "DELETE FROM shared_sieves WHERE created_at < NOW() - make_interval(days => $1)",
        )
        .bind(older_than_days)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected() as i64)
    }

    // ── Relay Node Management ─────────────────────────────────────

    /// Promote an operator node to relay role.
    pub async fn promote_to_relay(&self, worker_id: &str) -> Result<()> {
        sqlx::query("UPDATE operator_nodes SET network_role = 'relay' WHERE worker_id = $1")
            .bind(worker_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// Demote a relay node back to worker role.
    pub async fn demote_from_relay(&self, worker_id: &str) -> Result<()> {
        sqlx::query("UPDATE operator_nodes SET network_role = 'worker' WHERE worker_id = $1")
            .bind(worker_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// Get all active relay nodes (heartbeat within 5 minutes).
    pub async fn get_relay_nodes(&self) -> Result<Vec<RelayNodeRow>> {
        let rows = sqlx::query_as::<_, RelayNodeRow>(
            "SELECT worker_id, hostname, network_upload_mbps, network_region, last_heartbeat
             FROM operator_nodes
             WHERE network_role = 'relay'
               AND last_heartbeat > NOW() - INTERVAL '5 minutes'
             ORDER BY COALESCE(network_upload_mbps, 0) DESC",
        )
        .fetch_all(self.read_pool())
        .await?;
        Ok(rows)
    }

    /// Find the best relay for a given sieve hash, preferring same region.
    pub async fn find_relay_for_sieve(
        &self,
        sieve_hash: &str,
        region: Option<&str>,
    ) -> Result<Option<RelayNodeRow>> {
        let row = sqlx::query_as::<_, RelayNodeRow>(
            "SELECT on2.worker_id, on2.hostname, on2.network_upload_mbps,
                    on2.network_region, on2.last_heartbeat
             FROM relay_sieve_cache rsc
             JOIN operator_nodes on2 ON on2.worker_id = rsc.relay_worker_id
             WHERE rsc.sieve_hash = $1
               AND on2.network_role = 'relay'
               AND on2.last_heartbeat > NOW() - INTERVAL '5 minutes'
             ORDER BY
               CASE WHEN on2.network_region = $2 THEN 0 ELSE 1 END,
               COALESCE(on2.network_upload_mbps, 0) DESC
             LIMIT 1",
        )
        .bind(sieve_hash)
        .bind(region)
        .fetch_optional(self.read_pool())
        .await?;
        Ok(row)
    }

    /// Register a sieve cache entry on a relay node (relay announces it has this sieve).
    pub async fn register_relay_sieve_cache(
        &self,
        relay_worker_id: &str,
        sieve_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO relay_sieve_cache (relay_worker_id, sieve_hash)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(relay_worker_id)
        .bind(sieve_hash)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Log a relay event (promotion, demotion, cache hit/miss).
    pub async fn log_relay_event(
        &self,
        worker_id: &str,
        event_type: &str,
        detail: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO relay_events (worker_id, event_type, detail)
             VALUES ($1, $2, $3)",
        )
        .bind(worker_id)
        .bind(event_type)
        .bind(detail)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Get relay candidates: nodes with upload > threshold, public IP, trust >= 2.
    pub async fn get_relay_candidates(&self, min_upload_mbps: f32) -> Result<Vec<RelayNodeRow>> {
        let rows = sqlx::query_as::<_, RelayNodeRow>(
            "SELECT on2.worker_id, on2.hostname, on2.network_upload_mbps,
                    on2.network_region, on2.last_heartbeat
             FROM operator_nodes on2
             LEFT JOIN operator_trust ot ON ot.volunteer_id = on2.volunteer_id
             WHERE on2.network_role = 'worker'
               AND COALESCE(on2.network_upload_mbps, 0) >= $1
               AND on2.network_public_ip = TRUE
               AND on2.last_heartbeat > NOW() - INTERVAL '5 minutes'
               AND COALESCE(ot.trust_level, 0) >= 2
             ORDER BY COALESCE(on2.network_upload_mbps, 0) DESC",
        )
        .bind(min_upload_mbps)
        .fetch_all(self.read_pool())
        .await?;
        Ok(rows)
    }

    /// Get relay stats for dashboard: count of active relays, total cached sieves, events.
    pub async fn get_relay_stats(&self) -> Result<RelayStats> {
        let active_relays: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM operator_nodes
             WHERE network_role = 'relay'
               AND last_heartbeat > NOW() - INTERVAL '5 minutes'",
        )
        .fetch_one(self.read_pool())
        .await?;

        let cached_sieves: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM relay_sieve_cache")
            .fetch_one(self.read_pool())
            .await?;

        let recent_events: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM relay_events
             WHERE created_at > NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(self.read_pool())
        .await?;

        Ok(RelayStats {
            active_relays: active_relays.0 as u32,
            cached_sieves: cached_sieves.0 as u32,
            recent_events_24h: recent_events.0 as u32,
        })
    }
}

/// Active relay node row.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RelayNodeRow {
    pub worker_id: String,
    pub hostname: Option<String>,
    pub network_upload_mbps: Option<f32>,
    pub network_region: Option<String>,
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
}

/// Relay statistics for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStats {
    pub active_relays: u32,
    pub cached_sieves: u32,
    pub recent_events_24h: u32,
}
