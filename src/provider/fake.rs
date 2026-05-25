use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Fake;

#[async_trait]
impl Provider for Fake {
  async fn stream(
    &self,
    request: CompletionRequest,
    sink: ProviderSink,
  ) -> Result {
    let input = request
      .messages()
      .iter()
      .rev()
      .find(|message| message.role == Role::User)
      .map(|message| message.content.as_str())
      .unwrap_or_default();

    let response = format!("queued for {}: {input}", request.model());

    for c in response.chars() {
      sink.delta(c.to_string())?;
      sleep(Duration::from_millis(20)).await;
    }

    Ok(())
  }
}
