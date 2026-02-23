//! Operator badge system — achievement tracking and gamification.
//!
//! Badges are earned by operators when they reach metric thresholds
//! (primes found, blocks completed, core-hours, trust level, etc.).
//! The `check_and_grant_badges` PL/pgSQL function evaluates all criteria
//! and inserts newly earned badges atomically.

use super::Database;
use anyhow::Result;
use serde::Serialize;

/// A badge definition row from the `badge_definitions` table.
#[derive(Serialize, sqlx::FromRow)]
pub struct BadgeDefinitionRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tier: String,
    pub threshold: i64,
    pub metric: String,
    pub icon: String,
}

/// An earned badge row from the `operator_badges` join table.
#[derive(Serialize, sqlx::FromRow)]
pub struct OperatorBadgeRow {
    pub badge_id: String,
    pub granted_at: chrono::DateTime<chrono::Utc>,
}

/// Extended badge info including definition details (for display).
#[derive(Serialize, sqlx::FromRow)]
pub struct OperatorBadgeDetailRow {
    pub badge_id: String,
    pub name: String,
    pub description: String,
    pub tier: String,
    pub icon: String,
    pub granted_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    /// Evaluate all badge criteria for an operator and grant any newly earned
    /// badges. Returns the count of newly granted badges.
    pub async fn check_and_grant_badges(&self, operator_id: uuid::Uuid) -> Result<i32> {
        let row: (i32,) =
            sqlx::query_as("SELECT check_and_grant_badges($1)")
                .bind(operator_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }

    /// Get all badges earned by an operator, ordered by grant time.
    pub async fn get_operator_badges(
        &self,
        operator_id: uuid::Uuid,
    ) -> Result<Vec<OperatorBadgeRow>> {
        let rows = sqlx::query_as::<_, OperatorBadgeRow>(
            "SELECT badge_id, granted_at
             FROM operator_badges
             WHERE operator_id = $1
             ORDER BY granted_at DESC",
        )
        .bind(operator_id)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Get all badge definitions (for public display).
    pub async fn get_badge_definitions(&self) -> Result<Vec<BadgeDefinitionRow>> {
        let rows = sqlx::query_as::<_, BadgeDefinitionRow>(
            "SELECT id, name, description, tier, threshold, metric, icon
             FROM badge_definitions
             ORDER BY threshold",
        )
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }
}
