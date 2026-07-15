use std::collections::HashMap;

pub struct HttpClient {
    config: HttpClientConfig,
    client: reqwest::Client,
}

pub struct HttpClientConfig {
    pub timeout: std::time::Duration,
    pub connect_timeout: std::time::Duration,
    pub user_agent: String,
    pub max_retries: u32,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
            connect_timeout: std::time::Duration::from_secs(10),
            user_agent: String::from("Mozilla/5.0 (Linux; Android 14)"),
            max_retries: 3,
        }
    }
}

impl HttpClient {
    pub fn new(config: HttpClientConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to create HTTP client");
        Self { config, client }
    }

    pub async fn get(&self, url: &str) -> Result<String, String> {
        let response = self.client.get(url)
            .send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        let body = response.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        Ok(body)
    }

    pub async fn post(&self, url: &str, body: &str, content_type: &str) -> Result<String, String> {
        let response = self.client.post(url)
            .header("Content-Type", content_type)
            .body(body.to_string())
            .send().await
            .map_err(|e| format!("HTTP POST failed: {}", e))?;
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        Ok(text)
    }

    pub fn get_config(&self) -> &HttpClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = HttpClientConfig::default();
        let _client = HttpClient::new(config);
    }

    #[test]
    fn test_client_with_config() {
        let config = HttpClientConfig {
            timeout: std::time::Duration::from_secs(60),
            connect_timeout: std::time::Duration::from_secs(30),
            user_agent: "TestAgent/1.0".to_string(),
            max_retries: 5,
        };
        let client = HttpClient::new(config);
        assert_eq!(client.get_config().max_retries, 5);
    }
}