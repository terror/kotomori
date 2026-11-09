use super::*;

#[derive(Debug, Default, PartialEq)]
pub struct Session {
  conversation: Conversation,
  entries: Vec<SessionEntry>,
  run: Option<(RunId, usize)>,
}

impl Session {
  pub fn append(&mut self, entry: impl Into<SessionEntry>) {
    let entry = entry.into();

    if let Some(message) = entry.clone().into_message() {
      self.conversation.push(message);
    } else if matches!(entry, SessionEntry::Run(RunEntry::Message(_))) {
      return;
    }

    self.entries.push(entry);
  }

  pub fn checkpoint(&self, run: &AgentRun) -> Result<SessionSnapshot> {
    let received = self.received(run.id())?;

    let mut entries = self.entries.clone();

    entries.extend(
      run.state().entries[received..]
        .iter()
        .cloned()
        .map(SessionEntry::Run),
    );

    if !run.state().message.is_empty() {
      entries.push(SessionEntry::Run(RunEntry::Draft(
        run.state().message.clone(),
      )));
    }

    SessionSnapshot::new(entries)
  }

  pub fn clear(&mut self) {
    self.conversation.clear();
    self.entries.clear();
    self.run = None;
  }

  #[must_use]
  pub fn conversation(&self) -> &Conversation {
    &self.conversation
  }

  #[must_use]
  pub fn entries(&self) -> &[SessionEntry] {
    &self.entries
  }

  pub fn finish(&mut self, id: RunId, result: &RunResult) -> Result {
    self.import(id, &result.entries)?;
    self.run = None;

    ensure!(
      self.conversation == result.conversation,
      "session journal does not match the completed run"
    );

    Ok(())
  }

  pub fn fork(&self) -> Result<Self> {
    ensure!(self.run.is_none(), "cannot fork a running session");

    Ok(self.snapshot()?.into())
  }

  fn import(&mut self, id: RunId, entries: &[RunEntry]) -> Result {
    let received = self.received(id)?;

    for entry in entries
      .get(received..)
      .context("run state has fewer entries than the session journal")?
    {
      self.append(entry.clone());
    }

    self.run = Some((id, entries.len()));

    Ok(())
  }

  pub fn interrupt_pending_calls(&mut self) -> Result {
    self.entries.extend(
      self
        .conversation
        .interrupt_pending_calls()?
        .into_iter()
        .map(|message| SessionEntry::Run(RunEntry::Message(message))),
    );

    Ok(())
  }

  pub fn observe(&mut self, run: &AgentRun) -> Result {
    self.import(run.id(), &run.state().entries)
  }

  pub fn prompt(&mut self, text: impl Into<String>) -> Result {
    ensure!(self.run.is_none(), "session already has an active run");

    self.interrupt_pending_calls()?;
    self.append(RunEntry::Message(Message::User(vec![
      UserMessageContent::Text(text.into()),
    ])));

    Ok(())
  }

  fn received(&self, id: RunId) -> Result<usize> {
    let (current, received) =
      self.run.context("session does not have an active run")?;

    ensure!(current == id, "run does not belong to this session");

    Ok(received)
  }

  pub fn snapshot(&self) -> Result<SessionSnapshot> {
    SessionSnapshot::new(self.entries.clone())
  }

  pub fn start(&mut self, agent: &Agent) -> Result<AgentRun> {
    self.start_with(|conversation| agent.spawn(conversation))
  }

  fn start_with(
    &mut self,
    spawn: impl FnOnce(Conversation) -> AgentRun,
  ) -> Result<AgentRun> {
    ensure!(self.run.is_none(), "session already has an active run");

    self.interrupt_pending_calls()?;

    let run = spawn(self.conversation.clone());

    self.run = Some((run.id(), 0));

    Ok(run)
  }

  pub fn start_with_deadline(
    &mut self,
    agent: &Agent,
    deadline: Instant,
  ) -> Result<AgentRun> {
    self.start_with(|conversation| {
      agent.spawn_with_deadline(conversation, deadline)
    })
  }
}

impl From<SessionSnapshot> for Session {
  fn from(snapshot: SessionSnapshot) -> Self {
    Self {
      conversation: snapshot.conversation(),
      entries: snapshot.into_entries(),
      run: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestProvider {
    pending: bool,
  }

  impl Provider for TestProvider {
    fn stream<'a>(
      &'a self,
      request: Request<'a>,
      sink: &'a ProviderSink,
    ) -> BoxFuture<'a, Result<ProviderResponse>> {
      Box::pin(async move {
        assert_eq!(
          request.messages,
          [Message::User(vec![UserMessageContent::Text("foo".into())])]
        );

        sink.update(MessageUpdate::Text {
          delta: "bar".into(),
          index: 0,
        })?;

        if self.pending {
          return pending().await;
        }

        Ok(ProviderResponse {
          message: vec![AssistantContent::text("bar")].into(),
          ..Default::default()
        })
      })
    }
  }

  #[test]
  fn custom_entries_survive_restore_and_fork() {
    let custom = CustomEntry {
      data: serde_json::json!({"foo": [1, "bar", null]}),
      kind: "foo".into(),
      namespace: "bar".into(),
      version: 42,
    };

    let mut session = Session::default();
    session.append(custom.clone());
    session.append(RunEntry::Message(Message::User(vec![
      UserMessageContent::Text("foo".into()),
    ])));

    let snapshot = session.snapshot().unwrap();
    let snapshot = serde_json::from_str::<SessionSnapshot>(
      &serde_json::to_string(&snapshot).unwrap(),
    )
    .unwrap();
    let session = Session::from(snapshot);
    let fork = session.fork().unwrap();

    assert_eq!(session, fork);
    assert_eq!(
      session.entries(),
      [
        SessionEntry::Custom(custom),
        SessionEntry::Run(RunEntry::Message(Message::User(vec![
          UserMessageContent::Text("foo".into()),
        ]))),
      ]
    );
    assert_eq!(
      session.conversation().messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[tokio::test]
  async fn observing_and_finishing_import_each_entry_once() {
    async fn case(pending: bool) {
      let agent = Agent::new(
        AgentConfig {
          generation: GenerationOptions::default(),
          max_rounds: 1,
          model: "foo:bar".parse().unwrap(),
          tools: ToolRegistry::default(),
        },
        Arc::new(TestProvider { pending }),
        String::new(),
      );

      let mut session = Session::default();
      session.prompt("foo").unwrap();

      let mut run = session.start(&agent).unwrap();
      let id = run.id();

      assert_eq!(
        session.start(&agent).unwrap_err().to_string(),
        "session already has an active run"
      );
      assert_eq!(
        session.fork().unwrap_err().to_string(),
        "cannot fork a running session"
      );

      while run.state().message.is_empty() {
        run.recv().await.unwrap();
      }

      session.observe(&run).unwrap();

      let checkpoint = session.checkpoint(&run).unwrap();
      assert_eq!(
        checkpoint.entries(),
        [
          SessionEntry::Run(RunEntry::Message(Message::User(vec![
            UserMessageContent::Text("foo".into()),
          ]))),
          SessionEntry::Run(RunEntry::Draft(run.state().message.clone())),
        ]
      );

      let custom = CustomEntry {
        data: serde_json::json!("foo"),
        kind: "bar".into(),
        namespace: "baz".into(),
        version: 1,
      };

      session.append(custom.clone());

      if pending {
        run.cancel();
      }

      while run.recv().await.is_some() {
        session.observe(&run).unwrap();
        session.observe(&run).unwrap();
      }

      let result = run.join().await;
      session.finish(id, &result).unwrap();

      assert_eq!(
        session.entries(),
        [
          SessionEntry::Run(RunEntry::Message(Message::User(vec![
            UserMessageContent::Text("foo".into()),
          ]))),
          SessionEntry::Custom(custom),
          SessionEntry::Run(RunEntry::Message(Message::Agent(
            vec![AssistantContent::text("bar")].into()
          ))),
        ]
      );
      assert_eq!(session.conversation(), &result.conversation);
      assert_eq!(
        result.outcome,
        if pending {
          RunOutcome::Cancelled
        } else {
          RunOutcome::Completed
        }
      );
      assert_eq!(
        session.finish(id, &result).unwrap_err().to_string(),
        "session does not have an active run"
      );
    }

    case(false).await;
    case(true).await;
  }

  #[test]
  fn prompting_after_restore_recovers_pending_calls() {
    let call = ToolRequest::new(
      CallId::new("foo"),
      ToolFunction {
        arguments: serde_json::json!({}),
        name: ToolName::new("bar").unwrap(),
      },
    );

    let mut draft = MessageBuffer::default();
    draft.apply_many(&[
      MessageUpdate::ReasoningDelta {
        delta: "baz".into(),
        index: 0,
      },
      MessageUpdate::Content {
        content: AssistantContent::ToolCall(call.clone()),
        index: 1,
      },
    ]);

    let snapshot = SessionSnapshot::new(vec![SessionEntry::Run(
      RunEntry::Draft(draft.clone()),
    )])
    .unwrap();
    let mut session = Session::from(snapshot);
    session.prompt("qux").unwrap();

    let result = Message::User(vec![UserMessageContent::ToolResult {
      call: call.id.clone(),
      name: call.function.name.clone(),
      result: ToolResult {
        content: "interrupted".into(),
        outcome: ToolOutcome::Interrupted,
        ..Default::default()
      },
    }]);
    let prompt = Message::User(vec![UserMessageContent::Text("qux".into())]);

    assert_eq!(
      session.entries(),
      [
        SessionEntry::Run(RunEntry::Draft(draft)),
        SessionEntry::Run(RunEntry::Message(result.clone())),
        SessionEntry::Run(RunEntry::Message(prompt.clone())),
      ]
    );
    assert_eq!(
      session.snapshot().unwrap().conversation().messages(),
      [
        Message::Agent(vec![AssistantContent::ToolCall(call)].into()),
        result,
        prompt,
      ]
    );
  }
}
