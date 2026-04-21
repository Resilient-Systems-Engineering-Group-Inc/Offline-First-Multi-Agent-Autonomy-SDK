//! JWT token generation and validation.

use crate::Claims;
use anyhow::{Result, anyhow};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use tracing::{info, warn};

/// JWT handler.
pub struct JwtHandler {
    secret: String,
}

impl JwtHandler {
    /// Create new JWT handler.
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
        }
    }

    /// Encode claims into JWT token.
    pub fn encode(&self, claims: &Claims) -> Result<String> {
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to encode JWT: {}", e))
    }

    /// Decode and validate JWT token.
    pub fn decode(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            warn!("JWT decode error: {}", e);
            anyhow!("Invalid token: {}", e)
        })?;

        Ok(token_data.claims)
    }

    /// Refresh token.
    pub fn refresh(&self, old_token: &str, new_roles: Option<Vec<String>>) -> Result<String> {
        // Validate old token
        let old_claims = self.decode(old_token)?;

        // Create new claims
        let mut new_claims = Claims::new(&old_claims.sub, &old_claims.username, old_claims.roles);
        
        if let Some(roles) = new_roles {
            new_claims.roles = roles;
        }

        // Encode new token
        self.encode(&new_claims)
    }

    /// Generate access token for user.
    pub fn generate_access_token(&self, user_id: &str, username: &str, roles: Vec<String>) -> Result<String> {
        let claims = Claims::new(user_id, username, roles);
        self.encode(&claims)
    }

    /// Validate token and extract user ID.
    pub fn validate(&self, token: &str) -> Result<String> {
        let claims = self.decode(token)?;
        
        if claims.is_expired() {
            return Err(anyhow!("Token expired"));
        }

        Ok(claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_lifecycle() {
        let handler = JwtHandler::new("test-secret");

        // Generate token
        let token = handler.generate_access_token(
            "user-123",
            "testuser",
            vec!["user".to_string()]
        ).unwrap();

        assert!(!token.is_empty());

        // Validate token
        let user_id = handler.validate(&token).unwrap();
        assert_eq!(user_id, "user-123");

        // Decode token
        let claims = handler.decode(&token).unwrap();
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.roles, vec!["user"]);

        // Invalid token should fail
        let result = handler.validate("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_refresh() {
        let handler = JwtHandler::new("test-secret");

        // Generate initial token
        let token = handler.generate_access_token(
            "user-123",
            "testuser",
            vec!["user".to_string()]
        ).unwrap();

        // Refresh token
        let new_token = handler.refresh(&token, Some(vec!["user".to_string(), "admin".to_string()])).unwrap();

        assert!(!new_token.is_empty());
        assert_ne!(token, new_token);

        // Verify new token has admin role
        let claims = handler.decode(&new_token).unwrap();
        assert!(claims.roles.contains(&"admin".to_string()));
    }
}
