use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Ollama {
  client: Client,
  url: String,
}

impl Ollama {
  fn handle_line(line: &str, sink: &ProviderSink) -> Result {
    if line.is_empty() {
      return Ok(());
    }

    let response = serde_json::from_str::<ChatResponse>(line)?;

    let content = response
      .message
      .and_then(|message| message.content)
      .filter(|content| !content.is_empty());

    if let Some(content) = content {
      sink.delta(content)?;
    }

    Ok(())
  }

  pub(crate) fn new() -> Self {
    Self {
      client: Client::new(),
      url: format!(
        "{}/api/chat",
        env::var("OLLAMA_HOST")
          .unwrap_or_else(|_| "http://localhost:11434".into())
      ),
    }
  }
}

#[async_trait]
impl Provider for Ollama {
  async fn stream(
    &self,
    request: CompletionRequest,
    sink: ProviderSink,
  ) -> Result {
    let request = ChatRequest::from(request);

    let response = self
      .client
      .post(&self.url)
      .json(&request)
      .send()
      .await
      .with_context(|| format!("failed to connect to `{}`", self.url))?
      .error_for_status()?;

    let stream = response.bytes_stream();

    pin_mut!(stream);

    let mut buffer = Vec::new();

    while let Some(chunk) = stream.next().await {
      let chunk = chunk?;

      buffer.extend_from_slice(&chunk);

      while let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
        let line = buffer.drain(..=index).collect::<Vec<_>>();

        Self::handle_line(
          str::from_utf8(&line[..line.len() - 1])?.trim(),
          &sink,
        )?;
      }
    }

    Self::handle_line(str::from_utf8(&buffer)?.trim(), &sink)?;

    Ok(())
  }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
  messages: Vec<ChatMessage>,
  model: String,
  stream: bool,
}

impl From<CompletionRequest> for ChatRequest {
  fn from(request: CompletionRequest) -> Self {
    Self {
      messages: request
        .messages()
        .iter()
        .map(ChatMessage::from)
        .collect::<Vec<_>>(),
      model: request.model().name().into(),
      stream: true,
    }
  }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
  message: Option<ChatResponseMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
  content: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
  content: String,
  role: String,
}

impl From<&Message> for ChatMessage {
  fn from(message: &Message) -> Self {
    Self {
      content: message.content.clone(),
      role: match message.role {
        Role::Agent => "assistant".into(),
        Role::User => "user".into(),
      },
    }
  }
}
