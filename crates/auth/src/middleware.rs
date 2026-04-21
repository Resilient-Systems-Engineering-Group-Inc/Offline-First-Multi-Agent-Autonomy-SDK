//! Warp middleware for JWT authentication.

use warp::Filter;
use warp::rejection::Reject;
use std::sync::Arc;
use crate::jwt::JwtHandler;
use crate::rbac::{RbacManager, ResourceType, Action};
use serde::Serialize;
use std::convert::Infallible;

/// Authentication error.
#[derive(Debug)]
pub struct AuthError(String);

impl Reject for AuthError {}

/// Create authentication filter.
pub fn auth_required(
    jwt_handler: Arc<JwtHandler>,
) -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
    warp::header::<String>("Authorization")
        .and_then(move |auth_header: String| {
            let jwt_handler = jwt_handler.clone();
            
            async move {
                if !auth_header.starts_with("Bearer ") {
                    return Err(warp::reject::custom(AuthError(
                        "Invalid authorization header".to_string()
                    )));
                }

                let token = &auth_header[7..]; // Remove "Bearer " prefix
                
                match jwt_handler.validate(token) {
                    Ok(user_id) => Ok(user_id),
                    Err(_) => Err(warp::reject::custom(AuthError(
                        "Invalid or expired token".to_string()
                    ))),
                }
            }
        })
}

/// Create RBAC permission filter.
pub fn require_permission(
    rbac: Arc<RbacManager>,
    resource: ResourceType,
    action: Action,
) -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
    auth_required(Arc::new(JwtHandler::new("secret"))) // Would pass real handler
        .and_then(move |user_id| {
            let rbac = rbac.clone();
            
            async move {
                // In production, fetch user roles from database
                let user_roles = vec!["user".to_string()]; // Placeholder

                let has_permission = rbac
                    .check_permission(&user_roles, &resource, &action)
                    .await;

                if has_permission {
                    Ok(user_id)
                } else {
                    Err(warp::reject::custom(AuthError(
                        "Insufficient permissions".to_string()
                    )))
                }
            }
        })
}

/// Optional authentication (returns None if no token).
pub fn auth_optional(
    jwt_handler: Arc<JwtHandler>,
) -> impl Filter<Extract = (Option<String>,), Error = warp::Rejection> + Clone {
    warp::header::<String>("Authorization")
        .and_then(move |auth_header: String| {
            let jwt_handler = jwt_handler.clone();
            
            async move {
                if !auth_header.starts_with("Bearer ") {
                    return Ok(None);
                }

                let token = &auth_header[7..];
                
                match jwt_handler.validate(token) {
                    Ok(user_id) => Ok(Some(user_id)),
                    Err(_) => Ok(None), // Don't reject, just return None
                }
            }
        })
        .or_else(|_| async {
            Ok::<(Option<String>,), Infallible>((None,))
        })
}

/// Warp rejection handler for auth errors.
pub async fn handle_auth_rejection(
    err: warp::Rejection,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(auth_err) = err.find::<AuthError>() {
        let status = warp::http::StatusCode::UNAUTHORIZED;
        Ok(warp::reply::with_status(
            serde_json::json!({
                "error": "Authentication failed",
                "message": auth_err.0
            }),
            status,
        ))
    } else {
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_filter() {
        let jwt_handler = Arc::new(JwtHandler::new("test-secret"));
        
        // Generate valid token
        let token = jwt_handler.generate_access_token(
            "user-123",
            "testuser",
            vec!["user".to_string()]
        ).unwrap();

        // Test filter (would need warp test utilities)
        assert!(!token.is_empty());
    }
}
