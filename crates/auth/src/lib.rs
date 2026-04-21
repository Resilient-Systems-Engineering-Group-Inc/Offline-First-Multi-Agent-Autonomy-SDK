//! Authentication and authorization module.
//!
//! Provides:
//! - JWT token management
//! - Password hashing
//! - API token management
//! - Role-based access control (RBAC)

pub mod jwt;
pub mod password;
pub mod tokens;
pub mod rbac;

use anyhow::Result;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use jwt::*;
pub use password::*;
pub use tokens::*;
pub use rbac::*;

/// Authentication configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
    pub api_token_expiry_days: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: Uuid::new_v4().to_string(),
            jwt_expiry_hours: 24,
            refresh_token_expiry_days: 30,
            api_token_expiry_days: 90,
        }
    }
}

/// Authentication result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub user_id: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Login request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Register request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

/// Token refresh request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Password reset request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub username: String,
    pub new_password: String,
}

/// JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // User ID
    pub username: String,
    pub roles: Vec<String>,
    pub exp: i64,              // Expiration time
    pub iat: i64,              // Issued at
    pub nbf: i64,              // Not before
    pub jti: String,           // JWT ID (unique identifier)
}

impl Claims {
    /// Create new claims.
    pub fn new(user_id: &str, username: &str, roles: Vec<String>) -> Self {
        let now = Utc::now();
        let expiry = now + Duration::hours(24);

        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            roles,
            exp: expiry.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    /// Check if token is expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}
