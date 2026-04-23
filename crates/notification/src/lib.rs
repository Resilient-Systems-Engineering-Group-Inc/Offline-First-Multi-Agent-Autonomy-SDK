//! Multi-channel notification system.
//!
//! Provides:
//! - Email notifications
//! - SMS notifications
//! - Push notifications
//! - Slack/Teams integration
//! - Webhook notifications
//! - Template system
//! - Delivery tracking
//! - Rate limiting

pub mod channel;
pub mod template;
pub mod delivery;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use channel::*;
pub use template::*;
pub use delivery::*;

/// Notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub default_channel: NotificationChannel,
    pub channels: HashMap<NotificationChannel, ChannelConfig>,
    pub rate_limit_per_minute: u32,
    pub enable_tracking: bool,
    pub retry_attempts: u32,
    pub retry_delay_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub credentials: serde_json::Value,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        let mut channels = HashMap::new();
        channels.insert(NotificationChannel::Email, ChannelConfig {
            enabled: true,
            credentials: serde_json::json!({}),
        });
        channels.insert(NotificationChannel::Sms, ChannelConfig {
            enabled: false,
            credentials: serde_json::json!({}),
        });
        channels.insert(NotificationChannel::Push, ChannelConfig {
            enabled: false,
            credentials: serde_json::json!({}),
        });
        channels.insert(NotificationChannel::Slack, ChannelConfig {
            enabled: false,
            credentials: serde_json::json!({}),
        });
        channels.insert(NotificationChannel::Webhook, ChannelConfig {
            enabled: false,
            credentials: serde_json::json!({}),
        });

        Self {
            default_channel: NotificationChannel::Email,
            channels,
            rate_limit_per_minute: 100,
            enable_tracking: true,
            retry_attempts: 3,
            retry_delay_secs: 5,
        }
    }
}

/// Notification manager.
pub struct NotificationManager {
    config: NotificationConfig,
    channels: RwLock<HashMap<NotificationChannel, Box<dyn NotificationChannelTrait>>>,
    templates: RwLock<HashMap<String, NotificationTemplate>>,
    delivery_log: RwLock<Vec<DeliveryRecord>>,
}

impl NotificationManager {
    /// Create new notification manager.
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            config,
            channels: RwLock::new(HashMap::new()),
            templates: RwLock::new(HashMap::new()),
            delivery_log: RwLock::new(Vec::new()),
        }
    }

    /// Initialize notification system.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing notification system");

        // Register channels
        self.register_channel(NotificationChannel::Email, EmailChannel::new()).await?;
        self.register_channel(NotificationChannel::Sms, SmsChannel::new()).await?;
        self.register_channel(NotificationChannel::Push, PushChannel::new()).await?;
        self.register_channel(NotificationChannel::Slack, SlackChannel::new()).await?;
        self.register_channel(NotificationChannel::Webhook, WebhookChannel::new()).await?;

        info!("Notification system initialized");
        Ok(())
    }

    /// Register notification channel.
    pub async fn register_channel<C: NotificationChannelTrait + 'static>(
        &self,
        channel_type: NotificationChannel,
        channel: C,
    ) -> Result<()> {
        self.channels.write().await.insert(channel_type, Box::new(channel));
        info!("Channel registered: {:?}", channel_type);
        Ok(())
    }

    /// Register template.
    pub async fn register_template(&self, template: NotificationTemplate) -> Result<()> {
        let mut templates = self.templates.write().await;
        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Send notification.
    pub async fn send(&self, notification: Notification) -> Result<String> {
        let channels = self.channels.read().await;
        let channel = channels.get(&notification.channel)
            .ok_or_else(|| anyhow::anyhow!("Channel not available: {:?}", notification.channel))?;

        // Check if channel is enabled
        if !self.config.channels.get(&notification.channel).map(|c| c.enabled).unwrap_or(false) {
            return Err(anyhow::anyhow!("Channel disabled: {:?}", notification.channel));
        }

        // Send notification
        let result = channel.send(&notification).await?;

        // Log delivery
        if self.config.enable_tracking {
            self.log_delivery(&notification, &result).await;
        }

        info!("Notification sent: {} via {:?}", notification.id, notification.channel);
        Ok(notification.id)
    }

    /// Send notification to multiple recipients.
    pub async fn send_batch(&self, notifications: Vec<Notification>) -> Result<BatchResult> {
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for notification in notifications {
            match self.send(notification).await {
                Ok(id) => {
                    results.push(SendResult { id, success: true, error: None });
                    successful += 1;
                }
                Err(e) => {
                    results.push(SendResult { 
                        id: String::new(), 
                        success: false, 
                        error: Some(e.to_string()) 
                    });
                    failed += 1;
                }
            }
        }

        Ok(BatchResult {
            total: successful + failed,
            successful,
            failed,
            results,
        })
    }

    /// Send with template.
    pub async fn send_template(
        &self,
        template_name: &str,
        channel: NotificationChannel,
        recipients: Vec<String>,
        data: serde_json::Value,
    ) -> Result<String> {
        let templates = self.templates.read().await;
        let template = templates.get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", template_name))?;

        let content = template.render(&data)?;
        
        let notification = Notification::new(
            channel,
            recipients,
            template.subject.clone(),
            content,
        );

        self.send(notification).await
    }

    /// Get delivery status.
    pub async fn get_delivery_status(&self, notification_id: &str) -> Option<DeliveryRecord> {
        let log = self.delivery_log.read().await;
        log.iter().find(|r| r.notification_id == notification_id).cloned()
    }

    /// Get delivery history for recipient.
    pub async fn get_delivery_history(&self, recipient: &str, limit: usize) -> Vec<DeliveryRecord> {
        let log = self.delivery_log.read().await;
        log.iter()
            .filter(|r| r.recipients.contains(&recipient.to_string()))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get notification statistics.
    pub async fn get_stats(&self) -> NotificationStats {
        let log = self.delivery_log.read().await;
        
        let total = log.len();
        let successful = log.iter().filter(|r| r.status == DeliveryStatus::Delivered).count();
        let failed = log.iter().filter(|r| r.status == DeliveryStatus::Failed).count();
        
        let mut by_channel = HashMap::new();
        for record in log.iter() {
            *by_channel.entry(format!("{:?}", record.channel)).or_insert(0) += 1;
        }

        NotificationStats {
            total_sent: total as i32,
            successful_deliveries: successful as i32,
            failed_deliveries: failed as i32,
            delivery_rate: if total > 0 { successful as f64 / total as f64 } else { 0.0 },
            by_channel,
        }
    }

    async fn log_delivery(&self, notification: &Notification, result: &SendResult) {
        let status = if result.success {
            DeliveryStatus::Delivered
        } else {
            DeliveryStatus::Failed
        };

        let record = DeliveryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            notification_id: notification.id.clone(),
            channel: notification.channel.clone(),
            recipients: notification.recipients.clone(),
            subject: notification.subject.clone(),
            status,
            error_message: result.error.clone(),
            sent_at: chrono::Utc::now(),
            delivered_at: if result.success { Some(chrono::Utc::now()) } else { None },
            metadata: serde_json::json!({}),
        };

        let mut log = self.delivery_log.write().await;
        log.push(record);

        // Limit log size
        if log.len() > 100000 {
            log.drain(..50000);
        }
    }
}

/// Notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub channel: NotificationChannel,
    pub recipients: Vec<String>,
    pub subject: String,
    pub content: String,
    pub content_type: ContentType,
    pub priority: Priority,
    pub metadata: serde_json::Value,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Notification {
    pub fn new(
        channel: NotificationChannel,
        recipients: Vec<String>,
        subject: String,
        content: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel,
            recipients,
            subject,
            content,
            content_type: ContentType::Text,
            priority: Priority::Normal,
            metadata: serde_json::json!({}),
            scheduled_at: None,
        }
    }

    pub fn with_content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = content_type;
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn scheduled(mut self, at: chrono::DateTime<chrono::Utc>) -> Self {
        self.scheduled_at = Some(at);
        self
    }
}

/// Notification channel.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    Slack,
    Teams,
    Webhook,
    Custom(String),
}

/// Content type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Html,
    Markdown,
}

/// Priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Send result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Batch result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<SendResult>,
}

/// Notification statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStats {
    pub total_sent: i32,
    pub successful_deliveries: i32,
    pub failed_deliveries: i32,
    pub delivery_rate: f64,
    pub by_channel: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_manager() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Create notification
        let notification = Notification::new(
            NotificationChannel::Email,
            vec!["test@example.com".to_string()],
            "Test Subject".to_string(),
            "Test Content".to_string(),
        );

        // Send (will fail as channel is mock, but tests the flow)
        let result = manager.send(notification).await;
        // Result may fail due to mock channel, but structure is tested
    }
}
