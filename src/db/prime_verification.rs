//! Distributed prime verification queue operations.
//!
//! Manages the `prime_verification_queue` and `prime_verification_summary`
//! tables for independent re-verification of discovered primes by network
//! nodes. Multiple nodes run the 3-tier `verify_prime()` pipeline on each
//! prime; when quorum is met, the prime is tagged `verified-distributed`.
//!
//! This is distinct from the work-block `verification_queue` (in `trust.rs`)
//! which cross-checks search blocks for node reliability scoring.

use super::Database;
use anyhow::Result;
use serde::Serialize;

/// A prime verification task with joined prime details for the verifier.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PrimeVerificationTask {
    pub task_id: i64,
    pub prime_id: i64,
    pub form: String,
    pub expression: String,
    pub digits: i64,
    pub search_params: String,
    pub proof_method: String,
}

/// Aggregate queue statistics for the dashboard.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PrimeVerificationQueueStats {
    pub pending: i64,
    pub claimed: i64,
    pub verified: i64,
    pub failed: i64,
    pub total_primes: i64,
    pub quorum_met: i64,
}

/// Individual verification result row for per-prime history.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PrimeVerificationResultRow {
    pub id: i64,
    pub prime_id: i64,
    pub status: String,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub verification_tier: Option<i16>,
    pub verification_method: Option<String>,
    pub result_detail: Option<serde_json::Value>,
    pub error_reason: Option<String>,
}

impl Database {
    /// Enqueue a prime for distributed verification.
    ///
    /// Creates the summary row (quorum tracker) and N pending queue entries
    /// where N = required_quorum. The `discoverer_worker` is recorded so the
    /// claim query can exclude the original discoverer.
    pub async fn enqueue_prime_verification(
        &self,
        prime_id: i64,
        required_quorum: i16,
        discoverer_worker: Option<&str>,
    ) -> Result<()> {
        // Insert or update summary row (idempotent for re-enqueue)
        sqlx::query(
            "INSERT INTO prime_verification_summary
                (prime_id, required_quorum, discoverer_worker)
             VALUES ($1, $2, $3)
             ON CONFLICT (prime_id) DO NOTHING",
        )
        .bind(prime_id)
        .bind(required_quorum)
        .bind(discoverer_worker)
        .execute(&self.pool)
        .await?;

        // Create pending queue entries (one per required verifier)
        for _ in 0..required_quorum {
            sqlx::query("INSERT INTO prime_verification_queue (prime_id) VALUES ($1)")
                .bind(prime_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Claim the next pending prime verification task.
    ///
    /// Uses CTE + `FOR UPDATE SKIP LOCKED` for concurrent safety.
    /// Excludes tasks where the claimer is the original discoverer.
    /// Returns task with joined prime details so the verifier can reconstruct
    /// the candidate and run `verify_prime()`.
    pub async fn claim_prime_verification(
        &self,
        worker_id: &str,
    ) -> Result<Option<PrimeVerificationTask>> {
        // Step 1: atomically claim one pending entry
        let claimed_id: Option<i64> = sqlx::query_scalar(
            "WITH claimable AS (
                SELECT pvq.id
                FROM prime_verification_queue pvq
                JOIN prime_verification_summary pvs ON pvs.prime_id = pvq.prime_id
                WHERE pvq.status = 'pending'
                  AND (pvs.discoverer_worker IS NULL OR pvs.discoverer_worker <> $1)
                  AND NOT EXISTS (
                      SELECT 1 FROM prime_verification_queue pvq2
                      WHERE pvq2.prime_id = pvq.prime_id
                        AND pvq2.claimed_by = $1
                  )
                ORDER BY pvq.id
                FOR UPDATE OF pvq SKIP LOCKED
                LIMIT 1
            )
            UPDATE prime_verification_queue pvq
            SET status = 'claimed',
                claimed_by = $1,
                claimed_at = NOW()
            FROM claimable
            WHERE pvq.id = claimable.id
            RETURNING pvq.id",
        )
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await?;

        match claimed_id {
            Some(id) => {
                // Step 2: fetch task with joined prime details
                let task = sqlx::query_as::<_, PrimeVerificationTask>(
                    "SELECT pvq.id AS task_id, pvq.prime_id,
                            p.form, p.expression, p.digits,
                            p.search_params, p.proof_method
                     FROM prime_verification_queue pvq
                     JOIN primes p ON p.id = pvq.prime_id
                     WHERE pvq.id = $1",
                )
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
                Ok(task)
            }
            None => Ok(None),
        }
    }

    /// Submit a prime verification result.
    ///
    /// Updates the queue entry, increments the summary counters, and
    /// checks if quorum is met. When quorum is reached, auto-tags the
    /// prime with `verified-distributed`.
    pub async fn submit_prime_verification(
        &self,
        task_id: i64,
        worker_id: &str,
        tier: i16,
        method: &str,
        result_json: Option<&serde_json::Value>,
        success: bool,
        error_reason: Option<&str>,
    ) -> Result<bool> {
        let status = if success { "verified" } else { "failed" };

        // Update the queue entry
        sqlx::query(
            "UPDATE prime_verification_queue
             SET status = $2,
                 verification_tier = $3,
                 verification_method = $4,
                 result_detail = $5,
                 error_reason = $6,
                 completed_at = NOW()
             WHERE id = $1 AND claimed_by = $7",
        )
        .bind(task_id)
        .bind(status)
        .bind(tier)
        .bind(method)
        .bind(result_json)
        .bind(error_reason)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;

        // Get the prime_id for this task
        let prime_id: i64 =
            sqlx::query_scalar("SELECT prime_id FROM prime_verification_queue WHERE id = $1")
                .bind(task_id)
                .fetch_one(&self.pool)
                .await?;

        // Update summary counters
        if success {
            sqlx::query(
                "UPDATE prime_verification_summary
                 SET verified_count = verified_count + 1,
                     highest_tier = GREATEST(highest_tier, $2)
                 WHERE prime_id = $1",
            )
            .bind(prime_id)
            .bind(tier)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE prime_verification_summary
                 SET failed_count = failed_count + 1
                 WHERE prime_id = $1",
            )
            .bind(prime_id)
            .execute(&self.pool)
            .await?;
        }

        // Check if quorum is now met
        let (verified_count, required_quorum): (i16, i16) = sqlx::query_as(
            "SELECT verified_count, required_quorum
             FROM prime_verification_summary
             WHERE prime_id = $1",
        )
        .bind(prime_id)
        .fetch_one(&self.pool)
        .await?;

        let quorum_met = verified_count >= required_quorum;

        if quorum_met {
            // Mark quorum as met in summary
            sqlx::query(
                "UPDATE prime_verification_summary
                 SET quorum_met = TRUE, quorum_met_at = NOW()
                 WHERE prime_id = $1 AND NOT quorum_met",
            )
            .bind(prime_id)
            .execute(&self.pool)
            .await?;

            // Tag the prime as verified-distributed
            self.add_prime_tags(prime_id, &["verified-distributed"])
                .await?;
        }

        Ok(quorum_met)
    }

    /// Reclaim stale prime verification tasks by calling the SQL function.
    pub async fn reclaim_stale_prime_verifications(&self, stale_seconds: i32) -> Result<i64> {
        let reclaimed: i32 = sqlx::query_scalar("SELECT reclaim_stale_prime_verifications($1)")
            .bind(stale_seconds)
            .fetch_one(&self.pool)
            .await?;
        Ok(reclaimed as i64)
    }

    /// Get aggregate queue statistics for the dashboard.
    pub async fn get_prime_verification_stats(&self) -> Result<PrimeVerificationQueueStats> {
        let stats = sqlx::query_as::<_, PrimeVerificationQueueStats>(
            "SELECT
                COALESCE(SUM(CASE WHEN pvq.status = 'pending' THEN 1 ELSE 0 END), 0)::BIGINT AS pending,
                COALESCE(SUM(CASE WHEN pvq.status = 'claimed' THEN 1 ELSE 0 END), 0)::BIGINT AS claimed,
                COALESCE(SUM(CASE WHEN pvq.status = 'verified' THEN 1 ELSE 0 END), 0)::BIGINT AS verified,
                COALESCE(SUM(CASE WHEN pvq.status = 'failed' THEN 1 ELSE 0 END), 0)::BIGINT AS failed,
                (SELECT COUNT(*)::BIGINT FROM prime_verification_summary) AS total_primes,
                (SELECT COUNT(*)::BIGINT FROM prime_verification_summary WHERE quorum_met) AS quorum_met
             FROM prime_verification_queue pvq",
        )
        .fetch_one(&self.read_pool)
        .await?;
        Ok(stats)
    }

    /// Get verification history for a specific prime.
    pub async fn get_prime_verification_results(
        &self,
        prime_id: i64,
    ) -> Result<Vec<PrimeVerificationResultRow>> {
        let rows = sqlx::query_as::<_, PrimeVerificationResultRow>(
            "SELECT id, prime_id, status, claimed_by, claimed_at, completed_at,
                    verification_tier, verification_method, result_detail, error_reason
             FROM prime_verification_queue
             WHERE prime_id = $1
             ORDER BY id",
        )
        .bind(prime_id)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Check if a prime already has pending/claimed verification entries.
    pub async fn has_pending_prime_verification(&self, prime_id: i64) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM prime_verification_queue
             WHERE prime_id = $1
               AND status IN ('pending', 'claimed')",
        )
        .bind(prime_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_verification_task_serialize() {
        let task = PrimeVerificationTask {
            task_id: 1,
            prime_id: 42,
            form: "factorial".to_string(),
            expression: "73! + 1".to_string(),
            digits: 105,
            search_params: "{}".to_string(),
            proof_method: "Pocklington".to_string(),
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["task_id"], 1);
        assert_eq!(json["prime_id"], 42);
        assert_eq!(json["form"], "factorial");
    }

    #[test]
    fn prime_verification_stats_serialize() {
        let stats = PrimeVerificationQueueStats {
            pending: 10,
            claimed: 3,
            verified: 50,
            failed: 2,
            total_primes: 65,
            quorum_met: 48,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["pending"], 10);
        assert_eq!(json["quorum_met"], 48);
    }

    #[test]
    fn verify_result_serde_roundtrip() {
        use crate::verify::VerifyResult;

        let cases = vec![
            VerifyResult::Verified {
                method: "Proth".to_string(),
                tier: 1,
            },
            VerifyResult::Failed {
                reason: "Composite".to_string(),
            },
            VerifyResult::Skipped {
                reason: "Too large".to_string(),
            },
        ];

        for original in cases {
            let json = serde_json::to_string(&original).unwrap();
            let deserialized: VerifyResult = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", original), format!("{:?}", deserialized));
        }
    }
}
