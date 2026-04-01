//! REST API client for external service integration.

use crate::error::{ApiIntegrationError, Result};
use reqwest::{Client, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, info};

/// Configuration for REST client.
#[derive(Debug, Clone)]
pub struct RestClientConfig {
    /// Base URL for API requests.
    pub base_url: String,
    /// Default timeout for requests.
    pub timeout_seconds: u64,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Retry delay in milliseconds.
    pub retry_delay_ms: u64,
    /// Authentication token (optional).
    pub auth_token: Option<String>,
    /// Additional headers.
    pub headers: Vec<(String, String)>,
}

impl Default for RestClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            auth_token: None,
            headers: Vec::new(),
        }
    }
}

/// REST client for making HTTP requests.
pub struct RestClient {
    client: Client,
    config: RestClientConfig,
}

impl RestClient {
    /// Create a new REST client with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(RestClientConfig::default())
    }

    /// Create a new REST client with custom configuration.
    pub fn with_config(config: RestClientConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ApiIntegrationError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Create a new REST client with a base URL.
    pub fn with_base_url(base_url: &str) -> Result<Self> {
        let config = RestClientConfig {
            base_url: base_url.to_string(),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Make a GET request.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request::<(), T>("GET", path, None).await
    }

    /// Make a POST request.
    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        self.request("POST", path, Some(body)).await
    }

    /// Make a PUT request.
    pub async fn put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        self.request("PUT", path, Some(body)).await
    }

    /// Make a PATCH request.
    pub async fn patch<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        self.request("PATCH", path, Some(body)).await
    }

    /// Make a DELETE request.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request::<(), T>("DELETE", path, None).await
    }

    /// Make a generic HTTP request with retry logic.
    async fn request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        let url = format!("{}{}", self.config.base_url, path);
        let mut retries = 0;

        loop {
            match self.make_request(method, &url, body).await {
                Ok(response) => {
                    let status = response.status();
                    
                    if status.is_success() {
                        let response_body = response
                            .json::<T>()
                            .await
                            .map_err(ApiIntegrationError::HttpRequestError)?;
                        
                        debug!("{} {} succeeded with status {}", method, path, status);
                        return Ok(response_body);
                    } else if status.is_server_error() && retries < self.config.max_retries {
                        retries += 1;
                        error!(
                            "{} {} failed with status {} (retry {}/{})",
                            method, path, status, retries, self.config.max_retries
                        );
                        
                        if retries < self.config.max_retries {
                            tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                            continue;
                        }
                    }
                    
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Failed to read error response".to_string());
                    
                    return Err(ApiIntegrationError::InvalidResponse(format!(
                        "{} {} failed with status {}: {}",
                        method, path, status, error_text
                    )));
                }
                Err(e) => {
                    if retries < self.config.max_retries {
                        retries += 1;
                        error!(
                            "{} {} failed with error: {} (retry {}/{})",
                            method, path, e, retries, self.config.max_retries
                        );
                        
                        if retries < self.config.max_retries {
                            tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                            continue;
                        }
                    }
                    
                    return Err(e);
                }
            }
        }
    }

    /// Make a single HTTP request without retry logic.
    async fn make_request<B: Serialize>(
        &self,
        method: &str,
        url: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let mut request = self.client.request(
            method.parse().map_err(|e| {
                ApiIntegrationError::Internal(format!("Invalid HTTP method {}: {}", method, e))
            })?,
            url,
        );

        // Add authentication header if token is provided
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add custom headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add body if provided
        if let Some(body) = body {
            request = request.json(body);
        }

        debug!("Making {} request to {}", method, url);
        
        let response = request
            .send()
            .await
            .map_err(ApiIntegrationError::HttpRequestError)?;

        Ok(response)
    }

    /// Set authentication token.
    pub fn set_auth_token(&mut self, token: &str) {
        self.config.auth_token = Some(token.to_string());
    }

    /// Add a custom header.
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.config.headers.push((key.to_string(), value.to_string()));
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Make a raw request and return the response.
    pub async fn raw_request<B: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let url = format!("{}{}", self.config.base_url, path);
        self.make_request(method, &url, body).await
    }
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default REST client")
    }
}

/// REST API adapter for specific services.
pub trait RestApiAdapter: Send + Sync {
    /// Get the base URL for the API.
    fn base_url(&self) -> &str;

    /// Make a request to the API.
    async fn request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T>;
}

/// Simple REST API adapter implementation.
pub struct SimpleRestAdapter {
    client: RestClient,
}

impl SimpleRestAdapter {
    /// Create a new simple REST adapter.
    pub fn new(base_url: &str) -> Result<Self> {
        let client = RestClient::with_base_url(base_url)?;
        Ok(Self { client })
    }

    /// Get the underlying REST client.
    pub fn client(&self) -> &RestClient {
        &self.client
    }

    /// Get the underlying REST client mutably.
    pub fn client_mut(&mut self) -> &mut RestClient {
        &mut self.client
    }
}

impl RestApiAdapter for SimpleRestAdapter {
    fn base_url(&self) -> &str {
        self.client.base_url()
    }

    async fn request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        self.client.request(method, path, body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_rest_client_get() {
        let mock_server = MockServer::start().await;
        
        // Mock a GET endpoint
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"message": "Hello"})))
            .mount(&mock_server)
            .await;
        
        let client = RestClient::with_base_url(&mock_server.uri()).unwrap();
        let response: serde_json::Value = client.get("/test").await.unwrap();
        
        assert_eq!(response["message"], "Hello");
    }

    #[tokio::test]
    async fn test_rest_client_post() {
        let mock_server = MockServer::start().await;
        
        // Mock a POST endpoint
        Mock::given(method("POST"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 123})))
            .mount(&mock_server)
            .await;
        
        let client = RestClient::with_base_url(&mock_server.uri()).unwrap();
        let body = json!({"name": "Test"});
        let response: serde_json::Value = client.post("/test", &body).await.unwrap();
        
        assert_eq!(response["id"], 123);
    }

    #[tokio::test]
    async fn test_rest_client_with_auth() {
        let mock_server = MockServer::start().await;
        
        // Mock an endpoint that requires authentication
        Mock::given(method("GET"))
            .and(path("/secure"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"secure": true})))
            .mount(&mock_server)
            .await;
        
        let mut client = RestClient::with_base_url(&mock_server.uri()).unwrap();
        client.set_auth_token("test-token");
        
        // Note: This test doesn't verify the auth header is sent,
        // but it verifies the request succeeds
        let response: serde_json::Value = client.get("/secure").await.unwrap();
        assert_eq!(response["secure"], true);
    }
}