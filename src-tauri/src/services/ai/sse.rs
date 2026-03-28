use std::pin::Pin;

use futures::{Stream, StreamExt};
use reqwest::Response;

pub fn parse_sse_stream(
    response: Response,
) -> Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>> {
    let byte_stream = response.bytes_stream().map(|r| r.map(|b| b.to_vec()));
    sse_stream(byte_stream)
}

fn sse_stream(
    byte_stream: impl Stream<Item = Result<Vec<u8>, reqwest::Error>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>> {
    Box::pin(futures::stream::unfold(
        (Box::pin(byte_stream) as Pin<Box<dyn Stream<Item = Result<Vec<u8>, reqwest::Error>> + Send>>, String::new()),
        |(mut byte_stream, mut buffer)| async move {
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim_end_matches('\r').to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return None;
                        }
                        return Some((Ok(data.to_string()), (byte_stream, buffer)));
                    }

                    continue;
                }

                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Some(Err(e)) => {
                        return Some((Err(e), (byte_stream, buffer)));
                    }
                    None => {
                        let trimmed = buffer.trim().to_string();
                        if let Some(data) = trimmed.strip_prefix("data: ") {
                            if data != "[DONE]" {
                                let result = data.to_string();
                                buffer.clear();
                                return Some((Ok(result), (byte_stream, buffer)));
                            }
                        }
                        return None;
                    }
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    fn test_sse_stream(
        chunks: Vec<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>> {
        let byte_stream = stream::iter(
            chunks
                .into_iter()
                .map(|s| Ok::<Vec<u8>, reqwest::Error>(s.as_bytes().to_vec()))
                .collect::<Vec<_>>(),
        );
        Box::pin(sse_stream(byte_stream))
    }

    #[tokio::test]
    async fn test_basic_sse_parsing() {
        let results: Vec<_> =
            tokio_stream::StreamExt::collect(test_sse_stream(vec!["data: hello\n\ndata: world\n\n"]))
                .await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap(), "hello");
        assert_eq!(results[1].as_ref().unwrap(), "world");
    }

    #[tokio::test]
    async fn test_done_terminates_stream() {
        let results: Vec<_> =
            tokio_stream::StreamExt::collect(test_sse_stream(vec!["data: first\n\ndata: [DONE]\n\n"]))
                .await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), "first");
    }

    #[tokio::test]
    async fn test_skips_comments_and_empty_lines() {
        let results: Vec<_> =
            tokio_stream::StreamExt::collect(test_sse_stream(vec![": comment\n\n\ndata: value\n\n"]))
                .await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), "value");
    }

    #[tokio::test]
    async fn test_buffering_across_chunks() {
        let results: Vec<_> =
            tokio_stream::StreamExt::collect(test_sse_stream(vec!["dat", "a: split", "\n\n"]))
                .await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), "split");
    }

    #[tokio::test]
    async fn test_json_payload() {
        let results: Vec<_> = tokio_stream::StreamExt::collect(test_sse_stream(vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
        ]))
        .await;
        assert_eq!(results.len(), 1);
        let payload = results[0].as_ref().unwrap();
        assert!(payload.contains("\"content\":\"hi\""));
    }
}
