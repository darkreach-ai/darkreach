//! User profile queries — role lookup, profile CRUD, operator auto-provisioning.

use anyhow::Result;
use serde::Serialize;
use tracing::info;

use super::Database;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserProfile {
    pub id: String,
    pub role: String,
    pub operator_id: Option<uuid::Uuid>,
    pub display_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    /// Look up a user profile by Supabase auth user ID.
    pub async fn get_user_profile(&self, user_id: &str) -> Result<Option<UserProfile>> {
        let row = sqlx::query_as::<_, UserProfile>(
            "SELECT id::text, role, operator_id, display_name, created_at, updated_at
             FROM user_profiles WHERE id = $1::uuid",
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row)
    }

    /// Get the role for a user (returns "operator" as default if no profile exists).
    pub async fn get_user_role(&self, user_id: &str) -> Result<String> {
        let role =
            sqlx::query_scalar::<_, String>("SELECT role FROM user_profiles WHERE id = $1::uuid")
                .bind(user_id)
                .fetch_optional(self.pool())
                .await?;
        Ok(role.unwrap_or_else(|| "operator".to_string()))
    }

    /// Update a user's display name.
    pub async fn update_user_display_name(&self, user_id: &str, display_name: &str) -> Result<()> {
        sqlx::query(
            "UPDATE user_profiles SET display_name = $2, updated_at = NOW()
             WHERE id = $1::uuid",
        )
        .bind(user_id)
        .bind(display_name)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Link a user profile to an operator account.
    pub async fn link_user_to_operator(
        &self,
        user_id: &str,
        operator_id: uuid::Uuid,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE user_profiles SET operator_id = $2, updated_at = NOW()
             WHERE id = $1::uuid",
        )
        .bind(user_id)
        .bind(operator_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Auto-provision an operator account for a user on first login.
    ///
    /// Handles three cases:
    /// 1. Profile exists with operator_id → return as-is
    /// 2. Profile exists without operator_id → find/create operator, link
    /// 3. No profile → create profile + operator + trust record
    ///
    /// If an operator with the same email already exists (e.g. from v1 manual
    /// registration), links to that existing operator instead of creating a new one.
    pub async fn provision_operator_for_user(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<UserProfile> {
        // Check if profile already exists with an operator linked
        if let Some(profile) = self.get_user_profile(user_id).await? {
            if profile.operator_id.is_some() {
                return Ok(profile);
            }
            // Profile exists but no operator — find or create one
            let operator_id = self.find_or_create_operator(email).await?;
            self.link_user_to_operator(user_id, operator_id).await?;
            info!(user_id, %operator_id, "linked existing profile to operator");
            // Re-fetch to get updated profile
            return self
                .get_user_profile(user_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("profile disappeared after link"));
        }

        // No profile at all — create both
        let operator_id = self.find_or_create_operator(email).await?;
        sqlx::query(
            "INSERT INTO user_profiles (id, role, operator_id, display_name)
             VALUES ($1::uuid, 'operator', $2, NULL)
             ON CONFLICT (id) DO UPDATE SET operator_id = $2, updated_at = NOW()",
        )
        .bind(user_id)
        .bind(operator_id)
        .execute(self.pool())
        .await?;
        info!(user_id, %operator_id, "created new profile with operator");

        self.get_user_profile(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profile missing after creation"))
    }

    /// Find an existing operator by email, or create a new one.
    /// Handles username collisions with a numeric suffix retry.
    async fn find_or_create_operator(&self, email: &str) -> Result<uuid::Uuid> {
        // Check for existing operator with this email
        let existing: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT id FROM operators WHERE email = $1")
                .bind(email)
                .fetch_optional(self.pool())
                .await?;
        if let Some(id) = existing {
            return Ok(id);
        }

        // Derive username from email prefix
        let base_username = email
            .split('@')
            .next()
            .unwrap_or("user")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>();
        let base_username = if base_username.is_empty() {
            "user".to_string()
        } else {
            base_username
        };

        // Try inserting with the base username, retry with numeric suffix on conflict
        for suffix in 0..100 {
            let username = if suffix == 0 {
                base_username.clone()
            } else {
                format!("{}{}", base_username, suffix)
            };
            let result = sqlx::query_as::<_, (uuid::Uuid,)>(
                "INSERT INTO operators (username, email)
                 VALUES ($1, $2)
                 ON CONFLICT (username) DO NOTHING
                 RETURNING id",
            )
            .bind(&username)
            .bind(email)
            .fetch_optional(self.pool())
            .await?;

            if let Some((id,)) = result {
                // Initialize trust record
                sqlx::query(
                    "INSERT INTO operator_trust (volunteer_id) VALUES ($1)
                     ON CONFLICT DO NOTHING",
                )
                .bind(id)
                .execute(self.pool())
                .await?;
                info!(%id, username, email, "auto-provisioned new operator");
                return Ok(id);
            }
        }

        anyhow::bail!("failed to create operator: username collision after 100 retries")
    }
}
