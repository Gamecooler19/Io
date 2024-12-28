use std::{time::Duration, collections::HashMap};
use reqwest::{Client, Method, StatusCode, header};
use serde::{Serialize, Deserialize};
use crate::{Result, error::IoError};

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    timeout: Duration,
    max_retries: u32,
    retry_delay: Duration,
    user_agent: String,
    default_headers: HashMap<String, String>,
}

pub struct HttpClient {
    client: Client,
    config: HttpClientConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestBuilder {
    method: Method,
    url: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    body: Option<String>,
    timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpClient {
    pub fn new(config: HttpClientConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .default_headers(Self::convert_headers(&config.default_headers))
            .build()
            .map_err(|e| IoError::runtime_error(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    pub async fn request(&self, builder: RequestBuilder) -> Result<Response> {
        let mut attempts = 0;
        loop {
            match self.execute_request(&builder).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.config.max_retries {
                        return Err(e);
                    }
                    tokio::time::sleep(self.config.retry_delay).await;
                }
            }
        }
    }

    async fn execute_request(&self, builder: &RequestBuilder) -> Result<Response> {
        let mut req = self.client
            .request(builder.method.clone(), &builder.url)
            .headers(Self::convert_headers(&builder.headers));

        if !builder.query_params.is_empty() {
            req = req.query(&builder.query_params);
        }

        if let Some(body) = &builder.body {
            req = req.body(body.clone());
        }

        if let Some(timeout) = builder.timeout {
            req = req.timeout(timeout);
        }

        let response = req.send().await
            .map_err(|e| IoError::runtime_error(format!("Request failed: {}", e)))?;

        let status = response.status().as_u16();
        let headers = Self::extract_headers(response.headers());
        let body = response.text().await
            .map_err(|e| IoError::runtime_error(format!("Failed to read response body: {}", e)))?;

        Ok(Response { status, headers, body })
    }

    fn convert_headers(headers: &HashMap<String, String>) -> header::HeaderMap {
        let mut header_map = header::HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(name), Ok(val)) = (
                header::HeaderName::from_bytes(key.as_bytes()),
                header::HeaderValue::from_str(value)
            ) {
                header_map.insert(name, val);
            }
        }
        header_map
    }

    fn extract_headers(headers: &header::HeaderMap) -> HashMap<String, String> {
        headers.iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
            })
            .collect()
    }
}
