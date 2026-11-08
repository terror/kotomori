use super::*;

#[derive(Debug)]
pub(crate) struct State {
  pub(crate) composer: Composer,
  pub(crate) queued_inputs: VecDeque<String>,
  pub(crate) reasoning_expanded: bool,
  pub(crate) run: Option<Run>,
  pub(crate) session: Session,
  pub(crate) should_quit: bool,
}

impl State {
  fn finish_run(&mut self, entry: Option<TranscriptEntry>) {
    self
      .session
      .push_many(self.run.take().and_then(Run::finish));

    self.session.interrupt_pending_calls();

    self.session.push_many(entry);
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.run.is_some() {
      return self.interrupt_agent();
    }

    match self.run.as_ref().and_then(|run| run.approval.as_ref()) {
      Some(_) => match action {
        Action::Edit(input) if input.key == Key::Char('y') => {
          return self.resolve_approval(ToolApproval::Approved);
        }
        Action::Edit(input) if input.key == Key::Char('Y') => {
          return self.resolve_approval(ToolApproval::Approved);
        }
        Action::Edit(input) if input.key == Key::Char('n') => {
          return self.resolve_approval(ToolApproval::Denied);
        }
        Action::Edit(input) if input.key == Key::Char('N') => {
          return self.resolve_approval(ToolApproval::Denied);
        }
        Action::Interrupt => {
          return self.resolve_approval(ToolApproval::Denied);
        }
        Action::ToggleReasoning => {
          self.reasoning_expanded = !self.reasoning_expanded;
        }
        Action::CompleteCommand
        | Action::Edit(_)
        | Action::Paste(_)
        | Action::Quit
        | Action::SelectNext
        | Action::SelectPrevious
        | Action::Submit
        | Action::SubmitImmediately => {}
      },
      None => match action {
        Action::CompleteCommand => {
          if !self.composer.complete_command() {
            self.composer.input(Input {
              key: Key::Tab,
              ..Default::default()
            });
          }
        }
        Action::Edit(input) => self.composer.input(input),
        Action::Interrupt => {
          let effects = self.interrupt_agent();

          if effects.is_empty() {
            return effects;
          }

          return effects.into_iter().chain(self.run_next_queued()).collect();
        }
        Action::Paste(input) => self.composer.paste(&input),
        Action::Quit => self.quit(),
        Action::SelectNext => self.composer.select_next(),
        Action::SelectPrevious => self.composer.select_previous(),
        Action::Submit | Action::SubmitImmediately => {
          return self.submit(&action);
        }
        Action::ToggleReasoning => {
          self.reasoning_expanded = !self.reasoning_expanded;
        }
      },
    }

    Vec::new()
  }

  fn handle_agent_event(&mut self, event: RunEvent) -> Vec<Effect> {
    let RunEvent { event, run_id } = event;

    let Some(run) = self.run.as_mut().filter(|run| run.id == Some(run_id))
    else {
      return Vec::new();
    };

    match event {
      AgentEvent::RunCompleted => {
        self.finish_run(None);

        return once(Effect::SaveSession)
          .chain(self.run_next_queued())
          .collect();
      }
      AgentEvent::MessagePreview { update, .. } => {
        run.update(update);
      }
      AgentEvent::MessageCommitted { message, .. } => {
        run.reset_message();

        self.session.push(TranscriptEntry::Message(message));

        return vec![Effect::SaveSession];
      }
      AgentEvent::RunFailed(error) => {
        self.finish_run(Some(TranscriptEntry::Error(error)));

        return once(Effect::SaveSession)
          .chain(self.run_next_queued())
          .collect();
      }
      AgentEvent::ApprovalRequested(request) if request.run_id == run_id => {
        run.approval = Some(request);
      }
      AgentEvent::ApprovalResolved { tool_id, .. } => {
        if run
          .approval
          .as_ref()
          .is_some_and(|request| request.tool_id == tool_id)
        {
          run.approval = None;
        }
      }
      AgentEvent::ApprovalRequested(_)
      | AgentEvent::RunStarted
      | AgentEvent::ToolCompleted { .. }
      | AgentEvent::ToolDenied { .. }
      | AgentEvent::ToolRequested { .. }
      | AgentEvent::ToolStarted { .. } => {}
    }

    Vec::new()
  }

  fn handle_command(
    &mut self,
    command: Command,
    arguments: &str,
  ) -> Vec<Effect> {
    let effects = match command {
      Command::Clear => {
        let interrupt_agent = self.run.take().is_some();

        self.session.clear();
        self.composer.clear_history();
        self.queued_inputs.clear();

        if interrupt_agent {
          vec![Effect::InterruptAgent, Effect::SaveSession]
        } else {
          vec![Effect::SaveSession]
        }
      }
      Command::Copy => {
        let response = self
          .run
          .as_ref()
          .and_then(|run| run.message.message().text())
          .or_else(|| self.session.conversation.last_response());

        if let Some(response) = response {
          vec![Effect::CopyToClipboard(response)]
        } else {
          self.session.push(TranscriptEntry::Notice(
            "No assistant response to copy.".into(),
          ));

          Vec::new()
        }
      }
      Command::Fork => {
        if self.run.is_some() {
          self.finish_run(Some(TranscriptEntry::Interrupted));
          vec![Effect::InterruptAgent, Effect::ForkSession]
        } else {
          vec![Effect::ForkSession]
        }
      }
      Command::Quit => self.handle_action(Action::Quit),
      Command::Rename => {
        if arguments.is_empty() {
          self
            .session
            .push(TranscriptEntry::Notice("Usage: /rename <title>".into()));

          Vec::new()
        } else {
          let title =
            arguments.split_whitespace().collect::<Vec<_>>().join(" ");

          self.session.push(TranscriptEntry::Notice(format!(
            "Session renamed to '{title}'."
          )));

          self.session.title = Some(title);

          vec![Effect::SaveSession]
        }
      }
    };

    self.reset_input();

    effects
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::Agent(event) => return self.handle_agent_event(event),
      Event::ClipboardCopied(result) => match result {
        Ok(()) => self.session.push(TranscriptEntry::Notice(
          "Copied last assistant response to clipboard.".into(),
        )),
        Err(error) => self.session.push(TranscriptEntry::Error(format!(
          "failed to copy response to clipboard: {error}"
        ))),
      },
      Event::Error(error) => {
        let effects = if self.run.is_some() {
          vec![Effect::InterruptAgent]
        } else {
          Vec::new()
        };

        self.finish_run(Some(TranscriptEntry::Error(error)));

        return effects
          .into_iter()
          .chain(once(Effect::SaveSession))
          .collect();
      }
      Event::RunSpawned(run_id) => {
        if let Some(run) = self.run.as_mut().filter(|run| run.id.is_none()) {
          run.id = Some(run_id);
        }
      }
      Event::SessionForked(result) => match result {
        Ok(session) => {
          self.session = session;

          self
            .session
            .push(TranscriptEntry::Notice("Forked conversation.".into()));

          self.queued_inputs.clear();
        }
        Err(error) => self.session.push(TranscriptEntry::Error(format!(
          "failed to fork session: {error}"
        ))),
      },
      Event::SessionSaved(result) => match result {
        Ok(saved) => {
          self.session.id = Some(saved.id);
          self.session.title = saved.title;
          self.session.updated_at = saved.updated_at;
        }
        Err(error) => self.session.push(TranscriptEntry::Error(format!(
          "failed to save session: {error}"
        ))),
      },
      Event::Tick(elapsed) => {
        if let Some(run) = &mut self.run {
          run.tick(elapsed);
        }
      }
    }

    Vec::new()
  }

  fn interrupt_agent(&mut self) -> Vec<Effect> {
    if self.run.is_none() {
      return Vec::new();
    }

    self.finish_run(Some(TranscriptEntry::Interrupted));

    vec![Effect::InterruptAgent, Effect::SaveSession]
  }

  pub(crate) fn new(mut session: Session) -> Self {
    session.interrupt_pending_calls();

    let history = session
      .conversation
      .messages()
      .iter()
      .filter_map(Message::user_content)
      .map(str::to_owned)
      .collect();

    Self {
      composer: Composer::new(
        session.settings.prompt.as_deref().unwrap_or_default(),
        history,
      ),
      queued_inputs: VecDeque::new(),
      reasoning_expanded: false,
      run: None,
      session,
      should_quit: false,
    }
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn resolve_approval(&mut self, approval: ToolApproval) -> Vec<Effect> {
    self
      .run
      .as_mut()
      .and_then(|run| run.resolve_approval(approval))
      .map(Effect::RespondToApproval)
      .into_iter()
      .collect()
  }

  fn run(&mut self, input: String) -> Effect {
    self
      .session
      .push(TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::Text(input),
      ])));

    let conversation = self.session.conversation.clone();

    self.run = Some(Run::new(None));

    Effect::RunAgent { conversation }
  }

  fn run_next_queued(&mut self) -> Vec<Effect> {
    self
      .queued_inputs
      .pop_front()
      .map(|input| vec![self.run(input)])
      .unwrap_or_default()
  }

  fn submit(&mut self, action: &Action) -> Vec<Effect> {
    let input = self.composer.input_text();
    let trimmed = input.trim();

    if let Some((command, arguments)) =
      Command::from_input(trimmed).or_else(|| {
        self
          .composer
          .selected_command()
          .map(|command| (command, ""))
      })
    {
      return self.handle_command(command, arguments);
    }

    if trimmed.starts_with('/') {
      if trimmed.len() > 1 {
        self.session.push(TranscriptEntry::Notice(format!(
          "Unrecognized command '{trimmed}'. Type \"/\" for a list of supported commands."
        )));

        self.reset_input();
      }

      return Vec::new();
    }

    if trimmed.is_empty() {
      return Vec::new();
    }

    self.composer.remember(&input);
    self.reset_input();

    match action {
      Action::SubmitImmediately if self.run.is_some() => self
        .interrupt_agent()
        .into_iter()
        .chain(once(self.run(input)))
        .collect(),
      _ if self.run.is_some() => {
        self.queued_inputs.push_back(input);
        Vec::new()
      }
      _ => vec![Effect::SaveSession, self.run(input)],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_events_without_run_are_ignored() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    let invocation = ToolInvocation {
      protocol: ToolRequest::new(
        CallId::new("foo"),
        ToolFunction {
          arguments: serde_json::json!({"command": "bar"}),
          name: ToolName::new("command").unwrap(),
        },
      ),
    };

    for event in [
      AgentEvent::MessageCommitted {
        message_id: MessageId(0),
        message: Message::Agent(
          vec![AssistantContent::ToolCall(invocation.protocol)].into(),
        ),
      },
      AgentEvent::MessageCommitted {
        message_id: MessageId(0),
        message: Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::new("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult::default(),
        }]),
      },
      AgentEvent::RunCompleted,
    ] {
      assert_eq!(
        state.handle_event(Event::Agent(RunEvent {
          event,
          run_id: RunId(0)
        })),
        Vec::new(),
      );
    }

    assert_eq!(state.run, None);
    assert_eq!(state.session.transcript.entries, []);
  }

  #[test]
  fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let request = ApprovalRequest {
      invocation: ToolInvocation {
        protocol: ToolRequest::new(
          CallId::new("foo"),
          ToolFunction {
            arguments: serde_json::json!({"command": "bar"}),
            name: ToolName::new("command").unwrap(),
          },
        ),
      },
      run_id: RunId(0),
      tool_id: ToolId(0),
    };

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(Some(RunId(0)))
    });

    state.handle_event(Event::Agent(RunEvent {
      event: AgentEvent::MessageCommitted {
        message_id: MessageId(0),
        message: Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::new("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        }]),
      },
      run_id: RunId(0),
    }));

    assert_eq!(state.run, Some(Run::new(Some(RunId(0)))));
    assert_eq!(state.resolve_approval(ToolApproval::Approved), Vec::new());
  }

  #[test]
  fn approval_terminal_error_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    let request = ApprovalRequest {
      invocation: ToolInvocation {
        protocol: ToolRequest::new(
          CallId::new("foo"),
          ToolFunction {
            arguments: serde_json::json!({"command": "bar"}),
            name: ToolName::new("command").unwrap(),
          },
        ),
      },
      run_id: RunId(0),
      tool_id: ToolId(0),
    };

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(Some(RunId(0)))
    });

    state.handle_event(Event::Agent(RunEvent {
      event: AgentEvent::RunFailed("bar".into()),
      run_id: RunId(0),
    }));

    assert_eq!(state.run, None);
    assert_eq!(state.resolve_approval(ToolApproval::Approved), Vec::new());
  }

  #[test]
  fn completed_messages_survive_resumption() {
    let settings = Settings {
      directory: "foo".into(),
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    };

    let mut state = State::new(Session::new(&settings, 0));

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::RunSpawned(RunId(0)));

    for update in [
      MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      },
      MessageUpdate::ReasoningDelta {
        delta: "baz".into(),
        index: 1,
      },
    ] {
      state.handle_event(Event::Agent(RunEvent {
        event: AgentEvent::MessagePreview {
          message_id: MessageId(0),
          update,
        },
        run_id: RunId(0),
      }));
    }

    let messages = vec![
      Message::Agent(
        vec![
          AssistantContent::reasoning("foo", "foo"),
          AssistantContent::text("bar"),
          AssistantContent::ToolCall(ToolRequest::new(
            CallId::new("foo"),
            ToolFunction {
              arguments: serde_json::json!({"command": "bar"}),
              name: ToolName::new("command").unwrap(),
            },
          )),
          AssistantContent::text("baz"),
          AssistantContent::ToolCall(ToolRequest::new(
            CallId::new("bar"),
            ToolFunction {
              arguments: serde_json::json!({"command": "qux"}),
              name: ToolName::new("command").unwrap(),
            },
          )),
        ]
        .into(),
      ),
      Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::new("bar"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("foo".into()),
            ..Default::default()
          },
        },
        UserMessageContent::ToolResult {
          call: CallId::new("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        },
      ]),
      Message::Agent(vec![AssistantContent::text("qux")].into()),
    ];

    for message in &messages {
      state.handle_event(Event::Agent(RunEvent {
        event: AgentEvent::MessageCommitted {
          message_id: MessageId(0),
          message: message.clone(),
        },
        run_id: RunId(0),
      }));
    }

    state.handle_event(Event::Agent(RunEvent {
      event: AgentEvent::RunCompleted,
      run_id: RunId(0),
    }));

    let mut state = State::new(state.session);

    let messages =
      once(Message::User(vec![UserMessageContent::Text("foo".into())]))
        .chain(messages)
        .chain(once(Message::User(vec![UserMessageContent::Text(
          "foo".into(),
        )])))
        .collect::<Vec<_>>();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      [
        Effect::SaveSession,
        Effect::RunAgent {
          conversation: messages.into(),
        }
      ]
    );
  }

  #[test]
  fn error_is_not_included_in_next_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          conversation: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])]
          .into(),
        }
      ]
    );

    state.handle_event(Event::RunSpawned(RunId(0)));

    state.handle_event(Event::Agent(RunEvent {
      event: AgentEvent::RunFailed("bar".into()),
      run_id: RunId(0),
    }));

    assert_eq!(
      state.session.conversation.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );

    for c in "baz".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          conversation: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::User(vec![UserMessageContent::Text("baz".into())]),
          ]
          .into(),
        }
      ]
    );
  }

  #[test]
  fn failed_save_preserves_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some({
      let mut run = Run::new(Some(RunId(0)));
      run.update(MessageUpdate::Text {
        delta: "foo".into(),
        index: 0,
      });
      run
    });

    state.session.id = Some(0);

    assert_eq!(
      state.handle_event(Event::SessionSaved(Err("foo".into()))),
      Vec::new()
    );

    assert_eq!(
      state.run,
      Some({
        let mut run = Run::new(Some(RunId(0)));
        run.update(MessageUpdate::Text {
          delta: "foo".into(),
          index: 0,
        });
        run
      })
    );
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("failed to save session: foo".into())]
    );
  }

  #[test]
  fn finish_run_saves_partial_output() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some({
      let mut run = Run::new(Some(RunId(0)));
      run.update(MessageUpdate::Text {
        delta: "foo".into(),
        index: 0,
      });
      run
    });

    assert_eq!(
      state.handle_event(Event::Agent(RunEvent {
        event: AgentEvent::RunCompleted,
        run_id: RunId(0)
      })),
      [Effect::SaveSession]
    );

    assert_eq!(state.run, None);
    assert_eq!(
      state.session.conversation.messages(),
      [Message::Agent(vec![AssistantContent::text("foo")].into())]
    );
  }

  #[test]
  fn interruption_preserves_streamed_protocol() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:foo".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some(Run::new(Some(RunId(0))));

    let (event_sender, mut events) = mpsc::unbounded_channel();

    let sink = ProviderSink::new(event_sender, RunId(0), MessageId(0));

    let reasoning = Reasoning {
      content: vec![ReasoningContent::Text {
        text: "foo".into(),
        signature: Some("bar".into()),
      }],
      id: Some("baz".into()),
      issuer: "foo".into(),
    };

    let encrypted = Reasoning {
      content: vec![ReasoningContent::Encrypted("qux".into())],
      id: Some("quux".into()),
      issuer: "foo".into(),
    };

    for update in [
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        index: 0,
      },
      MessageUpdate::Content {
        index: 0,
        content: AssistantContent::Reasoning(reasoning.clone()),
      },
      MessageUpdate::Content {
        index: 1,
        content: AssistantContent::Reasoning(encrypted.clone()),
      },
      MessageUpdate::Text {
        delta: "quuz".into(),
        index: 2,
      },
    ] {
      sink.update(update).unwrap();
    }

    while let Ok(event) = events.try_recv() {
      state.handle_event(Event::Agent(event));
    }

    let content = vec![
      AssistantContent::Reasoning(reasoning),
      AssistantContent::Reasoning(encrypted),
      AssistantContent::text("quuz"),
    ];

    state.handle_event(Event::Action(Action::Interrupt));

    let session = state.session;

    assert_eq!(
      session.transcript.entries,
      [
        TranscriptEntry::Message(Message::Agent(content.into())),
        TranscriptEntry::Interrupted,
      ]
    );
  }

  #[allow(clippy::too_many_lines)]
  #[test]
  fn stale_agent_events_do_not_mutate_new_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("old".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::RunSpawned(RunId(0)));
    state.handle_event(Event::Action(Action::Interrupt));

    for c in "new".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      [
        Effect::SaveSession,
        Effect::RunAgent {
          conversation: vec![
            Message::User(vec![UserMessageContent::Text("old".into())]),
            Message::User(vec![UserMessageContent::Text("new".into())]),
          ]
          .into(),
        }
      ]
    );

    state.handle_event(Event::RunSpawned(RunId(1)));

    let invocation = ToolInvocation {
      protocol: ToolRequest::new(
        CallId::new("stale"),
        ToolFunction {
          arguments: serde_json::json!({"command": "echo"}),
          name: ToolName::new("command").unwrap(),
        },
      ),
    };

    let request = ApprovalRequest {
      invocation: invocation.clone(),
      run_id: RunId(0),
      tool_id: ToolId(0),
    };

    for event in [
      AgentEvent::MessagePreview {
        message_id: MessageId(0),
        update: MessageUpdate::Text {
          delta: "stale".into(),
          index: 0,
        },
      },
      AgentEvent::MessagePreview {
        message_id: MessageId(0),
        update: MessageUpdate::ReasoningDelta {
          delta: "stale".into(),
          index: 0,
        },
      },
      AgentEvent::MessageCommitted {
        message_id: MessageId(0),
        message: Message::Agent(
          vec![AssistantContent::ToolCall(invocation.protocol)].into(),
        ),
      },
      AgentEvent::MessageCommitted {
        message_id: MessageId(0),
        message: Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::new("stale"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("stale".into()),
            ..Default::default()
          },
        }]),
      },
      AgentEvent::ApprovalRequested(request),
      AgentEvent::RunFailed("stale".into()),
      AgentEvent::RunCompleted,
    ] {
      assert_eq!(
        state.handle_event(Event::Agent(RunEvent {
          event,
          run_id: RunId(0)
        })),
        Vec::new()
      );
    }

    assert_eq!(state.resolve_approval(ToolApproval::Approved), Vec::new());

    assert_eq!(
      state.run.as_ref().and_then(|run| run.approval.as_ref()),
      None
    );

    assert_eq!(state.run, Some(Run::new(Some(RunId(1)))));

    state.handle_event(Event::Agent(RunEvent {
      event: AgentEvent::MessagePreview {
        message_id: MessageId(0),
        update: MessageUpdate::Text {
          delta: "current".into(),
          index: 0,
        },
      },
      run_id: RunId(1),
    }));

    state.handle_event(Event::Agent(RunEvent {
      event: AgentEvent::RunCompleted,
      run_id: RunId(1),
    }));

    assert_eq!(
      state.session.conversation.messages(),
      [
        Message::User(vec![UserMessageContent::Text("old".into())]),
        Message::User(vec![UserMessageContent::Text("new".into())]),
        Message::Agent(vec![AssistantContent::text("current")].into()),
      ]
    );
  }

  #[test]
  fn successful_save_updates_metadata_without_saving_again() {
    let settings = Settings {
      directory: "foo".into(),
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    let mut state = State::new(Session::new(&settings, 1));

    state
      .session
      .push(TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::Text("bar".into()),
      ])));

    assert_eq!(
      state.handle_event(Event::SessionSaved(Ok(SavedSession {
        id: 2,
        title: Some("bar".into()),
        updated_at: 3,
      }))),
      Vec::new(),
    );
    assert_eq!(
      state.session,
      Session {
        conversation: vec![Message::User(vec![UserMessageContent::Text(
          "bar".into()
        )])]
        .into(),
        created_at: 1,
        id: Some(2),
        settings,
        title: Some("bar".into()),
        transcript: Transcript::new(vec![TranscriptEntry::Message(
          Message::User(vec![UserMessageContent::Text("bar".into())]),
        )]),
        updated_at: 3,
      }
    );
    assert_eq!(state.session.settings.directory, Path::new("foo"));
  }

  #[test]
  fn terminal_error_interrupts_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some(Run::new(Some(RunId(0))));

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );
    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn terminal_error_without_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::SaveSession]
    );
    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn unfinished_reasoning_survives_without_entering_requests() {
    #[track_caller]
    fn case(event: Event, entry: TranscriptEntry) {
      let settings = Settings {
        directory: "foo".into(),
        model: "mock:foo".parse().unwrap(),
        prompt: Some("qux".into()),
        yolo: false,
      };

      let mut state = State::new(Session::new(&settings, 0));

      let reasoning = Reasoning::new(
        "foo",
        vec![ReasoningContent::Text {
          text: "foo".into(),
          signature: Some("bar".into()),
        }],
      );

      let mut run = Run::new(Some(RunId(0)));

      run.update_many(&[
        MessageUpdate::Content {
          index: 0,
          content: AssistantContent::Reasoning(reasoning.clone()),
        },
        MessageUpdate::Text {
          delta: "baz".into(),
          index: 1,
        },
        MessageUpdate::ReasoningDelta {
          delta: "quux".into(),
          index: 2,
        },
      ]);

      let draft = run.message.clone();

      state.run = Some(run);
      state.handle_event(event);

      let session = state.session;

      assert_eq!(
        session.transcript.entries,
        [TranscriptEntry::Draft(draft), entry]
      );

      let mut state = State::new(session);

      assert_eq!(
        state.handle_event(Event::Action(Action::Submit)),
        [
          Effect::SaveSession,
          Effect::RunAgent {
            conversation: vec![
              Message::Agent(
                vec![
                  AssistantContent::Reasoning(reasoning),
                  AssistantContent::text("baz"),
                ]
                .into(),
              ),
              Message::User(vec![UserMessageContent::Text("qux".into())]),
            ]
            .into(),
          }
        ]
      );
    }

    case(
      Event::Action(Action::Interrupt),
      TranscriptEntry::Interrupted,
    );

    case(
      Event::Agent(RunEvent {
        event: AgentEvent::RunFailed("foo".into()),
        run_id: RunId(0),
      }),
      TranscriptEntry::Error("foo".into()),
    );
  }
}
