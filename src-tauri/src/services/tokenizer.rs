use std::sync::OnceLock;

static OPENAI_BPE: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();

fn openai_bpe() -> &'static tiktoken_rs::CoreBPE {
    OPENAI_BPE.get_or_init(|| tiktoken_rs::o200k_base().expect("failed to load o200k_base BPE"))
}

pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    openai_bpe().encode_with_special_tokens(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_polish() {
        assert!(count_tokens("Cześć, jak się masz?") > 0);
    }

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_count_tokens_english() {
        assert_eq!(count_tokens("Hello world"), 2);
    }
}
