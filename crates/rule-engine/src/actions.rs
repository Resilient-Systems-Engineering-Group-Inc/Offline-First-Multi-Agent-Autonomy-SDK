//! Rule actions.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Rule action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Set field value
    Set {
        field: String,
        value: serde_json::Value,
    },

    /// Delete field
    Delete {
        field: String,
    },

    /// Increment field value
    Increment {
        field: String,
        delta: i64,
    },

    /// Append to array
    Append {
        field: String,
        value: serde_json::Value,
    },

    /// Log message
    Log {
        level: String,
        message: String,
    },

    /// Send notification
    Notify {
        channel: String,
        recipient: String,
        template: String,
    },

    /// Trigger webhook
    Webhook {
        url: String,
        method: String,
        payload: serde_json::Value,
    },

    /// Chain to another rule
    Chain {
        rule_id: String,
    },

    /// Stop execution
    Stop,

    /// Custom action
    Custom {
        action_type: String,
        parameters: serde_json::Value,
    },
}

impl Action {
    /// Execute action on context data.
    pub async fn execute(&self, data: &mut serde_json::Value) -> Result<bool> {
        match self {
            Self::Set { field, value } => {
                set_field(data, field, value.clone());
                Ok(true)
            }

            Self::Delete { field } => {
                delete_field(data, field);
                Ok(true)
            }

            Self::Increment { field, delta } => {
                increment_field(data, field, *delta);
                Ok(true)
            }

            Self::Append { field, value } => {
                append_to_array(data, field, value.clone());
                Ok(true)
            }

            Self::Log { level, message } => {
                match level.as_str() {
                    "error" => tracing::error!("{}", message),
                    "warn" => tracing::warn!("{}", message),
                    "info" => tracing::info!("{}", message),
                    "debug" => tracing::debug!("{}", message),
                    _ => tracing::info!("{}", message),
                }
                Ok(true)
            }

            Self::Notify { channel, recipient, template } => {
                tracing::info!("Notification via {} to {}: {}", channel, recipient, template);
                // Would send actual notification
                Ok(true)
            }

            Self::Webhook { url, method, payload } => {
                tracing::info!("Webhook {} to {}: {:?}", method, url, payload);
                // Would send actual webhook
                Ok(true)
            }

            Self::Chain { rule_id } => {
                tracing::debug!("Chaining to rule: {}", rule_id);
                Ok(true)
            }

            Self::Stop => {
                tracing::debug!("Stopping rule execution");
                Ok(false) // Signal to stop
            }

            Self::Custom { action_type, parameters } => {
                tracing::debug!("Custom action {}: {:?}", action_type, parameters);
                Ok(true)
            }
        }
    }
}

/// Set field value in JSON using dot notation.
fn set_field(value: &mut serde_json::Value, path: &str, new_value: serde_json::Value) {
    let parts: Vec<&str> = path.split('.').collect();
    
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate().take(parts.len() - 1) {
        if current.is_null() {
            *current = serde_json::json!({});
        }
        
        if !current.is_object() {
            return;
        }

        if !current.as_object().unwrap().contains_key(*part) {
            current.as_object_mut().unwrap().insert(part.to_string(), serde_json::json!({}));
        }

        current = current.get_mut(*part).unwrap();
    }

    if let Some(obj) = current.as_object_mut() {
        obj.insert(parts.last().unwrap().to_string(), new_value);
    }
}

/// Delete field from JSON.
fn delete_field(value: &mut serde_json::Value, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for part in parts.iter().take(parts.len() - 1) {
        if !current.is_object() {
            return;
        }
        current = current.get_mut(*part)?;
    }

    if let Some(obj) = current.as_object_mut() {
        obj.remove(parts.last().unwrap());
    }
}

/// Increment field value.
fn increment_field(value: &mut serde_json::Value, path: &str, delta: i64) {
    if let Some(current) = get_field_mut(value, path) {
        if let Some(num) = current.as_i64() {
            *current = serde_json::json!(num + delta);
        } else if let Some(num) = current.as_f64() {
            *current = serde_json::json!(num + delta as f64);
        } else {
            *current = serde_json::json!(delta);
        }
    }
}

/// Append value to array field.
fn append_to_array(value: &mut serde_json::Value, path: &str, item: serde_json::Value) {
    let parts: Vec<&str> = path.split('.').collect();
    
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for part in parts.iter().take(parts.len() - 1) {
        if !current.is_object() {
            return;
        }
        current = current.get_mut(*part)?;
    }

    if let Some(obj) = current.as_object_mut() {
        let field = parts.last().unwrap();
        if let Some(arr) = obj.get_mut(field).and_then(|v| v.as_array_mut()) {
            arr.push(item);
        } else {
            obj.insert(field.to_string(), serde_json::json!([item]));
        }
    }
}

/// Get mutable reference to field.
fn get_field_mut<'a>(value: &'a mut serde_json::Value, path: &str) -> Option<&'a mut serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    
    if parts.is_empty() {
        return None;
    }

    let mut current = value;
    for part in parts.iter().take(parts.len() - 1) {
        if !current.is_object() {
            return None;
        }
        current = current.get_mut(*part)?;
    }

    current.get_mut(parts.last().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_action_set() {
        let action = Action::Set {
            field: "status".to_string(),
            value: serde_json::json!("approved"),
        };

        let mut data = serde_json::json!({
            "count": 10
        });

        let result = action.execute(&mut data).await.unwrap();
        assert!(result);
        assert_eq!(data["status"], "approved");
    }

    #[tokio::test]
    async fn test_action_nested_set() {
        let action = Action::Set {
            field: "user.status".to_string(),
            value: serde_json::json!("active"),
        };

        let mut data = serde_json::json!({
            "user": {
                "name": "John"
            }
        });

        action.execute(&mut data).await.unwrap();
        assert_eq!(data["user"]["status"], "active");
    }

    #[tokio::test]
    async fn test_action_increment() {
        let action = Action::Increment {
            field: "count".to_string(),
            delta: 5,
        };

        let mut data = serde_json::json!({
            "count": 10
        });

        action.execute(&mut data).await.unwrap();
        assert_eq!(data["count"], 15);
    }

    #[tokio::test]
    async fn test_action_append() {
        let action = Action::Append {
            field: "tags".to_string(),
            value: serde_json::json!("new-tag"),
        };

        let mut data = serde_json::json!({
            "tags": ["tag1", "tag2"]
        });

        action.execute(&mut data).await.unwrap();
        assert_eq!(data["tags"].as_array().unwrap().len(), 3);
    }
}
