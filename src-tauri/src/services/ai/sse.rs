use std::pin::Pin;

use futures::Stream;
use tokio_stream::StreamExt;

pub fn parse_sse_stream(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>> {
    let stream = async_stream(byte_stream);
    Box::pin(stream)
}

fn async_stream(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<String, reqwest::Error>> + Send {
    futures::stream::unfold(
        (Box::pin(byte_stream), String::new()),
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
                        if !buffer.trim().is_empty() {
                            if let Some(data) = buffer.trim().strip_prefix("data: ") {
                                if data != "[DONE]" {
                                    buffer.clear();
                                    return Some((Ok(data.to_string()), (byte_stream, buffer)));
                                }
                            }
                        }
                        return None;
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    fn bytes_stream(
        chunks: Vec<&str>,
    ) -> impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static {
        stream::iter(
            chunks
                .into_iter()
                .map(|s| Ok(bytes::Bytes::from(s.to_string())))
                .collect::<Vec<_>>(),
        )
    }

    #[tokio::test]
    async fn test_basic_sse_parsing() {
        let input = bytes_stream(vec!["data: hello\n\ndata: world\n\n"]);
        let results: Vec<_> = tokio_stream::StreamExt::collect(parse_sse_stream(input)).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap(), "hello");
        assert_eq!(results[1].as_ref().unwrap(), "world");
    }

    #[tokio::test]
    async fn test_done_terminates_stream() {
        let input = bytes_stream(vec!["data: first\n\ndata: [DONE]\n\n"]);
        let results: Vec<_> = tokio_stream::StreamExt::collect(parse_sse_stream(input)).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), "first");
    }

    #[tokio::test]
    async fn test_skips_comments_and_empty_lines() {
        let input = bytes_stream(vec![": comment\n\n\ndata: value\n\n"]);
        let results: Vec<_> = tokio_stream::StreamExt::collect(parse_sse_stream(input)).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), "value");
    }

    #[tokio::test]
    async fn test_buffering_across_chunks() {
        let input = bytes_stream(vec!["dat", "a: split", "\n\n"]);
        let results: Vec<_> = tokio_stream::StreamExt::collect(parse_sse_stream(input)).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), "split");
    }

    #[tokio::test]
    async fn test_json_payload() {
        let input = bytes_stream(vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
        ]);
        let results: Vec<_> = tokio_stream::StreamExt::collect(parse_sse_stream(input)).await;
        assert_eq!(results.len(), 1);
        let payload = results[0].as_ref().unwrap();
        assert!(payload.contains("\"content\":\"hi\""));
    }
}
