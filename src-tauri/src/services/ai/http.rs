use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Response, StatusCode};

use crate::models::settings::ModelConfig;

use super::AiError;

pub struct OpenAiHttp {
    client: reqwest::Client,
    base_url: String,
}

impl OpenAiHttp {
    pub fn new(model: &ModelConfig) -> Result<Self, AiError> {
        let api_key = model.resolved_api_key().ok_or_else(|| {
            AiError::Authentication(format!(
                "no API key configured for model '{}'",
                model.display_name
            ))
        })?;

        let base_url = model
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1")
            .trim_end_matches('/')
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| AiError::Request(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AiError::Request(e.to_string()))?;

        Ok(Self { client, base_url })
    }

    pub async fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<Response, AiError> {
        let mut request = self
            .client
            .post(format!("{}{path}", self.base_url))
            .json(body);
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }

        let response = request.send().await.map_err(|e| {
            if e.is_connect() {
                AiError::Connection("connection failed — check your internet".into())
            } else {
                AiError::Request(e.to_string())
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(map_http_error(status, &body_text));
        }
        Ok(response)
    }
}

fn map_http_error(status: StatusCode, body: &str) -> AiError {
    match status.as_u16() {
        401 => AiError::Authentication("API key is invalid or expired".into()),
        429 => AiError::RateLimit,
        status => AiError::ApiStatus {
            status,
            message: body.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_status_to_error() {
        assert!(matches!(
            map_http_error(StatusCode::UNAUTHORIZED, "x"),
            AiError::Authentication(_)
        ));
        assert!(matches!(
            map_http_error(StatusCode::TOO_MANY_REQUESTS, "x"),
            AiError::RateLimit
        ));
        assert!(matches!(
            map_http_error(StatusCode::BAD_GATEWAY, "down"),
            AiError::ApiStatus { status: 502, message } if message == "down"
        ));
        assert!(matches!(
            map_http_error(StatusCode::BAD_REQUEST, "bad"),
            AiError::ApiStatus { status: 400, message } if message == "bad"
        ));
    }
}
