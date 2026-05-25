use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Fake;

impl Provider for Fake {
  fn stream(
    &self,
    request: CompletionRequest,
    sink: ProviderSink,
  ) -> BoxFuture<'_, Result> {
    Box::pin(async move {
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

      sink.done()?;

      Ok(())
    })
  }
}
