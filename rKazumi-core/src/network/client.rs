use std::time::Duration;

pub struct HttpClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub user_agent: String,
    pub max_retries: u32,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            user_agent: String::from("Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36"),
            max_retries: 3,
        }
    }
}

pub struct HttpClient {
    config: HttpClientConfig,
    client: reqwest::Client,
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
        let response = self.client.get(url).send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        let body = response.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        Ok(body)
    }
}
