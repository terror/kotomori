use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Agent {
  model: String,
}

impl Agent {
  pub(crate) fn new(model: String) -> Self {
    Self { model }
  }

  pub(crate) fn spawn(
    &self,
    input: String,
    runtime: &Runtime,
    sender: UnboundedSender<AppEvent>,
  ) {
    let model = self.model.clone();

    runtime.spawn(async move {
      let response = format!("queued for {model}: {input}");

      for c in response.chars() {
        if sender.send(Ok(Action::AgentDelta(c.to_string()))).is_err() {
          return;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
      }

      let _ = sender.send(Ok(Action::AgentDone));
    });
  }
}
