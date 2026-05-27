use super::*;

#[derive(Clone)]
pub(crate) struct Rig {
  max_tokens: Option<u64>,
  model: RigModel,
  provider: &'static str,
}

#[derive(Clone)]
enum RigModel {
  Anthropic(anthropic::CompletionModel),
  Ollama(ollama::CompletionModel),
  OpenAi(openai::CompletionModel),
}

impl Rig {
  pub(crate) fn anthropic(model: &Model) -> Result<Self> {
    let api_key = env::var("ANTHROPIC_API_KEY")
      .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN"))
      .unwrap_or_default();

    let base_url = env::var("ANTHROPIC_BASE_URL")
      .unwrap_or_else(|_| "https://api.anthropic.com".into());

    let client = anthropic::Client::builder()
      .api_key(api_key)
      .base_url(base_url)
      .build()?;

    let model = CompletionClient::completion_model(&client, model.name());

    let max_tokens = env::var("ANTHROPIC_MAX_TOKENS")
      .ok()
      .and_then(|max_tokens| max_tokens.parse::<u64>().ok())
      .unwrap_or(4096);

    Ok(Self {
      max_tokens: Some(max_tokens),
      model: RigModel::Anthropic(model),
      provider: "anthropic",
    })
  }

  pub(crate) fn ollama(model: &Model) -> Result<Self> {
    let api_key = env::var("OLLAMA_API_KEY").unwrap_or_default();

    let base_url = env::var("OLLAMA_API_BASE_URL")
      .or_else(|_| env::var("OLLAMA_HOST"))
      .unwrap_or_else(|_| "http://localhost:11434".into())
      .trim_end_matches('/')
      .to_string();

    let client = ollama::Client::builder()
      .api_key(api_key)
      .base_url(base_url)
      .build()?;

    let model = CompletionClient::completion_model(&client, model.name());

    Ok(Self {
      max_tokens: None,
      model: RigModel::Ollama(model),
      provider: "ollama",
    })
  }

  pub(crate) fn openai(model: &Model) -> Result<Self> {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut builder = openai::CompletionsClient::builder().api_key(api_key);

    if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
      builder = builder.base_url(base_url);
    }

    let client = builder.build()?;

    let model = CompletionClient::completion_model(&client, model.name());

    Ok(Self {
      max_tokens: None,
      model: RigModel::OpenAi(model),
      provider: "openai",
    })
  }

  async fn stream_model<M>(
    &self,
    model: &M,
    request: Request,
    sink: &mut ProviderSink,
  ) -> Result
  where
    M: CompletionModel,
  {
    let mut request = CompletionRequest::from(&request);

    request.max_tokens = self.max_tokens;

    let mut stream = <M as CompletionModel>::stream(model, request).await?;

    while let Some(chunk) = stream.next().await {
      match chunk? {
        StreamedAssistantContent::Text(text) if !text.text.is_empty() => {
          sink.delta(text.text)?;
        }
        StreamedAssistantContent::Reasoning(reasoning) => {
          sink.reasoning(reasoning)?;
        }
        StreamedAssistantContent::ReasoningDelta { id, reasoning }
          if !reasoning.is_empty() =>
        {
          sink.reasoning_delta(id, reasoning)?;
        }
        StreamedAssistantContent::ToolCall { tool_call, .. } => {
          sink.tool_call(tool_call.into())?;
        }
        StreamedAssistantContent::Final(_)
        | StreamedAssistantContent::ReasoningDelta { .. }
        | StreamedAssistantContent::Text(_)
        | StreamedAssistantContent::ToolCallDelta { .. } => {}
      }
    }

    Ok(())
  }
}

impl Debug for Rig {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Rig")
      .field("provider", &self.provider)
      .finish_non_exhaustive()
  }
}

#[async_trait]
impl Provider for Rig {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    match &self.model {
      RigModel::Anthropic(model) => {
        self.stream_model(model, request, sink).await
      }
      RigModel::Ollama(model) => self.stream_model(model, request, sink).await,
      RigModel::OpenAi(model) => self.stream_model(model, request, sink).await,
    }
  }
}
