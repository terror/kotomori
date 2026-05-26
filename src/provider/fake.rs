use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Fake;

#[async_trait]
impl Provider for Fake {
  async fn stream(&self, request: Request, sink: &mut ProviderSink) -> Result {
    let input = request
      .last_user_message()
      .and_then(Message::content)
      .unwrap_or_default();

    let response = format!("queued for {}: {input}", request.model());

    for c in response.chars() {
      sink.delta(c.to_string())?;
      sleep(Duration::from_millis(20)).await;
    }

    Ok(())
  }
}
