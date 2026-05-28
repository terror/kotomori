use super::*;

#[derive(Debug)]
pub(crate) struct State {
  composer: Composer,
  input_mode: InputMode,
  should_quit: bool,
  transcript: Transcript,
}

impl State {
  pub(crate) fn composer(&self) -> &Composer {
    &self.composer
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.transcript.is_agent_active() {
      return self.interrupt_agent();
    }

    match &self.input_mode {
      InputMode::Approval(_) => self.handle_approval_action(action),
      InputMode::Compose => self.handle_composer_action(action),
    }
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
      | Action::SelectNextCommand
      | Action::SelectPreviousCommand
      | Action::Submit => {}
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
        return self.interrupt_agent();
      }
      Action::Quit => self.quit(),
      Action::SelectNextCommand => {
        if self.composer.selected_command().is_some() {
          self.composer.select_next_command();
        } else {
          self.composer.input(Input {
            key: Key::Down,
            ..Default::default()
          });
        }
      }
      Action::SelectPreviousCommand => {
        if self.composer.selected_command().is_some() {
          self.composer.select_previous_command();
        } else {
          self.composer.input(Input {
            key: Key::Up,
            ..Default::default()
          });
        }
      }
      Action::Submit => return self.submit(),
    }

    Vec::new()
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::AgentDone => {
        self.input_mode.clear_approval();
        self.transcript.finish_agent_activity();
      }
      Event::AgentDelta(delta) => self.transcript.push_agent_delta(&delta),
      Event::AgentReasoningDelta(delta) => {
        self.transcript.push_agent_reasoning_delta(&delta);
      }
      Event::AgentToolCall(tool_call) => {
        self.transcript.push_tool_call(tool_call);
      }
      Event::AgentToolResult { id, result } => {
        self.input_mode.clear_approval();
        self.transcript.push_tool_result(&id, result);
      }
      Event::Error(error) => {
        self.input_mode.clear_approval();
        self.transcript.error(error);
      }
      Event::Tick(elapsed) => self.transcript.tick(elapsed),
      Event::ToolApprovalRequest(request) => {
        self.input_mode = InputMode::Approval(request);
      }
    }

    Vec::new()
  }

  #[cfg(test)]
  pub(crate) fn input_text(&self) -> String {
    self.composer.input_text()
  }

  fn interrupt_agent(&mut self) -> Vec<Effect> {
    if !self.transcript.is_agent_active() {
      return Vec::new();
    }

    self.input_mode.clear_approval();
    self.transcript.interrupt();

    vec![Effect::InterruptAgent]
  }

  pub(crate) fn new(options: &Options) -> Result<Self> {
    Ok(Self {
      composer: Composer::new(options.prompt.as_deref().unwrap_or_default())
        .footer(Footer::try_from(&options.model)?),
      input_mode: InputMode::Compose,
      should_quit: false,
      transcript: Transcript::default(),
    })
  }

  pub(crate) fn pending_approval(&self) -> Option<&ApprovalRequest> {
    self.input_mode.approval()
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

  fn run_command(&mut self, command: Command) -> Vec<Effect> {
    match command {
      Command::Clear => self.transcript.clear(),
      Command::Quit => self.quit(),
    }

    self.reset_input();

    Vec::new()
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
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
        self.transcript.push_agent(format!(
          "Unrecognized command '{input}'. Type \"/\" for a list of supported commands."
        ));

        self.reset_input();
      }

      return Vec::new();
    }

    if self.transcript.is_agent_active() {
      return Vec::new();
    }

    if input.is_empty() {
      return Vec::new();
    }

    let input = input.to_string();

    self.transcript.send(input.clone());

    let messages = self.transcript.messages();

    self.reset_input();

    vec![Effect::RunAgent { messages }]
  }

  pub(crate) fn transcript(&self) -> &Transcript {
    &self.transcript
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_events_update_transcript() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    state.handle_event(Event::AgentReasoningDelta("bar".into()));
    state.handle_event(Event::AgentDelta("baz".into()));

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    };

    state.handle_event(Event::AgentToolCall(invocation.clone()));

    let result = ToolResult::command(Some(0), "qux", "");

    state.handle_event(Event::AgentToolResult {
      id: "foo".into(),
      result: result.clone(),
    });

    state.handle_event(Event::AgentDone);

    assert_eq!(
      state.transcript().messages(),
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::Agent(vec![AgentMessageContent::Text("baz".into())]),
        invocation.message(),
        result.message("foo"),
      ],
    );
  }

  #[tokio::test]
  async fn approval_approves_with_lowercase_y() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('y'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Approved);

    assert!(state.pending_approval().is_none());
  }

  #[tokio::test]
  async fn approval_approves_with_uppercase_y() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('Y'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Approved);

    assert!(state.pending_approval().is_none());
  }

  #[test]
  fn approval_complete_command_leaves_request_pending() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::CompleteCommand)),
      Vec::new()
    );

    assert!(state.pending_approval().is_some());
  }

  #[tokio::test]
  async fn approval_denies_with_escape() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert!(state.pending_approval().is_none());
  }

  #[tokio::test]
  async fn approval_denies_with_lowercase_n() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('n'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert!(state.pending_approval().is_none());
  }

  #[tokio::test]
  async fn approval_denies_with_quit() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(state.handle_event(Event::Action(Action::Quit)), Vec::new());
    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert!(state.pending_approval().is_none());
    assert!(state.should_quit());
  }

  #[tokio::test]
  async fn approval_denies_with_uppercase_n() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('N'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert!(state.pending_approval().is_none());
  }

  #[test]
  fn approval_edit_other_key_leaves_request_pending() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('x'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert!(state.pending_approval().is_some());
  }

  #[test]
  fn approval_select_next_command_leaves_request_pending() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::SelectNextCommand)),
      Vec::new()
    );

    assert!(state.pending_approval().is_some());
  }

  #[test]
  fn approval_select_previous_command_leaves_request_pending() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::SelectPreviousCommand)),
      Vec::new()
    );

    assert!(state.pending_approval().is_some());
  }

  #[test]
  fn approval_submit_leaves_request_pending() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.pending_approval().is_some());
  }

  #[tokio::test]
  async fn approval_terminal_agent_done_drops_pending_request() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));
    state.handle_event(Event::AgentDone);

    assert!(state.pending_approval().is_none());
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    state.handle_event(Event::AgentToolResult {
      id: "foo".into(),
      result: ToolResult::content("bar"),
    });

    assert!(state.pending_approval().is_none());
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn approval_terminal_error_drops_pending_request() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));
    state.handle_event(Event::Error("bar".into()));

    assert!(state.pending_approval().is_none());
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn blank_submit_does_nothing() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("  ".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.transcript().messages().is_empty());

    assert_eq!(state.input_text(), "  ");
  }

  #[test]
  fn command_autocomplete_select_next() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state
        .composer()
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectNextCommand));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.input_text(), "/quit");
  }

  #[test]
  fn command_autocomplete_select_previous() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state
        .composer()
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectPreviousCommand));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.input_text(), "/quit");
  }

  #[test]
  fn command_clear_from_empty_slash() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    state.handle_event(Event::AgentDelta("bar".into()));
    state.handle_event(Event::AgentDone);

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Char('/'),
      ..Default::default()
    })));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.transcript().messages().is_empty());

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn command_clear_from_name() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    state.handle_event(Event::AgentDelta("bar".into()));
    state.handle_event(Event::AgentDone);

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

    assert!(state.transcript().messages().is_empty());

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn command_clear_from_prefix() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    state.handle_event(Event::AgentDelta("bar".into()));
    state.handle_event(Event::AgentDone);

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

    assert!(state.transcript().messages().is_empty());

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn command_quit_from_name() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/quit".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit());

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn command_quit_from_prefix() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/q".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit());

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn error_clears_active_message() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    state.handle_event(Event::Error("bar".into()));

    assert_eq!(
      state.transcript().messages()[1],
      Message::Agent(vec![AgentMessageContent::Text("bar".into())])
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
          Message::Agent(vec![AgentMessageContent::Text("bar".into())]),
          Message::User(vec![UserMessageContent::Text("baz".into())]),
        ]
      }]
    );
  }

  #[test]
  fn interrupt_stops_active_agent() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.transcript().is_agent_active());

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
    );
  }

  #[test]
  fn multiline_input() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );
  }

  #[test]
  fn new_uses_empty_prompt_by_default() {
    let state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn quit_interrupts_active_agent() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit());
    assert!(!state.transcript().is_agent_active());

    assert_eq!(state.handle_event(Event::Action(Action::Quit)), Vec::new());

    assert!(state.should_quit());
  }

  #[tokio::test]
  async fn quit_interrupts_active_approval() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.handle_event(Event::ToolApprovalRequest(request));

    assert!(state.pending_approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit());
    assert!(!state.transcript().is_agent_active());

    assert!(state.pending_approval().is_none());
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn submit_is_ignored_while_agent_active() {
    let mut state = State::new(&Options {
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
        )])]
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

    assert_eq!(state.input_text(), "bar");

    assert_eq!(
      state.transcript().messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[test]
  fn submit_trims_input() {
    let mut state = State::new(&Options {
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
        )])]
      }]
    );

    assert_eq!(state.input_text(), "");
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new(&Options {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/foobar".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert_eq!(
      state.transcript().messages(),
      vec![Message::Agent(vec![AgentMessageContent::Text(
        "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands.".into()
      )])]
    );

    assert_eq!(state.input_text(), "");
  }
}
