use super::*;

#[derive(Debug)]
pub(crate) struct State {
  composer: Composer,
  next_run_id: u64,
  queued_inputs: VecDeque<String>,
  reasoning_expanded: bool,
  run: Option<Run>,
  session: Session,
  should_quit: bool,
}

impl State {
  pub(crate) fn active_run(&self) -> Option<&Run> {
    self.run.as_ref()
  }

  pub(crate) fn approval(&self) -> Option<&ApprovalRequest> {
    self.run.as_ref().and_then(|run| run.approval.as_ref())
  }

  pub(crate) fn composer(&self) -> &Composer {
    &self.composer
  }

  pub(crate) fn directory(&self) -> &Path {
    &self.session.settings.directory
  }

  fn finish_run(&mut self, entry: Option<TranscriptEntry>) {
    if let Some(entry) = self.run.take().and_then(Run::finish) {
      self.session.transcript.entries.push(entry);
    }

    self.session.transcript.entries.extend(entry);
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.run.is_some() {
      return self.interrupt_agent();
    }

    match self.approval() {
      Some(_) => match action {
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
        Action::ToggleReasoning => {
          self.reasoning_expanded = !self.reasoning_expanded;
        }
        Action::CompleteCommand
        | Action::Edit(_)
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

  fn handle_command(&mut self, command: Command) -> Vec<Effect> {
    let effects = match command {
      Command::Clear => {
        let interrupt_agent = self.run.take().is_some();

        self.session.transcript.clear();
        self.composer.clear_history();
        self.queued_inputs.clear();

        if interrupt_agent {
          vec![Effect::InterruptAgent, Effect::SaveSession]
        } else {
          vec![Effect::SaveSession]
        }
      }
      Command::Quit => self.handle_action(Action::Quit),
    };

    self.reset_input();

    effects
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::Agent { event, run_id } => {
        let Some(run) = self.run.as_mut().filter(|run| run.id == run_id) else {
          return Vec::new();
        };

        match event {
          AgentEvent::Done => {
            self.finish_run(None);

            return once(Effect::SaveSession)
              .chain(self.run_next_queued())
              .collect();
          }
          AgentEvent::Update(update) => {
            run.update(update);
          }
          AgentEvent::Message(message) => {
            run.reset_message();
            self.session.transcript.push_message(message);
            return vec![Effect::SaveSession];
          }
          AgentEvent::Error(error) => {
            self.finish_run(Some(TranscriptEntry::Error(error)));

            return once(Effect::SaveSession)
              .chain(self.run_next_queued())
              .collect();
          }
          AgentEvent::ToolApprovalRequest(request) => {
            run.approval = Some(request);
          }
        }
      }
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
      Event::SessionSaved(result) => match result {
        Ok(saved) => {
          self.session.id = Some(saved.id);
          self.session.title = saved.title;
          self.session.updated_at = saved.updated_at;
        }
        Err(error) => self
          .session
          .transcript
          .error(format!("failed to save session: {error}")),
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

  pub(crate) fn model(&self) -> &Model {
    &self.session.settings.model
  }

  pub(crate) fn new(session: Session) -> Self {
    let history = session
      .transcript
      .entries
      .iter()
      .filter_map(TranscriptEntry::message)
      .filter_map(Message::user_content)
      .map(str::to_owned)
      .collect();

    Self {
      composer: Composer::new(
        session.settings.prompt.as_deref().unwrap_or_default(),
        history,
      ),
      next_run_id: 0,
      queued_inputs: VecDeque::new(),
      reasoning_expanded: false,
      run: None,
      session,
      should_quit: false,
    }
  }

  pub(crate) fn queued_inputs(&self) -> &VecDeque<String> {
    &self.queued_inputs
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  pub(crate) fn reasoning_expanded(&self) -> bool {
    self.reasoning_expanded
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn resolve_approval(&mut self, approval: ToolApproval) {
    if let Some(run) = &mut self.run {
      run.resolve_approval(approval);
    }
  }

  fn run(&mut self, input: String) -> Effect {
    self.session.transcript.send(input);

    let messages = self.session.transcript.messages();

    let run_id = self.next_run_id;

    self.next_run_id = self
      .next_run_id
      .checked_add(1)
      .expect("agent run ID overflow");

    self.run = Some(Run::new(run_id));

    Effect::RunAgent { messages, run_id }
  }

  fn run_next_queued(&mut self) -> Vec<Effect> {
    self
      .queued_inputs
      .pop_front()
      .map(|input| vec![self.run(input)])
      .unwrap_or_default()
  }

  pub(crate) fn session(&self) -> &Session {
    &self.session
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self, action: &Action) -> Vec<Effect> {
    let input = self.composer.input_text();
    let input = input.trim();

    if let Some(command) =
      Command::from_input(input).or_else(|| self.composer.selected_command())
    {
      return self.handle_command(command);
    }

    if input.starts_with('/') {
      if input.len() > 1 {
        self.session.transcript.notice(format!(
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

  pub(crate) fn transcript(&self) -> &Transcript {
    &self.session.transcript
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_events_update_transcript() {
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::ReasoningDelta {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "baz".into(),
        index: 1,
      }),
      run_id: 0,
    });

    let invocation = ToolInvocation::new(
      "foo",
      ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    );

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Message(Message::agent(vec![
          AssistantContent::reasoning("foo", "bar"),
          AssistantContent::text("baz"),
          AssistantContent::ToolCall(invocation.protocol.clone()),
        ])),
        run_id: 0,
      }),
      vec![Effect::SaveSession]
    );

    let result = ToolResult {
      exit_status: Some(0),
      outcome: ToolOutcome::Success,
      stdout: Some("qux".into()),
      ..Default::default()
    };

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result: result.clone(),
        },
      ])),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert_eq!(state.run, None);

    assert_eq!(
      state.session.transcript.messages(),
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::agent(vec![
          AssistantContent::reasoning("foo", "bar"),
          AssistantContent::text("baz"),
          AssistantContent::ToolCall(invocation.protocol),
        ]),
        Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result,
        }]),
      ],
    );
  }

  #[tokio::test]
  async fn approval_approves_with_lowercase_y() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('y'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Approved);

    assert_eq!(state.run, Some(Run::new(0)));
  }

  #[tokio::test]
  async fn approval_approves_with_uppercase_y() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('Y'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Approved);

    assert_eq!(state.run, Some(Run::new(0)));
  }

  #[test]
  fn approval_complete_command_leaves_request_pending() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, _response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    let invocation = request.invocation.clone();

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::CompleteCommand)),
      Vec::new()
    );

    assert_eq!(state.approval().unwrap().invocation, invocation);
  }

  #[tokio::test]
  async fn approval_denies_with_escape() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_eq!(state.run, Some(Run::new(0)));
  }

  #[tokio::test]
  async fn approval_denies_with_lowercase_n() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('n'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_eq!(state.run, Some(Run::new(0)));
  }

  #[tokio::test]
  async fn approval_denies_with_uppercase_n() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('N'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(response_receiver.await.unwrap(), ToolApproval::Denied);

    assert_eq!(state.run, Some(Run::new(0)));
  }

  #[test]
  fn approval_edit_other_key_leaves_request_pending() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, _response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    let invocation = request.invocation.clone();

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char('x'),
        ..Default::default()
      }))),
      Vec::new()
    );

    assert_eq!(state.approval().unwrap().invocation, invocation);
  }

  #[test]
  fn approval_select_next_command_leaves_request_pending() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, _response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    let invocation = request.invocation.clone();

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::SelectNext)),
      Vec::new()
    );

    assert_eq!(state.approval().unwrap().invocation, invocation);
  }

  #[test]
  fn approval_select_previous_command_leaves_request_pending() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, _response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    let invocation = request.invocation.clone();

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::SelectPrevious)),
      Vec::new()
    );

    assert_eq!(state.approval().unwrap().invocation, invocation);
  }

  #[test]
  fn approval_submit_leaves_request_pending() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, _response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    let invocation = request.invocation.clone();

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert_eq!(state.approval().unwrap().invocation, invocation);
  }

  #[tokio::test]
  async fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        },
      ])),
      run_id: 0,
    });

    assert_eq!(state.run, Some(Run::new(0)));
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn approval_terminal_error_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Error("bar".into()),
      run_id: 0,
    });

    assert_eq!(state.run, None);
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn blank_submit_does_nothing() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("  ".into()),
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert_eq!(state.session.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "  ");
  }

  #[test]
  fn command_autocomplete_select_next() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("/".into()),
        yolo: false,
      },
      0,
    ));

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
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("/".into()),
        yolo: false,
      },
      0,
    ));

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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Char('/'),
      ..Default::default()
    })));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::SaveSession]
    );

    assert_eq!(state.session.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_from_name() {
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::SaveSession]
    );

    assert_eq!(state.session.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_from_prefix() {
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "/c".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::SaveSession]
    );

    assert_eq!(state.session.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_interrupts_active_agent_and_ignores_late_events() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );

    assert_eq!(state.run, None);

    assert_eq!(state.session.transcript.messages(), Vec::new());

    let invocation = ToolInvocation::new(
      "late",
      ToolInvocationKind::Command(CommandTool {
        command: "echo late".into(),
        cwd: None,
      }),
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::agent(vec![
        AssistantContent::ToolCall(invocation.protocol),
      ])),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("late"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult::default(),
        },
      ])),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert_eq!(state.session.transcript.messages(), Vec::new());
  }

  #[test]
  fn command_quit_from_name() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("/quit".into()),
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit);

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_from_prefix() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("/q".into()),
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit);

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_interrupts_active_agent_and_saves_partial_output() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "partial response".into(),
        index: 0,
      }),
      run_id: 0,
    });

    for character in "/quit".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(character),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(state.composer.input_text(), "");

    assert_eq!(
      state.session.transcript.entries,
      [
        TranscriptEntry::Message(Message::User(vec![
          UserMessageContent::Text("foo".into())
        ])),
        TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
          "partial response"
        )])),
        TranscriptEntry::Interrupted,
      ],
    );
  }

  #[test]
  fn command_submission_returns_effects() {
    #[track_caller]
    fn case(input: &str, action: Action) {
      let mut state = State::new(Session::new(
        &Settings {
          directory: "foo".into(),
          model: "mock:local".parse().unwrap(),
          prompt: Some(input.into()),
          yolo: false,
        },
        0,
      ));

      state.run("foo".into());

      assert_eq!(
        state.handle_event(Event::Action(action)),
        vec![Effect::InterruptAgent, Effect::SaveSession]
      );

      assert_eq!(state.run, None);
      assert_eq!(state.composer.input_text(), "");
    }

    case("/cl", Action::Submit);
    case("/cl", Action::SubmitImmediately);
    case("/q", Action::Submit);
    case("/q", Action::SubmitImmediately);
    case("  /clear  ", Action::Submit);
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

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::ReasoningDelta {
        delta: "baz".into(),
        index: 1,
      }),
      run_id: 0,
    });

    let messages = vec![
      Message::agent(vec![
        AssistantContent::reasoning("foo", "foo"),
        AssistantContent::text("bar"),
        AssistantContent::ToolCall(
          ToolInvocation::new(
            "foo",
            ToolInvocationKind::Command(CommandTool {
              command: "bar".into(),
              cwd: None,
            }),
          )
          .protocol,
        ),
        AssistantContent::text("baz"),
        AssistantContent::ToolCall(
          ToolInvocation::new(
            "bar",
            ToolInvocationKind::Command(CommandTool {
              command: "qux".into(),
              cwd: None,
            }),
          )
          .protocol,
        ),
      ]),
      Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("bar"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("foo".into()),
            ..Default::default()
          },
        },
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        },
      ]),
      Message::agent(vec![AssistantContent::text("qux")]),
    ];

    for message in &messages {
      state.handle_event(Event::Agent {
        event: AgentEvent::Message(message.clone()),
        run_id: 0,
      });
    }

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    let session = state.session;

    let mut state = State::new(session);

    let messages =
      once(Message::User(vec![UserMessageContent::Text("foo".into())]))
        .chain(messages)
        .chain(once(Message::User(vec![UserMessageContent::Text(
          "foo".into(),
        )])))
        .collect();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      [
        Effect::SaveSession,
        Effect::RunAgent {
          messages,
          run_id: 0
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Error("bar".into()),
      run_id: 0,
    });

    assert_eq!(
      state.session.transcript.messages(),
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
          messages: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::User(vec![UserMessageContent::Text("baz".into())]),
          ],
          run_id: 1,
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
      let mut run = Run::new(0);
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
        let mut run = Run::new(0);
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
      let mut run = Run::new(0);
      run.update(MessageUpdate::Text {
        delta: "foo".into(),
        index: 0,
      });
      run
    });

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Done,
        run_id: 0
      }),
      [Effect::SaveSession]
    );

    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.messages(),
      [Message::agent(vec![AssistantContent::text("foo")])]
    );
  }

  #[test]
  fn immediate_submit_interrupts_active_agent_and_starts_new_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "partial".into(),
        index: 0,
      }),
      run_id: 0,
    });

    state.queued_inputs.push_back("baz".into());

    for c in "  bar  ".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::SubmitImmediately)),
      vec![
        Effect::InterruptAgent,
        Effect::SaveSession,
        Effect::RunAgent {
          messages: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::agent(vec![AssistantContent::text("partial")]),
            Message::User(vec![UserMessageContent::Text("bar".into())]),
          ],
          run_id: 1,
        },
      ]
    );

    assert_eq!(state.run, Some(Run::new(1)));

    assert_eq!(state.composer.input_text(), "");
    assert_eq!(state.queued_inputs(), &VecDeque::from(["baz".into()]));

    state.handle_event(Event::Action(Action::SelectPrevious));

    assert_eq!(state.composer.input_text(), "bar");

    state.handle_event(Event::Action(Action::SelectPrevious));

    assert_eq!(state.composer.input_text(), "foo");

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert_eq!(state.run, Some(Run::new(1)));
  }

  #[test]
  fn interrupt_advances_to_next_queued_submission() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("first".into()),
        yolo: false,
      },
      0,
    ));

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
      [
        Effect::InterruptAgent,
        Effect::SaveSession,
        Effect::RunAgent { run_id: 1, .. }
      ]
    );

    assert!(state.queued_inputs().is_empty());
    assert_eq!(state.run, Some(Run::new(1)));
  }

  #[test]
  fn interrupt_stops_active_agent() {
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );

    assert_eq!(state.run, None);

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
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

    state.run = Some(Run::new(0));

    let (event_sender, mut events) = mpsc::unbounded_channel();

    let sink = ProviderSink {
      event_sender,
      run_id: 0,
    };

    let reasoning = Reasoning::new_with_signature("foo", Some("bar".into()))
      .with_id("baz".into())
      .sealed("foo");

    let encrypted = Reasoning::encrypted("qux")
      .with_id("quux".into())
      .sealed("foo");

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
      state.handle_event(event);
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
        TranscriptEntry::Message(Message::agent(content)),
        TranscriptEntry::Interrupted,
      ]
    );
  }

  #[test]
  fn multiline_input() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

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
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo\nbar".into()
          )])],
          run_id: 0,
        }
      ]
    );
  }

  #[test]
  fn new_uses_empty_prompt_by_default() {
    let state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn prompt_history_edit_detaches_navigation() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("history".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

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
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("history".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

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
      directory: "foo".into(),
      model: "mock:local".parse().unwrap(),
      prompt: Some("draft".into()),
      yolo: false,
    };

    let mut session = Session::new(&settings, 0);

    session.transcript.entries = vec![
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "foo".into(),
      )])),
      TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
        "bar",
      )])),
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "baz\nqux".into(),
      )])),
    ];

    let mut state = State::new(session);

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "baz\nqux");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.cursor().0, 0);

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "foo");
  }

  #[test]
  fn prompt_history_navigates_and_restores_draft() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 1,
    });

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
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("history".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

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
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("first".into()),
        yolo: false,
      },
      0,
    ));

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
      [Effect::SaveSession, Effect::RunAgent { run_id: 1, .. }]
    );

    assert_matches!(
      state
        .handle_event(Event::Agent {
          event: AgentEvent::Done,
          run_id: 1,
        })
        .as_slice(),
      [Effect::SaveSession, Effect::RunAgent { run_id: 2, .. }]
    );

    assert_eq!(
      state.session.transcript.messages(),
      [
        Message::User(vec![UserMessageContent::Text("first".into())]),
        Message::User(vec![UserMessageContent::Text("second".into())]),
        Message::User(vec![UserMessageContent::Text("third".into())]),
      ]
    );
  }

  #[test]
  fn quit_interrupts_active_agent() {
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(state.handle_event(Event::Action(Action::Quit)), Vec::new());

    assert!(state.should_quit);
  }

  #[tokio::test]
  async fn quit_interrupts_active_approval() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.handle_event(Event::Agent {
      event: AgentEvent::ToolApprovalRequest(request),
      run_id: 0,
    });

    assert!(state.approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(state.approval(), None);
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn session_excludes_streaming_content() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.session.transcript.send("foo".into());

    state.run = Some({
      let mut run = Run::new(0);
      run.update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      });
      run
    });

    let session = state.session;

    assert_eq!(
      session.transcript.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[tokio::test]
  async fn stale_agent_events_do_not_mutate_new_run() {
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
    state.handle_event(Event::Action(Action::Interrupt));

    for c in "new".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_matches!(
      state.handle_event(Event::Action(Action::Submit)).as_slice(),
      [Effect::SaveSession, Effect::RunAgent { run_id: 1, .. }]
    );

    let invocation = ToolInvocation::new(
      "stale",
      ToolInvocationKind::Command(CommandTool {
        command: "echo".into(),
        cwd: None,
      }),
    );

    let (request, response_receiver) = ApprovalRequest::new(invocation.clone());

    for event in [
      AgentEvent::Update(MessageUpdate::Text {
        delta: "stale".into(),
        index: 0,
      }),
      AgentEvent::Update(MessageUpdate::ReasoningDelta {
        delta: "stale".into(),
        index: 0,
      }),
      AgentEvent::Message(Message::agent(vec![AssistantContent::ToolCall(
        invocation.protocol,
      )])),
      AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("stale"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("stale".into()),
            ..Default::default()
          },
        },
      ])),
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

    assert_eq!(state.approval(), None);

    assert_eq!(state.run, Some(Run::new(1)));

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "current".into(),
        index: 0,
      }),
      run_id: 1,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 1,
    });

    assert_eq!(
      state.session.transcript.messages(),
      [
        Message::User(vec![UserMessageContent::Text("old".into())]),
        Message::User(vec![UserMessageContent::Text("new".into())]),
        Message::agent(vec![AssistantContent::text("current")]),
      ]
    );
  }

  #[test]
  fn submit_is_queued_while_agent_active() {
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
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
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
      state.session.transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Done,
        run_id: 0,
      }),
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          messages: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::User(vec![UserMessageContent::Text("bar".into())]),
          ],
          run_id: 1,
        }
      ]
    );

    assert!(state.queued_inputs().is_empty());
  }

  #[test]
  fn submit_trims_input() {
    #[track_caller]
    fn case(action: Action) {
      let mut state = State::new(Session::new(
        &Settings {
          directory: "foo".into(),
          model: "mock:local".parse().unwrap(),
          prompt: Some("  foo  ".into()),
          yolo: false,
        },
        0,
      ));

      assert_eq!(
        state.handle_event(Event::Action(action)),
        vec![
          Effect::SaveSession,
          Effect::RunAgent {
            messages: vec![Message::User(vec![UserMessageContent::Text(
              "foo".into()
            )])],
            run_id: 0,
          }
        ]
      );

      assert_eq!(state.composer.input_text(), "");

      state.handle_event(Event::Action(Action::SelectPrevious));

      assert_eq!(state.composer.input_text(), "foo");
    }

    case(Action::Submit);
    case(Action::SubmitImmediately);
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

    state.session.transcript.send("bar".into());

    assert_eq!(
      state.handle_event(Event::SessionSaved(Ok(SavedSession {
        id: 2,
        title: Some("bar".into()),
        updated_at: 3,
      }))),
      Vec::new(),
    );
    assert_eq!(
      state.session(),
      &Session {
        created_at: 1,
        id: Some(2),
        settings,
        title: Some("bar".into()),
        transcript: Transcript::with_entries(vec![TranscriptEntry::Message(
          Message::User(vec![UserMessageContent::Text("bar".into())]),
        )]),
        updated_at: 3,
      }
    );
    assert_eq!(state.directory(), Path::new("foo"));
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

    state.run = Some(Run::new(0));

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

      let reasoning =
        Reasoning::new_with_signature("foo", Some("bar".into())).sealed("foo");

      let mut run = Run::new(0);

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
            messages: vec![
              Message::agent(vec![
                AssistantContent::Reasoning(reasoning),
                AssistantContent::text("baz"),
              ]),
              Message::User(vec![UserMessageContent::Text("qux".into())]),
            ],
            run_id: 0,
          }
        ]
      );
    }

    case(
      Event::Action(Action::Interrupt),
      TranscriptEntry::Interrupted,
    );

    case(
      Event::Agent {
        event: AgentEvent::Error("foo".into()),
        run_id: 0,
      },
      TranscriptEntry::Error("foo".into()),
    );
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("/foobar".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    assert_matches!(
      &state.session.transcript.entries[..],
      [TranscriptEntry::Notice(notice)]
        if notice
          == "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
    );

    assert_eq!(state.session.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "");
  }
}
