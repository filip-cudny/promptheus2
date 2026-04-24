use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct AiProvider {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
}

pub const PROVIDERS: &[AiProvider] = &[
    AiProvider {
        id: "claude",
        name: "Claude",
        url: "https://claude.ai/",
    },
    AiProvider {
        id: "openai",
        name: "ChatGPT",
        url: "https://chatgpt.com/",
    },
];

pub fn find(id: &str) -> Option<&'static AiProvider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, Serialize)]
pub struct AiProviderDto {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl From<&AiProvider> for AiProviderDto {
    fn from(p: &AiProvider) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name.to_string(),
            url: p.url.to_string(),
        }
    }
}
