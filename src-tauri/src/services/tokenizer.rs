use std::sync::OnceLock;

use crate::models::settings::Provider;

static OPENAI_BPE: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();

fn openai_bpe() -> &'static tiktoken_rs::CoreBPE {
    OPENAI_BPE.get_or_init(|| tiktoken_rs::o200k_base().expect("failed to load o200k_base BPE"))
}

static GEMINI_TOKENIZER: OnceLock<gemini_tokenizer::LocalTokenizer> = OnceLock::new();

fn gemini_tok() -> &'static gemini_tokenizer::LocalTokenizer {
    GEMINI_TOKENIZER.get_or_init(|| {
        gemini_tokenizer::LocalTokenizer::new("gemini-2.0-flash")
            .expect("failed to load Gemini tokenizer")
    })
}

pub fn count_tokens(text: &str, provider: &Provider) -> usize {
    if text.is_empty() {
        return 0;
    }

    match provider {
        Provider::Openai | Provider::ElevenLabs => {
            openai_bpe().encode_with_special_tokens(text).len()
        }
        Provider::Anthropic => claude_tokenizer::count_tokens(text).unwrap_or(0),
        Provider::Gemini => gemini_tok().count_tokens(text, None).total_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_polish() {
        let text = "Cześć, jak się masz?";
        let openai = count_tokens(text, &Provider::Openai);
        let anthropic = count_tokens(text, &Provider::Anthropic);
        let gemini = count_tokens(text, &Provider::Gemini);

        assert!(openai > 0);
        assert!(anthropic > 0);
        assert!(gemini > 0);
    }

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens("", &Provider::Openai), 0);
        assert_eq!(count_tokens("", &Provider::Anthropic), 0);
        assert_eq!(count_tokens("", &Provider::Gemini), 0);
    }

    #[test]
    fn test_count_tokens_english() {
        let text = "Hello world";
        let count = count_tokens(text, &Provider::Openai);
        assert_eq!(count, 2);
    }
}
