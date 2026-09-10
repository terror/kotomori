use super::*;

#[derive(Debug)]
pub(crate) struct Agent {
  task: Option<task::JoinHandle<()>>,
  worker: Arc<Worker>,
}

impl Agent {
  pub(crate) fn interrupt(&mut self) {
    if let Some(task) = self.task.take() {
      task.abort();
    }
  }

  pub(crate) fn new(
    event_sender: UnboundedSender<Event>,
    settings: &Settings,
  ) -> Result<Self> {
    Ok(Self {
      task: None,
      worker: Arc::new(Worker::new(event_sender, settings)?),
    })
  }

  pub(crate) fn spawn(&mut self, run_id: u64, messages: Vec<Message>) {
    self.interrupt();

    let worker = self.worker.clone();

    self.task = Some(tokio::spawn(async move {
      if let Err(error) = worker.stream(run_id, messages).await {
        let _ = worker.event_sender.send(Event::Agent {
          event: AgentEvent::Error(error.to_string()),
          run_id,
        });
      }
    }));
  }
}

impl Drop for Agent {
  fn drop(&mut self) {
    self.interrupt();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn spawning_interrupts_previous_task() {
    let (sender, mut events) = mpsc::unbounded_channel();

    let mut agent = Agent::new(
      sender,
      &Settings {
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: true,
      },
    )
    .unwrap();

    agent.spawn(0, Vec::new());
    agent.spawn(1, Vec::new());
    agent.task.take().unwrap().await.unwrap();

    for event in [
      AgentEvent::Delta("queued for mock:local: ".into()),
      AgentEvent::Message(Message::Agent(vec![AgentMessageContent::Text(
        "queued for mock:local: ".into(),
      )])),
      AgentEvent::Done,
    ] {
      assert_eq!(
        events.try_recv().unwrap(),
        Event::Agent { event, run_id: 1 }
      );
    }

    assert!(events.is_empty());
  }
}
