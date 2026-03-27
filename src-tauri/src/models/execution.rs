use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NoActivePrompt,
    ExecutionInProgress,
    ClipboardError,
    ApiError,
    ValidationError,
    TimeoutError,
    UnknownError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_code: Option<ErrorCode>,
    #[serde(default)]
    pub execution_time: Option<f64>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub execution_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_error_code_serialization() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::NoActivePrompt).unwrap(),
            "\"no_active_prompt\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::ExecutionInProgress).unwrap(),
            "\"execution_in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::UnknownError).unwrap(),
            "\"unknown_error\""
        );
    }
}
