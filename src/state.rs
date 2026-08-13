use super::*;

#[derive(Debug)]
pub(crate) struct State {
  active_run_id: Option<u64>,
  pub(crate) composer: Composer,
  database: Database,
  pub(crate) directory: PathBuf,
  pub(crate) input_mode: InputMode,
  pub(crate) model: Model,
  next_run_id: u64,
  queued_inputs: VecDeque<String>,
  session: Session,
  pub(crate) should_quit: bool,
  pub(crate) transcript: Transcript,
}

impl State {
  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.transcript.is_agent_active() {
      return self.interrupt_agent();
    }

    match &self.input_mode {
      InputMode::Approval(_) => self.handle_approval_action(action),
      InputMode::Compose => self.handle_composer_action(action),
    }
  }

  fn handle_agent_event(&mut self, event: AgentEvent) -> Vec<Effect> {
    match event {
      AgentEvent::Done => {
        self.active_run_id = None;
        self.input_mode.clear_approval();
        self.transcript.finish_agent_activity();
        self.save_session();

        return self.run_next_queued();
      }
      AgentEvent::Delta(delta) if self.transcript.is_agent_active() => {
        self.transcript.push_agent_delta(&delta);
      }
      AgentEvent::ReasoningDelta(delta)
        if self.transcript.is_agent_active() =>
      {
        self.transcript.push_agent_reasoning_delta(&delta);
      }
      AgentEvent::Delta(_) | AgentEvent::ReasoningDelta(_) => {}
      AgentEvent::ToolCall(tool_call) => {
        self.transcript.push_tool_call(tool_call);
        self.save_session();
      }
      AgentEvent::ToolResult { id, result } => {
        self.input_mode.clear_approval();
        self.transcript.push_tool_result(&id, result);
        self.save_session();
      }
      AgentEvent::Error(error) => {
        self.active_run_id = None;
        self.input_mode.clear_approval();
        self.transcript.error(error);
        self.save_session();

        return self.run_next_queued();
      }
      AgentEvent::ToolApprovalRequest(request) => {
        self.input_mode = InputMode::Approval(request);
      }
    }

    Vec::new()
  }

  fn handle_approval_action(&mut self, action: Action) -> Vec<Effect> {
    match action {
      Action::Edit(input) if input.key == Key::Char('y') => {
        self.resolve_approval(ToolApproval::Approved);
      }
      Action::Edit(input) if input.key == Key::Char('Y') => {
        self.resolve_approval(ToolApproval::Approved);
      }
      Action::Edit(input) if input.key == Key::Char('n') => {
        self.resolve_approval(ToolApproval::Denied);
      }
      Action::Edit(input) if input.key == Key::Char('N') => {
        self.resolve_approval(ToolApproval::Denied);
      }
      Action::Interrupt => {
        self.resolve_approval(ToolApproval::Denied);
      }
      Action::Quit => {
        self.resolve_approval(ToolApproval::Denied);
        self.quit();
      }
      Action::CompleteCommand
      | Action::Edit(_)
      | Action::SelectNext
      | Action::SelectPrevious
      | Action::Submit
      | Action::SubmitImmediately => {}
    }

    Vec::new()
  }

  fn handle_composer_action(&mut self, action: Action) -> Vec<Effect> {
    match action {
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
      Action::Quit => self.quit(),
      Action::SelectNext => self.composer.select_next(),
      Action::SelectPrevious => self.composer.select_previous(),
      Action::Submit => return self.submit(),
      Action::SubmitImmediately => return self.submit_immediately(),
    }

    Vec::new()
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::Agent { event, run_id } if self.active_run_id == Some(run_id) => {
        return self.handle_agent_event(event);
      }
      Event::Agent { .. } => {}
      Event::Error(error) => {
        self.input_mode.clear_approval();
        self.transcript.error(error);
        self.save_session();
      }
      Event::Tick(elapsed) => self.transcript.tick(elapsed),
    }

    Vec::new()
  }

  fn interrupt_agent(&mut self) -> Vec<Effect> {
    if !self.transcript.is_agent_active() {
      return Vec::new();
    }

    self.input_mode.clear_approval();
    self.active_run_id = None;
    self.transcript.interrupt();

    self.save_session();

    vec![Effect::InterruptAgent]
  }

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    Self::with_session(settings, Database::new()?, Session::new(settings)?)
  }

  pub(crate) fn queued_inputs(&self) -> &VecDeque<String> {
    &self.queued_inputs
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn resolve_approval(&mut self, approval: ToolApproval) {
    self.input_mode.resolve_approval(approval);
  }

  fn run(&mut self, input: String) -> Effect {
    self.transcript.send(input);

    self.save_session();

    let messages = self.transcript.messages();

    let run_id = self.next_run_id;

    self.next_run_id = self
      .next_run_id
      .checked_add(1)
      .expect("agent run ID overflow");

    self.active_run_id = Some(run_id);

    Effect::RunAgent { messages, run_id }
  }

  fn run_command(&mut self, command: Command) -> Vec<Effect> {
    let effects = match command {
      Command::Clear => {
        let interrupt_agent = self.active_run_id.take().is_some();

        self.input_mode.clear_approval();
        self.transcript.clear();
        self.composer.clear_history();
        self.queued_inputs.clear();

        self.save_session();

        if interrupt_agent {
          vec![Effect::InterruptAgent]
        } else {
          Vec::new()
        }
      }
      Command::Quit => self.handle_action(Action::Quit),
    };

    self.reset_input();

    effects
  }

  fn run_next_queued(&mut self) -> Vec<Effect> {
    self
      .queued_inputs
      .pop_front()
      .map(|input| vec![self.run(input)])
      .unwrap_or_default()
  }

  fn save_session(&mut self) {
    if let Err(error) = self.session.save(&self.database, &self.transcript) {
      self
        .transcript
        .error(format!("failed to save session: {error}"));
    }
  }

  fn submit(&mut self) -> Vec<Effect> {
    let input = self.composer.input_text();
    let input = input.trim();

    if let Some(command) = Command::from_input(input) {
      return self.run_command(command);
    }

    if input.starts_with('/') {
      let command = self.composer.selected_command();

      if let Some(command) = command {
        self.run_command(command);
      } else if input.len() > 1 {
        self.transcript.notice(format!(
          "Unrecognized command '{input}'. Type \"/\" for a list of supported commands."
        ));

        self.reset_input();
      }

      return Vec::new();
    }

    if input.is_empty() {
      return Vec::new();
    }

    let input = input.to_string();

    self.composer.remember(&input);
    self.reset_input();

    if self.transcript.is_agent_active() {
      self.queued_inputs.push_back(input);
      Vec::new()
    } else {
      vec![self.run(input)]
    }
  }

  fn submit_immediately(&mut self) -> Vec<Effect> {
    if !self.transcript.is_agent_active() {
      return self.submit();
    }

    let input = self.composer.input_text();
    let input = input.trim();

    if input.is_empty() || input.starts_with('/') {
      return self.submit();
    }

    let input = input.to_string();

    self.composer.remember(&input);
    self.reset_input();

    self
      .interrupt_agent()
      .into_iter()
      .chain(once(self.run(input)))
      .collect()
  }

  pub(crate) fn with_session(
    settings: &Settings,
    database: Database,
    mut session: Session,
  ) -> Result<Self> {
    let transcript = Transcript::with_entries(session.entries.clone());

    let history = transcript
      .entries
      .iter()
      .filter_map(|entry| match entry {
        TranscriptEntry::User(input) => Some(input.clone()),
        _ => None,
      })
      .collect();

    session.set_model(&settings.model);

    Ok(Self {
      active_run_id: None,
      composer: Composer::new(
        settings.prompt.as_deref().unwrap_or_default(),
        history,
      ),
      database,
      directory: env::current_dir()?,
      input_mode: InputMode::Compose,
      model: settings.model.clone(),
      next_run_id: 0,
      queued_inputs: VecDeque::new(),
      session,
      should_quit: false,
      transcript,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_events_update_transcript() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_agent_event(AgentEvent::ReasoningDelta("bar".into()));
    state.handle_agent_event(AgentEvent::Delta("baz".into()));

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    state.handle_agent_event(AgentEvent::ToolCall(invocation.clone()));

    let result = ToolResult {
      exit_status: Some(0),
      outcome: ToolOutcome::Success,
      stdout: Some("qux".into()),
      ..Default::default()
    };

    state.handle_agent_event(AgentEvent::ToolResult {
      id: "foo".into(),
      result: result.clone(),
    });

    state.handle_agent_event(AgentEvent::Done);

    assert_eq!(
      state.transcript.messages(),
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::Agent(vec![
          AgentMessageContent::Reasoning("bar".into()),
          AgentMessageContent::Text("baz".into()),
          AgentMessageContent::ToolCall(invocation),
        ]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "foo".into(),
          result,
        }]),
      ],
    );
  }

  #[tokio::test]
  async fn approval_approves_with_lowercase_y() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('y'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Approved);

    assert_matches!(state.input_mode, InputMode::Compose);
  }

  #[tokio::test]
  async fn approval_approves_with_uppercase_y() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('Y'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Approved);

    assert_matches!(state.input_mode, InputMode::Compose);
  }

  #[test]
  fn approval_complete_command_leaves_request_pending() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::CompleteCommand)),
      Vec::new()
    );

    assert_matches!(state.input_mode, InputMode::Approval(_));
  }

  #[tokio::test]
  async fn approval_denies_with_escape() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_matches!(state.input_mode, InputMode::Compose);
  }

  #[tokio::test]
  async fn approval_denies_with_lowercase_n() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('n'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_matches!(state.input_mode, InputMode::Compose);
  }

  #[tokio::test]
  async fn approval_denies_with_quit() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(state.handle_event(Event::Action(Action::Quit)), Vec::new());
    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_matches!(state.input_mode, InputMode::Compose);
    assert!(state.should_quit);
  }

  #[tokio::test]
  async fn approval_denies_with_uppercase_n() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('N'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_matches!(state.input_mode, InputMode::Compose);
  }

  #[test]
  fn approval_edit_other_key_leaves_request_pending() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('x'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_matches!(state.input_mode, InputMode::Approval(_));
  }

  #[test]
  fn approval_select_next_command_leaves_request_pending() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::SelectNext)),
      Vec::new()
    );

    assert_matches!(state.input_mode, InputMode::Approval(_));
  }

  #[test]
  fn approval_select_previous_command_leaves_request_pending() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::SelectPrevious)),
      Vec::new()
    );

    assert_matches!(state.input_mode, InputMode::Approval(_));
  }

  #[test]
  fn approval_submit_leaves_request_pending() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert_matches!(state.input_mode, InputMode::Approval(_));
  }

  #[tokio::test]
  async fn approval_terminal_agent_done_drops_pending_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));
    state.handle_agent_event(AgentEvent::Done);

    assert_matches!(state.input_mode, InputMode::Compose);
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    state.handle_agent_event(AgentEvent::ToolResult {
      id: "foo".into(),
      result: ToolResult {
        content: Some("bar".into()),
        ..Default::default()
      },
    });

    assert_matches!(state.input_mode, InputMode::Compose);
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn approval_terminal_error_drops_pending_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));
    state.handle_agent_event(AgentEvent::Error("bar".into()));

    assert_matches!(state.input_mode, InputMode::Compose);
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn blank_submit_does_nothing() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("  ".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "  ");
  }

  #[test]
  fn command_autocomplete_select_next() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state
        .composer
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectNext));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.composer.input_text(), "/quit");
  }

  #[test]
  fn command_autocomplete_select_previous() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state
        .composer
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectPrevious));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.composer.input_text(), "/quit");
  }

  #[test]
  fn command_clear_from_empty_slash() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_agent_event(AgentEvent::Delta("bar".into()));
    state.handle_agent_event(AgentEvent::Done);

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Char('/'),
      ..Default::default()
    })));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_from_name() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_agent_event(AgentEvent::Delta("bar".into()));
    state.handle_agent_event(AgentEvent::Done);

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_interrupts_active_agent_and_ignores_late_events() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::InterruptAgent]
    );

    assert_eq!(state.active_run_id, None);

    assert!(state.transcript.messages().is_empty());

    let invocation = ToolInvocation {
      id: "late".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "echo late".into(),
        cwd: None,
      }),
    };

    state.handle_event(Event::Agent {
      event: AgentEvent::ToolCall(invocation),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::ToolResult {
        id: "late".into(),
        result: ToolResult::default(),
      },
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert!(state.transcript.messages().is_empty());
  }

  #[test]
  fn command_clear_from_prefix() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_agent_event(AgentEvent::Delta("bar".into()));
    state.handle_agent_event(AgentEvent::Done);

    for c in "/c".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_from_name() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/quit".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit);

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_from_prefix() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/q".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit);

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_interrupts_active_agent_and_saves_partial_output() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Delta("partial response".into()));

    for character in "/quit".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(character),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit);
    assert!(!state.transcript.is_agent_active());

    assert_eq!(state.composer.input_text(), "");

    let saved = state
      .database
      .load_session(state.session.id.unwrap())
      .unwrap();

    assert_matches!(
      &saved.entries[..],
      [
        TranscriptEntry::User(input),
        TranscriptEntry::Agent(output),
        TranscriptEntry::Interrupted,
      ] if input == "foo" && output == "partial response"
    );
  }

  #[test]
  fn error_is_not_included_in_next_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_agent_event(AgentEvent::Error("bar".into()));

    assert_eq!(
      state.transcript.messages(),
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
      vec![Effect::RunAgent {
        messages: vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::User(vec![UserMessageContent::Text("baz".into())]),
        ],
        run_id: 1,
      }]
    );
  }

  #[test]
  fn immediate_submit_interrupts_active_agent_and_starts_new_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Delta("partial".into()));

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::SubmitImmediately)),
      vec![
        Effect::InterruptAgent,
        Effect::RunAgent {
          messages: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::Agent(vec![AgentMessageContent::Text("partial".into())]),
            Message::User(vec![UserMessageContent::Text("bar".into())]),
          ],
          run_id: 1,
        },
      ]
    );

    assert!(state.transcript.is_agent_active());

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert!(state.transcript.is_agent_active());
  }

  #[test]
  fn interrupt_advances_to_next_queued_submission() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("first".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    for c in "second".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));

    assert_matches!(
      state
        .handle_event(Event::Action(Action::Interrupt))
        .as_slice(),
      [Effect::InterruptAgent, Effect::RunAgent { run_id: 1, .. }]
    );

    assert!(state.queued_inputs().is_empty());
    assert!(state.transcript.is_agent_active());
  }

  #[test]
  fn interrupt_stops_active_agent() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.transcript.is_agent_active());

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
    );
  }

  #[test]
  fn multiline_input() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    for c in "foo".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Enter,
      ..Default::default()
    })));

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo\nbar".into()
        )])],
        run_id: 0,
      }]
    );
  }

  #[test]
  fn new_uses_empty_prompt_by_default() {
    let state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn prompt_history_edit_detaches_navigation() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("history".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Done);
    state.handle_event(Event::Action(Action::SelectPrevious));
    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Char('!'),
      ..Default::default()
    })));
    state.handle_event(Event::Action(Action::SelectNext));

    assert_eq!(state.composer.input_text(), "history!");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "history");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "history!");
  }

  #[test]
  fn prompt_history_is_cleared_by_clear_command() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("history".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Done);

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Action(Action::SelectPrevious));

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn prompt_history_loads_session() {
    let settings = Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("draft".into()),
      yolo: false,
    };

    let mut session = Session::new(&settings).unwrap();
    session.entries = vec![
      TranscriptEntry::User("foo".into()),
      TranscriptEntry::Agent("bar".into()),
      TranscriptEntry::User("baz\nqux".into()),
    ];

    let mut state =
      State::with_session(&settings, Database::new().unwrap(), session)
        .unwrap();

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "baz\nqux");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.cursor().0, 0);

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "foo");
  }

  #[test]
  fn prompt_history_navigates_and_restores_draft() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Done);

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Done);

    for c in "draft".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "bar");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "foo");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "bar");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "draft");
  }

  #[test]
  fn prompt_history_preserves_multiline_navigation() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("history".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_agent_event(AgentEvent::Done);

    for input in [
      Input {
        key: Key::Char('a'),
        ..Default::default()
      },
      Input {
        key: Key::Enter,
        ..Default::default()
      },
      Input {
        key: Key::Char('b'),
        ..Default::default()
      },
    ] {
      state.handle_event(Event::Action(Action::Edit(input)));
    }

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "a\nb");
    assert_eq!(state.composer.cursor().0, 0);

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "history");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "a\nb");
    assert_eq!(state.composer.cursor().0, 1);
  }

  #[test]
  fn queued_submissions_run_in_order() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("first".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    for input in ["second", "third"] {
      for c in input.chars() {
        state.handle_event(Event::Action(Action::Edit(Input {
          key: Key::Char(c),
          ..Default::default()
        })));
      }

      state.handle_event(Event::Action(Action::Submit));
    }

    assert_eq!(state.queued_inputs().len(), 2);

    assert_matches!(
      state
        .handle_event(Event::Agent {
          event: AgentEvent::Done,
          run_id: 0,
        })
        .as_slice(),
      [Effect::RunAgent { run_id: 1, .. }]
    );

    assert_matches!(
      state
        .handle_event(Event::Agent {
          event: AgentEvent::Done,
          run_id: 1,
        })
        .as_slice(),
      [Effect::RunAgent { run_id: 2, .. }]
    );

    assert_eq!(
      state.transcript.messages(),
      [
        Message::User(vec![UserMessageContent::Text("first".into())]),
        Message::User(vec![UserMessageContent::Text("second".into())]),
        Message::User(vec![UserMessageContent::Text("third".into())]),
      ]
    );
  }

  #[test]
  fn quit_interrupts_active_agent() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit);
    assert!(!state.transcript.is_agent_active());

    assert_eq!(state.handle_event(Event::Action(Action::Quit)), Vec::new());

    assert!(state.should_quit);
  }

  #[tokio::test]
  async fn quit_interrupts_active_approval() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_agent_event(AgentEvent::ToolApprovalRequest(request));

    assert_matches!(state.input_mode, InputMode::Approval(_));

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit);
    assert!(!state.transcript.is_agent_active());

    assert_matches!(state.input_mode, InputMode::Compose);
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn stale_agent_events_do_not_mutate_new_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("old".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Action(Action::Interrupt));

    for c in "new".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_matches!(
      state.handle_event(Event::Action(Action::Submit)).as_slice(),
      [Effect::RunAgent { run_id: 1, .. }]
    );

    let invocation = ToolInvocation {
      id: "stale".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "echo".into(),
        cwd: None,
      }),
    };

    let (request, response_receiver) = ApprovalRequest::new(invocation.clone());

    for event in [
      AgentEvent::Delta("stale".into()),
      AgentEvent::ReasoningDelta("stale".into()),
      AgentEvent::ToolCall(invocation),
      AgentEvent::ToolResult {
        id: "stale".into(),
        result: ToolResult {
          content: Some("stale".into()),
          ..Default::default()
        },
      },
      AgentEvent::ToolApprovalRequest(request),
      AgentEvent::Error("stale".into()),
      AgentEvent::Done,
    ] {
      state.handle_event(Event::Agent { event, run_id: 0 });
    }

    assert_eq!(
      response_receiver.await.unwrap_err().to_string(),
      "channel closed"
    );

    assert_matches!(state.input_mode, InputMode::Compose);

    assert!(state.transcript.is_agent_active());

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("current".into()),
      run_id: 1,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 1,
    });

    assert_eq!(
      state.transcript.messages(),
      [
        Message::User(vec![UserMessageContent::Text("old".into())]),
        Message::User(vec![UserMessageContent::Text("new".into())]),
        Message::Agent(vec![AgentMessageContent::Text("current".into())]),
      ]
    );
  }

  #[test]
  fn submit_is_queued_while_agent_active() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert_eq!(state.composer.input_text(), "");
    assert_eq!(state.queued_inputs().len(), 1);

    assert_eq!(
      state.transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Done,
        run_id: 0,
      }),
      vec![Effect::RunAgent {
        messages: vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::User(vec![UserMessageContent::Text("bar".into())]),
        ],
        run_id: 1,
      }]
    );

    assert!(state.queued_inputs().is_empty());
  }

  #[test]
  fn submit_trims_input() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("  foo  ".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/foobar".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert_matches!(
      &state.transcript.entries[..],
      [TranscriptEntry::Notice(notice)]
        if notice
          == "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
    );

    assert_eq!(state.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "");
  }
}
