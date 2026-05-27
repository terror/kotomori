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
  fn active_frame_ticks() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert!(
      state.transcript().render(80).ends_with(&[
        Line::blank(),
        vec![
          Span::styled("✦", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(" (0s • esc to interrupt)", Style::DarkGray),
        ]
        .into(),
        Line::blank(),
      ])
    );

    state.handle_event(Event::Tick(Duration::from_millis(120)));

    assert!(
      state.transcript().render(80).ends_with(&[
        Line::blank(),
        vec![
          Span::styled("✧", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
          Span::styled(" (0s • esc to interrupt)", Style::DarkGray),
        ]
        .into(),
        Line::blank(),
      ])
    );

    state.handle_event(Event::AgentDone);
    state.handle_event(Event::Tick(Duration::from_millis(120)));

    assert!(
      !state
        .transcript()
        .render(80)
        .iter()
        .any(|line| line.to_string().contains("Working"))
    );
  }

  #[tokio::test]
  async fn approval_actions_resolve_pending_request() {
    async fn case(action: Action, expected: ToolApproval) {
      let mut state = State::new(&Options {
        model: "fake:local".parse().unwrap(),
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

      state.handle_event(Event::Action(action));

      assert_eq!(response_receiver.await.unwrap(), expected);

      assert!(state.pending_approval().is_none());
    }

    case(
      Action::Edit(Input {
        key: Key::Char('y'),
        ..Default::default()
      }),
      ToolApproval::Approved,
    )
    .await;

    case(Action::Interrupt, ToolApproval::Denied).await;
  }

  #[test]
  fn command_autocomplete() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
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

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.should_quit());
  }

  #[test]
  fn command_clear() {
    #[track_caller]
    fn case(command: &str) {
      let mut state = State::new(&Options {
        model: "fake:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      })
      .unwrap();

      assert_eq!(
        state.handle_event(Event::Action(Action::Submit)),
        vec![Effect::RunAgent {
          messages: vec![Message::new(Role::User, "foo")]
        }]
      );

      state.handle_event(Event::AgentDelta("bar".into()));
      state.handle_event(Event::AgentDone);

      for c in command.chars() {
        state.handle_event(Event::Action(Action::Edit(Input {
          key: Key::Char(c),
          ..Default::default()
        })));
      }

      state.handle_event(Event::Action(Action::Submit));

      assert_eq!(state.transcript().messages(), &[]);
      assert_eq!(state.input_text(), "");
    }

    case("/");
    case("/c");
  }

  #[test]
  fn error_clears_active_message() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::new(Role::User, "foo")]
      }]
    );

    state.handle_event(Event::Error("bar".into()));

    assert_eq!(
      state.transcript().messages()[1],
      Message::new(Role::Agent, "bar")
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
          Message::new(Role::User, "foo"),
          Message::new(Role::Agent, "bar"),
          Message::new(Role::User, "baz"),
        ]
      }]
    );
  }

  #[test]
  fn interrupt_stops_active_agent() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::new(Role::User, "foo")]
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
  fn quit_interrupts_active_agent() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::new(Role::User, "foo")]
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
      model: "fake:local".parse().unwrap(),
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
  fn multiline_input() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
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
        messages: vec![Message::new(Role::User, "foo\nbar")]
      }]
    );
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("/foobar".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert_eq!(
      state.transcript().messages(),
      &[Message::new(
        Role::Agent,
        "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
      )]
    );

    assert_eq!(state.input_text(), "");
  }
}
